//! Adapter for converting sensor agent data to synheart-flux format.
//!
//! This module bridges the gap between the sensor agent's keyboard/mouse
//! events and the behavior module's event types.

use crate::collector::types::{KeyboardEvent, MouseEvent, MouseEventType};
use crate::core::windowing::EventWindow;
use synheart_flux::behavior::types::{ScrollEvent, TapEvent, TypingEvent};
use synheart_flux::behavior::{BehaviorEvent, BehaviorEventType, BehaviorSession};

/// Adapter for converting sensor events to behavioral session format.
pub struct SensorBehaviorAdapter {
    device_id: String,
    timezone: String,
}

impl SensorBehaviorAdapter {
    /// Create a new adapter with device and timezone info.
    pub fn new(device_id: String, timezone: String) -> Self {
        Self {
            device_id,
            timezone,
        }
    }

    /// Create with system defaults.
    pub fn with_defaults() -> Self {
        Self {
            device_id: format!("sensor-{}", uuid::Uuid::new_v4()),
            timezone: "UTC".to_string(),
        }
    }

    /// Convert an event window to a behavior session.
    pub fn convert(&self, session_id: &str, window: &EventWindow) -> BehaviorSession {
        let mut events = Vec::new();

        // Convert keyboard events to typing events
        for kb_event in &window.keyboard_events {
            if kb_event.is_key_down {
                events.push(self.keyboard_to_behavior(kb_event));
            }
        }

        // Convert mouse events to behavioral events
        for mouse_event in &window.mouse_events {
            if let Some(behavior_event) = self.mouse_to_behavior(mouse_event) {
                events.push(behavior_event);
            }
        }

        // Sort by timestamp
        events.sort_by_key(|e| e.timestamp);

        BehaviorSession {
            session_id: session_id.to_string(),
            device_id: self.device_id.clone(),
            timezone: self.timezone.clone(),
            start_time: window.start,
            end_time: window.end,
            events,
        }
    }

    /// Convert a keyboard event to a typing behavior event.
    fn keyboard_to_behavior(&self, kb: &KeyboardEvent) -> BehaviorEvent {
        BehaviorEvent {
            timestamp: kb.timestamp,
            event_type: BehaviorEventType::Typing,
            scroll: None,
            tap: None,
            swipe: None,
            interruption: None,
            typing: Some(TypingEvent {
                typing_speed_cpm: None, // Will be computed at session level
                cadence_stability: None,
                duration_sec: None,
                pause_count: None,
            }),
            app_switch: None,
        }
    }

    /// Convert a mouse event to a behavioral event.
    fn mouse_to_behavior(&self, mouse: &MouseEvent) -> Option<BehaviorEvent> {
        match mouse.event_type {
            MouseEventType::Move => {
                // Convert mouse movement to a scroll-like event for behavioral analysis
                // This captures interaction intensity
                Some(BehaviorEvent {
                    timestamp: mouse.timestamp,
                    event_type: BehaviorEventType::Scroll,
                    scroll: Some(ScrollEvent {
                        velocity: mouse.delta_magnitude,
                        direction: None, // Cursor movement doesn't have direction
                        direction_reversal: false,
                    }),
                    tap: None,
                    swipe: None,
                    interruption: None,
                    typing: None,
                    app_switch: None,
                })
            }
            MouseEventType::LeftClick | MouseEventType::RightClick => {
                Some(BehaviorEvent {
                    timestamp: mouse.timestamp,
                    event_type: BehaviorEventType::Tap,
                    scroll: None,
                    tap: Some(TapEvent {
                        tap_duration_ms: Some(100), // Estimated click duration
                        long_press: false,
                    }),
                    swipe: None,
                    interruption: None,
                    typing: None,
                    app_switch: None,
                })
            }
            MouseEventType::Scroll => {
                Some(BehaviorEvent {
                    timestamp: mouse.timestamp,
                    event_type: BehaviorEventType::Scroll,
                    scroll: Some(ScrollEvent {
                        velocity: mouse.delta_magnitude,
                        direction: None, // Could be inferred from scroll_direction
                        direction_reversal: false,
                    }),
                    tap: None,
                    swipe: None,
                    interruption: None,
                    typing: None,
                    app_switch: None,
                })
            }
        }
    }
}

/// Convenience function to convert an event window to a behavior session.
pub fn convert_to_behavior_session(
    session_id: &str,
    window: &EventWindow,
    device_id: Option<&str>,
) -> BehaviorSession {
    let adapter = match device_id {
        Some(id) => SensorBehaviorAdapter::new(id.to_string(), "UTC".to_string()),
        None => SensorBehaviorAdapter::with_defaults(),
    };
    adapter.convert(session_id, window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_adapter_creation() {
        let adapter = SensorBehaviorAdapter::with_defaults();
        assert!(adapter.device_id.starts_with("sensor-"));
    }

    #[test]
    fn test_empty_window_conversion() {
        let adapter = SensorBehaviorAdapter::with_defaults();
        let window = EventWindow::new(Utc::now(), Duration::seconds(10));
        let session = adapter.convert("test-session", &window);

        assert_eq!(session.session_id, "test-session");
        assert!(session.events.is_empty());
    }
}
