//! Windows implementation of event collection using Windows Hooks.
//!
//! This module captures keyboard and mouse events at the system level using
//! the Windows Hook API (SetWindowsHookEx). It captures low-level input events
//! in a privacy-preserving manner.

use crate::collector::types::{KeyboardEvent, MouseEvent, SensorEvent};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

/// Configuration for which event sources to capture.
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    pub capture_keyboard: bool,
    pub capture_mouse: bool,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            capture_keyboard: true,
            capture_mouse: true,
        }
    }
}

/// The Windows event collector using Windows Hooks.
pub struct WindowsCollector {
    config: CollectorConfig,
    sender: Sender<SensorEvent>,
    receiver: Receiver<SensorEvent>,
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl WindowsCollector {
    /// Create a new Windows collector with the given configuration.
    pub fn new(config: CollectorConfig) -> Self {
        // Use a bounded channel to prevent unbounded memory growth
        let (sender, receiver) = bounded(10_000);

        Self {
            config,
            sender,
            receiver,
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    /// Start capturing events in a background thread.
    ///
    /// Returns an error if the collector is already running.
    pub fn start(&mut self) -> Result<(), CollectorError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(CollectorError::AlreadyRunning);
        }

        self.running.store(true, Ordering::SeqCst);

        let sender = self.sender.clone();
        let running = self.running.clone();
        let config = self.config.clone();

        let handle = thread::spawn(move || {
            if let Err(e) = run_hook_loop(sender, running.clone(), config) {
                eprintln!("Hook loop error: {e:?}");
            }
            running.store(false, Ordering::SeqCst);
        });

        self.thread_handle = Some(handle);
        Ok(())
    }

    /// Stop capturing events.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            // The thread should exit when running becomes false
            let _ = handle.join();
        }
    }

    /// Check if the collector is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the receiver for sensor events.
    pub fn receiver(&self) -> &Receiver<SensorEvent> {
        &self.receiver
    }

    /// Try to receive an event without blocking.
    pub fn try_recv(&self) -> Option<SensorEvent> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for WindowsCollector {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Errors that can occur during event collection.
#[derive(Debug)]
pub enum CollectorError {
    AlreadyRunning,
    HookInstallationFailed,
}

impl std::fmt::Display for CollectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollectorError::AlreadyRunning => write!(f, "Collector is already running"),
            CollectorError::HookInstallationFailed => {
                write!(f, "Failed to install Windows hook")
            }
        }
    }
}

impl std::error::Error for CollectorError {}

/// Global state for the hook callbacks.
/// We use thread-local storage to avoid true globals.
thread_local! {
    static EVENT_SENDER: std::cell::RefCell<Option<Sender<SensorEvent>>> = const { std::cell::RefCell::new(None) };
    static LAST_MOUSE_X: std::cell::RefCell<i32> = const { std::cell::RefCell::new(0) };
    static LAST_MOUSE_Y: std::cell::RefCell<i32> = const { std::cell::RefCell::new(0) };
}

/// Low-level keyboard hook callback.
unsafe extern "system" fn keyboard_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let kb_struct = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
        let w_param_u32 = w_param.0 as u32;

        // Process keyboard events - only capture timing, NOT key codes
        let is_key_down = matches!(w_param_u32, WM_KEYDOWN | WM_SYSKEYDOWN);

        // Only send events for key down and key up (not for key repeats)
        if matches!(
            w_param_u32,
            WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP
        ) {
            let event = SensorEvent::Keyboard(KeyboardEvent::new(is_key_down));

            EVENT_SENDER.with(|sender| {
                if let Some(ref s) = *sender.borrow() {
                    let _ = s.try_send(event);
                }
            });
        }
    }

    // Pass the event to the next hook
    CallNextHookEx(HHOOK::default(), n_code, w_param, l_param)
}

/// Low-level mouse hook callback.
unsafe extern "system" fn mouse_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let mouse_struct = &*(l_param.0 as *const MSLLHOOKSTRUCT);
        let w_param_u32 = w_param.0 as u32;

        let event = match w_param_u32 {
            // Mouse movement - capture delta magnitude only, NO absolute position
            WM_MOUSEMOVE => {
                let current_x = mouse_struct.pt.x;
                let current_y = mouse_struct.pt.y;

                // Calculate delta from last position
                let (delta_x, delta_y) = LAST_MOUSE_X.with(|last_x| {
                    LAST_MOUSE_Y.with(|last_y| {
                        let lx = *last_x.borrow();
                        let ly = *last_y.borrow();

                        // Update stored position
                        *last_x.borrow_mut() = current_x;
                        *last_y.borrow_mut() = current_y;

                        // If this is the first event, delta is 0
                        if lx == 0 && ly == 0 {
                            (0.0, 0.0)
                        } else {
                            ((current_x - lx) as f64, (current_y - ly) as f64)
                        }
                    })
                });

                // Only send if there's actual movement
                if delta_x.abs() > 0.1 || delta_y.abs() > 0.1 {
                    Some(SensorEvent::Mouse(MouseEvent::movement(delta_x, delta_y)))
                } else {
                    None
                }
            }

            // Click events
            WM_LBUTTONDOWN => Some(SensorEvent::Mouse(MouseEvent::click(true))),
            WM_RBUTTONDOWN => Some(SensorEvent::Mouse(MouseEvent::click(false))),

            // Scroll events
            WM_MOUSEWHEEL => {
                // High word of mouseData contains the wheel delta
                let wheel_delta = ((mouse_struct.mouseData >> 16) & 0xFFFF) as i16 as f64;
                // Convert to scroll units (typically 120 per notch)
                let delta_y = wheel_delta / 120.0;
                Some(SensorEvent::Mouse(MouseEvent::scroll(0.0, delta_y)))
            }

            WM_MOUSEHWHEEL => {
                // Horizontal scroll
                let wheel_delta = ((mouse_struct.mouseData >> 16) & 0xFFFF) as i16 as f64;
                let delta_x = wheel_delta / 120.0;
                Some(SensorEvent::Mouse(MouseEvent::scroll(delta_x, 0.0)))
            }

            // Ignore button up events and middle button
            WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONDOWN | WM_MBUTTONUP => None,

            _ => None,
        };

        if let Some(event) = event {
            EVENT_SENDER.with(|sender| {
                if let Some(ref s) = *sender.borrow() {
                    let _ = s.try_send(event);
                }
            });
        }
    }

    // Pass the event to the next hook
    CallNextHookEx(HHOOK::default(), n_code, w_param, l_param)
}

/// Run the Windows hook message loop.
fn run_hook_loop(
    sender: Sender<SensorEvent>,
    running: Arc<AtomicBool>,
    config: CollectorConfig,
) -> Result<(), CollectorError> {
    // Store sender in thread-local
    EVENT_SENDER.with(|s| {
        *s.borrow_mut() = Some(sender);
    });

    // Initialize last mouse position
    LAST_MOUSE_X.with(|x| *x.borrow_mut() = 0);
    LAST_MOUSE_Y.with(|y| *y.borrow_mut() = 0);

    unsafe {
        // Install hooks based on configuration
        let mut hooks: Vec<HHOOK> = Vec::new();

        if config.capture_keyboard {
            let kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0);
            if kb_hook.is_err() {
                // Clean up any hooks we've installed
                for hook in hooks {
                    let _ = UnhookWindowsHookEx(hook);
                }
                return Err(CollectorError::HookInstallationFailed);
            }
            hooks.push(kb_hook.unwrap());
        }

        if config.capture_mouse {
            let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0);
            if mouse_hook.is_err() {
                // Clean up any hooks we've installed
                for hook in hooks {
                    let _ = UnhookWindowsHookEx(hook);
                }
                return Err(CollectorError::HookInstallationFailed);
            }
            hooks.push(mouse_hook.unwrap());
        }

        // Message loop
        let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
        while running.load(Ordering::SeqCst) {
            // Process messages with a timeout so we can check the running flag
            let result = GetMessageW(&mut msg, HWND::default(), 0, 0);

            if result.0 > 0 {
                // Message retrieved, but we don't need to dispatch it
                // The hooks run automatically
            } else if result.0 == 0 {
                // WM_QUIT received
                break;
            } else {
                // Error occurred
                break;
            }

            // Check running status periodically (we already do this in the loop condition)
        }

        // Unhook before exiting
        for hook in hooks {
            let _ = UnhookWindowsHookEx(hook);
        }
    }

    Ok(())
}

/// Check if the application has permission to capture events.
///
/// On Windows, low-level hooks generally work without explicit permission,
/// but may require the application to run with appropriate privileges.
/// This function attempts to install a temporary hook to verify.
pub fn check_permission() -> bool {
    unsafe {
        // Try to install a temporary keyboard hook
        let hook_result = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_hook_proc),
            None,
            0,
        );

        if let Ok(hook) = hook_result {
            // Successfully installed, clean up and return true
            let _ = UnhookWindowsHookEx(hook);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_config_default() {
        let config = CollectorConfig::default();
        assert!(config.capture_keyboard);
        assert!(config.capture_mouse);
    }

    #[test]
    fn test_collector_creation() {
        let collector = WindowsCollector::new(CollectorConfig::default());
        assert!(!collector.is_running());
    }

    #[test]
    fn test_collector_lifecycle() {
        let mut collector = WindowsCollector::new(CollectorConfig::default());
        assert!(!collector.is_running());

        // Note: Actually starting the collector requires a message loop
        // and may fail in test environment, so we just test creation
    }
}

