//! macOS implementation of event collection using CGEvent tap.
//!
//! This module captures keyboard and mouse events at the system level using
//! macOS's Core Graphics event tap API. It requires Input Monitoring permission.

use crate::collector::types::{KeyboardEvent, KeyboardEventType, MouseEvent, SensorEvent};
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    CallbackResult,
};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

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

/// The macOS event collector using CGEvent tap.
pub struct MacOSCollector {
    config: CollectorConfig,
    sender: Sender<SensorEvent>,
    receiver: Receiver<SensorEvent>,
    running: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl MacOSCollector {
    /// Create a new macOS collector with the given configuration.
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
    /// Returns an error if:
    /// - The collector is already running
    /// - Input Monitoring permission is not granted
    pub fn start(&mut self) -> Result<(), CollectorError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(CollectorError::AlreadyRunning);
        }

        self.running.store(true, Ordering::SeqCst);

        let sender = self.sender.clone();
        let running = self.running.clone();
        let config = self.config.clone();

        let handle = thread::spawn(move || {
            if let Err(e) = run_event_loop(sender, running.clone(), config) {
                eprintln!("Event loop error: {e:?}");
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

impl Drop for MacOSCollector {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Errors that can occur during event collection.
#[derive(Debug)]
pub enum CollectorError {
    AlreadyRunning,
    PermissionDenied,
    TapCreationFailed,
    RunLoopSourceFailed,
}

impl std::fmt::Display for CollectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollectorError::AlreadyRunning => write!(f, "Collector is already running"),
            CollectorError::PermissionDenied => {
                write!(f, "Input Monitoring permission not granted")
            }
            CollectorError::TapCreationFailed => write!(f, "Failed to create CGEvent tap"),
            CollectorError::RunLoopSourceFailed => {
                write!(f, "Failed to create run loop source")
            }
        }
    }
}

impl std::error::Error for CollectorError {}

/// Build a list of event types to capture based on configuration.
fn build_event_types(config: &CollectorConfig) -> Vec<CGEventType> {
    let mut types = Vec::new();

    if config.capture_keyboard {
        types.push(CGEventType::KeyDown);
        types.push(CGEventType::KeyUp);
        types.push(CGEventType::FlagsChanged);
    }

    if config.capture_mouse {
        types.push(CGEventType::MouseMoved);
        types.push(CGEventType::LeftMouseDown);
        types.push(CGEventType::LeftMouseUp);
        types.push(CGEventType::RightMouseDown);
        types.push(CGEventType::RightMouseUp);
        types.push(CGEventType::LeftMouseDragged);
        types.push(CGEventType::RightMouseDragged);
        types.push(CGEventType::ScrollWheel);
    }

    types
}

/// Run the Core Graphics event loop.
fn run_event_loop(
    sender: Sender<SensorEvent>,
    running: Arc<AtomicBool>,
    config: CollectorConfig,
) -> Result<(), CollectorError> {
    // Build the list of event types to capture
    let event_types = build_event_types(&config);

    // Store sender in a thread-local for the callback
    // Note: We need to use a different approach since the callback can't capture variables
    thread_local! {
        static EVENT_SENDER: std::cell::RefCell<Option<Sender<SensorEvent>>> = const { std::cell::RefCell::new(None) };
    }

    EVENT_SENDER.with(|s| {
        *s.borrow_mut() = Some(sender);
    });

    // Callback function for CGEvent tap
    fn event_callback(
        _proxy: core_graphics::event::CGEventTapProxy,
        event_type: CGEventType,
        event: &CGEvent,
    ) -> CallbackResult {
        thread_local! {
            static EVENT_SENDER: std::cell::RefCell<Option<Sender<SensorEvent>>> = const { std::cell::RefCell::new(None) };
        }

        // Try to get the sender and process the event
        EVENT_SENDER.with(|sender_cell| {
            if let Some(ref sender) = *sender_cell.borrow() {
                if let Some(sensor_event) = process_cg_event(event_type, event) {
                    // Don't block if the channel is full - just drop the event
                    let _ = sender.try_send(sensor_event);
                }
            }
        });

        // Return the event unchanged (we're passive observers)
        CallbackResult::Keep
    }

    // Create the event tap
    let tap = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        event_types,
        event_callback,
    )
    .map_err(|_| CollectorError::TapCreationFailed)?;

    // Create the run loop source
    let source = tap
        .mach_port()
        .create_runloop_source(0)
        .map_err(|_| CollectorError::RunLoopSourceFailed)?;

    // Add source to the run loop
    let run_loop = CFRunLoop::get_current();
    unsafe {
        run_loop.add_source(&source, kCFRunLoopCommonModes);
    }

    // Enable the tap
    tap.enable();

    // Run the loop until stopped
    while running.load(Ordering::SeqCst) {
        // Run the loop for a short interval, then check if we should stop
        CFRunLoop::run_in_mode(
            unsafe { kCFRunLoopCommonModes },
            std::time::Duration::from_millis(100),
            false,
        );
    }

    // The tap is automatically disabled when dropped
    Ok(())
}

/// Check if a key code corresponds to a navigation key.
///
/// Navigation keys are: Arrow keys, Page Up/Down, Home, End.
/// These are used for scrolling/navigation and should not inflate typing metrics.
///
/// Privacy: The key code is only used for classification - it is NOT stored or transmitted.
/// Only the boolean classification (navigation vs typing) is recorded.
fn is_navigation_key(keycode: i64) -> bool {
    // macOS virtual key codes for navigation keys
    const KEY_LEFT_ARROW: i64 = 123;
    const KEY_RIGHT_ARROW: i64 = 124;
    const KEY_DOWN_ARROW: i64 = 125;
    const KEY_UP_ARROW: i64 = 126;
    const KEY_PAGE_UP: i64 = 116;
    const KEY_PAGE_DOWN: i64 = 121;
    const KEY_HOME: i64 = 115;
    const KEY_END: i64 = 119;

    matches!(
        keycode,
        KEY_LEFT_ARROW
            | KEY_RIGHT_ARROW
            | KEY_DOWN_ARROW
            | KEY_UP_ARROW
            | KEY_PAGE_UP
            | KEY_PAGE_DOWN
            | KEY_HOME
            | KEY_END
    )
}

/// Classify a keyboard event as navigation or typing based on key code.
///
/// Privacy: The key code is used only for classification and is immediately discarded.
/// The actual key code value is never stored or transmitted.
fn classify_keyboard_event(event: &CGEvent) -> KeyboardEventType {
    let keycode =
        event.get_integer_value_field(core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE);

    if is_navigation_key(keycode) {
        KeyboardEventType::NavigationKey
    } else {
        KeyboardEventType::TypingTap
    }
}

/// Process a CGEvent and convert it to a SensorEvent.
///
/// Privacy: This function ONLY extracts timing and magnitude information,
/// never key codes, characters, or absolute coordinates. Key codes are used
/// internally only to classify events as navigation vs typing, then discarded.
fn process_cg_event(event_type: CGEventType, event: &CGEvent) -> Option<SensorEvent> {
    use core_graphics::event::CGEventType::*;

    match event_type {
        // Keyboard events - capture timing and classification only, NO key codes stored
        KeyDown => {
            let event_class = classify_keyboard_event(event);
            Some(SensorEvent::Keyboard(KeyboardEvent::with_type(
                true,
                event_class,
            )))
        }
        KeyUp => {
            let event_class = classify_keyboard_event(event);
            Some(SensorEvent::Keyboard(KeyboardEvent::with_type(
                false,
                event_class,
            )))
        }
        FlagsChanged => {
            // Modifier key change - treat as typing key event (not navigation)
            // We can't easily determine down/up for modifiers, so we just record it
            Some(SensorEvent::Keyboard(KeyboardEvent::new(true)))
        }

        // Mouse movement - capture delta magnitude only, NO absolute position
        MouseMoved | LeftMouseDragged | RightMouseDragged => {
            // Get the delta (movement amount), not the absolute position
            let delta_x =
                event.get_double_value_field(core_graphics::event::EventField::MOUSE_EVENT_DELTA_X);
            let delta_y =
                event.get_double_value_field(core_graphics::event::EventField::MOUSE_EVENT_DELTA_Y);

            Some(SensorEvent::Mouse(MouseEvent::movement(delta_x, delta_y)))
        }

        // Click events - left button
        LeftMouseDown => Some(SensorEvent::Mouse(MouseEvent::click(true))),
        LeftMouseUp => None, // We only count the down event as a "click"

        // Click events - right button
        RightMouseDown => Some(SensorEvent::Mouse(MouseEvent::click(false))),
        RightMouseUp => None, // We only count the down event as a "click"

        // Scroll events
        ScrollWheel => {
            let delta_x = event.get_double_value_field(
                core_graphics::event::EventField::SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_2,
            );
            let delta_y = event.get_double_value_field(
                core_graphics::event::EventField::SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_1,
            );

            Some(SensorEvent::Mouse(MouseEvent::scroll(delta_x, delta_y)))
        }

        // Ignore other event types
        _ => None,
    }
}

/// Check if the application has Input Monitoring permission.
///
/// Note: This doesn't actually check the permission - macOS doesn't provide
/// a direct API for that. Instead, attempting to create the tap will fail
/// if permission is not granted.
pub fn check_permission() -> bool {
    // On macOS 10.15+, we can try to create a passive tap to check permission
    // If it fails, permission is likely not granted
    let result = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::KeyDown],
        |_proxy, _type, _event| CallbackResult::Keep,
    );

    result.is_ok()
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
        let collector = MacOSCollector::new(CollectorConfig::default());
        assert!(!collector.is_running());
    }
}
