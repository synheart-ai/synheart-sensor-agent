//! Window management for collecting events into time-based windows.
//!
//! Events are collected into fixed-duration windows (default 10 seconds)
//! for feature extraction. Session boundaries are detected based on gaps.

use crate::collector::types::{KeyboardEvent, MouseEvent, SensorEvent};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// A time window containing collected events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWindow {
    /// Start time of the window
    pub start: DateTime<Utc>,
    /// End time of the window
    pub end: DateTime<Utc>,
    /// Keyboard events in this window
    pub keyboard_events: Vec<KeyboardEvent>,
    /// Mouse events in this window
    pub mouse_events: Vec<MouseEvent>,
    /// Whether this window marks the start of a new session
    pub is_session_start: bool,
}

impl EventWindow {
    /// Create a new empty window starting at the given time.
    pub fn new(start: DateTime<Utc>, duration: Duration) -> Self {
        Self {
            start,
            end: start + duration,
            keyboard_events: Vec::new(),
            mouse_events: Vec::new(),
            is_session_start: false,
        }
    }

    /// Check if a timestamp falls within this window.
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start && timestamp < self.end
    }

    /// Add an event to this window.
    pub fn add_event(&mut self, event: SensorEvent) {
        match event {
            SensorEvent::Keyboard(e) => self.keyboard_events.push(e),
            SensorEvent::Mouse(e) => self.mouse_events.push(e),
        }
    }

    /// Check if the window has any events.
    pub fn is_empty(&self) -> bool {
        self.keyboard_events.is_empty() && self.mouse_events.is_empty()
    }

    /// Get the total number of events in this window.
    pub fn event_count(&self) -> usize {
        self.keyboard_events.len() + self.mouse_events.len()
    }

    /// Get the duration of this window in seconds.
    pub fn duration_secs(&self) -> f64 {
        (self.end - self.start).num_milliseconds() as f64 / 1000.0
    }
}

/// Manages the collection of events into time windows.
pub struct WindowManager {
    /// Duration of each window
    window_duration: Duration,
    /// Gap threshold for session boundaries
    session_gap_threshold: Duration,
    /// Current window being filled
    current_window: Option<EventWindow>,
    /// Completed windows ready for processing
    completed_windows: Vec<EventWindow>,
    /// Timestamp of the last event received
    last_event_time: Option<DateTime<Utc>>,
}

impl WindowManager {
    /// Create a new window manager with the given window duration.
    pub fn new(window_duration_secs: u64, session_gap_threshold_secs: u64) -> Self {
        Self {
            window_duration: Duration::seconds(window_duration_secs as i64),
            session_gap_threshold: Duration::seconds(session_gap_threshold_secs as i64),
            current_window: None,
            completed_windows: Vec::new(),
            last_event_time: None,
        }
    }

    /// Process an incoming event.
    ///
    /// This will:
    /// 1. Detect session boundaries based on gaps
    /// 2. Create new windows as needed
    /// 3. Complete windows when their time expires
    pub fn process_event(&mut self, event: SensorEvent) {
        let event_time = event.timestamp();

        // Check for session boundary (gap in events)
        let is_new_session = if let Some(last_time) = self.last_event_time {
            event_time - last_time > self.session_gap_threshold
        } else {
            true // First event starts a session
        };

        // If this is a new session, complete the current window
        if is_new_session && self.current_window.is_some() {
            self.complete_current_window();
        }

        // Ensure we have a current window
        if self.current_window.is_none() {
            let mut window = EventWindow::new(event_time, self.window_duration);
            window.is_session_start = is_new_session;
            self.current_window = Some(window);
        }

        // Check if the event falls outside the current window
        let window = self.current_window.as_ref().unwrap();
        if event_time >= window.end {
            // Complete the current window and create a new one
            self.complete_current_window();

            // Align the new window to the event time
            let mut window = EventWindow::new(event_time, self.window_duration);
            window.is_session_start = is_new_session;
            self.current_window = Some(window);
        }

        // Add the event to the current window
        if let Some(ref mut window) = self.current_window {
            window.add_event(event);
        }

        self.last_event_time = Some(event_time);
    }

    /// Force completion of the current window (e.g., on pause or stop).
    pub fn flush(&mut self) {
        self.complete_current_window();
    }

    /// Get and remove completed windows.
    pub fn take_completed_windows(&mut self) -> Vec<EventWindow> {
        std::mem::take(&mut self.completed_windows)
    }

    /// Check if there are completed windows available.
    pub fn has_completed_windows(&self) -> bool {
        !self.completed_windows.is_empty()
    }

    /// Get the number of completed windows.
    pub fn completed_window_count(&self) -> usize {
        self.completed_windows.len()
    }

    /// Complete the current window and move it to completed.
    fn complete_current_window(&mut self) {
        if let Some(window) = self.current_window.take() {
            // Only keep non-empty windows
            if !window.is_empty() {
                self.completed_windows.push(window);
            }
        }
    }

    /// Check and complete the current window if it has expired.
    pub fn check_window_expiry(&mut self) {
        let now = Utc::now();
        if let Some(ref window) = self.current_window {
            if now >= window.end {
                self.complete_current_window();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_creation() {
        let start = Utc::now();
        let window = EventWindow::new(start, Duration::seconds(10));

        assert_eq!(window.start, start);
        assert_eq!(window.end, start + Duration::seconds(10));
        assert!(window.is_empty());
    }

    #[test]
    fn test_window_contains() {
        let start = Utc::now();
        let window = EventWindow::new(start, Duration::seconds(10));

        assert!(window.contains(start));
        assert!(window.contains(start + Duration::seconds(5)));
        assert!(!window.contains(start + Duration::seconds(10)));
        assert!(!window.contains(start - Duration::seconds(1)));
    }

    #[test]
    fn test_window_manager_basic() {
        let mut manager = WindowManager::new(10, 300);

        // Process some keyboard events
        for _ in 0..5 {
            let event = SensorEvent::Keyboard(crate::collector::types::KeyboardEvent::new(true));
            manager.process_event(event);
        }

        // Window shouldn't be complete yet
        assert!(!manager.has_completed_windows());

        // Flush to complete the current window
        manager.flush();
        assert!(manager.has_completed_windows());

        let windows = manager.take_completed_windows();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].keyboard_events.len(), 5);
    }
}
