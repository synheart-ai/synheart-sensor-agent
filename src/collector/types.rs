//! Privacy-preserving event types for the Synheart Sensor Agent.
//!
//! These types capture ONLY timing and magnitude information - never content or coordinates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A keyboard event capturing only timing information.
///
/// Privacy guarantee: No key codes, characters, or any content is captured.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardEvent {
    /// Timestamp when the event occurred
    pub timestamp: DateTime<Utc>,
    /// Whether this is a key press (true) or release (false)
    pub is_key_down: bool,
}

impl KeyboardEvent {
    pub fn new(is_key_down: bool) -> Self {
        Self {
            timestamp: Utc::now(),
            is_key_down,
        }
    }
}

/// Mouse event type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseEventType {
    /// Mouse movement
    Move,
    /// Left button click
    LeftClick,
    /// Right button click
    RightClick,
    /// Scroll event
    Scroll,
}

/// Scroll direction (privacy-preserving - no exact amounts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// A mouse event capturing only timing and magnitude information.
///
/// Privacy guarantee: No absolute coordinates are captured. Only movement
/// magnitudes (deltas) are recorded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseEvent {
    /// Timestamp when the event occurred
    pub timestamp: DateTime<Utc>,
    /// Type of mouse event
    pub event_type: MouseEventType,
    /// Movement magnitude (distance moved, not direction or absolute position)
    /// Only set for Move events
    pub delta_magnitude: Option<f64>,
    /// Scroll direction (only set for Scroll events)
    pub scroll_direction: Option<ScrollDirection>,
    /// Scroll magnitude bucket (small/medium/large)
    pub scroll_magnitude: Option<ScrollMagnitude>,
}

/// Bucketed scroll magnitude to avoid precise tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrollMagnitude {
    Small,  // < 3 lines
    Medium, // 3-10 lines
    Large,  // > 10 lines
}

impl MouseEvent {
    /// Create a new mouse move event with delta magnitude.
    pub fn movement(delta_x: f64, delta_y: f64) -> Self {
        let magnitude = (delta_x * delta_x + delta_y * delta_y).sqrt();
        Self {
            timestamp: Utc::now(),
            event_type: MouseEventType::Move,
            delta_magnitude: Some(magnitude),
            scroll_direction: None,
            scroll_magnitude: None,
        }
    }

    /// Create a new click event.
    pub fn click(is_left: bool) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type: if is_left {
                MouseEventType::LeftClick
            } else {
                MouseEventType::RightClick
            },
            delta_magnitude: None,
            scroll_direction: None,
            scroll_magnitude: None,
        }
    }

    /// Create a new scroll event.
    pub fn scroll(delta_x: f64, delta_y: f64) -> Self {
        // Determine direction from deltas
        let direction = if delta_y.abs() > delta_x.abs() {
            if delta_y > 0.0 {
                ScrollDirection::Down
            } else {
                ScrollDirection::Up
            }
        } else if delta_x > 0.0 {
            ScrollDirection::Right
        } else {
            ScrollDirection::Left
        };

        // Bucket the magnitude
        let total = (delta_x.abs() + delta_y.abs()) as i32;
        let magnitude = if total < 3 {
            ScrollMagnitude::Small
        } else if total <= 10 {
            ScrollMagnitude::Medium
        } else {
            ScrollMagnitude::Large
        };

        Self {
            timestamp: Utc::now(),
            event_type: MouseEventType::Scroll,
            delta_magnitude: None,
            scroll_direction: Some(direction),
            scroll_magnitude: Some(magnitude),
        }
    }
}

/// Unified event type for the collector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensorEvent {
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
}

impl SensorEvent {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            SensorEvent::Keyboard(e) => e.timestamp,
            SensorEvent::Mouse(e) => e.timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_event_creation() {
        let event = KeyboardEvent::new(true);
        assert!(event.is_key_down);
    }

    #[test]
    fn test_mouse_movement_magnitude() {
        let event = MouseEvent::movement(3.0, 4.0);
        assert_eq!(event.event_type, MouseEventType::Move);
        assert!((event.delta_magnitude.unwrap() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_scroll_bucketing() {
        let small = MouseEvent::scroll(0.0, 2.0);
        assert_eq!(small.scroll_magnitude, Some(ScrollMagnitude::Small));

        let medium = MouseEvent::scroll(0.0, 5.0);
        assert_eq!(medium.scroll_magnitude, Some(ScrollMagnitude::Medium));

        let large = MouseEvent::scroll(0.0, 15.0);
        assert_eq!(large.scroll_magnitude, Some(ScrollMagnitude::Large));
    }
}
