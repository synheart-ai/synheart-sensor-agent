//! Feature computation from event windows.
//!
//! This module extracts behavioral features from time windows of events.
//! All features are computed from timing and magnitude data only - never content.

use crate::collector::types::{KeyboardEvent, KeyboardEventType, MouseEvent, MouseEventType};
use crate::core::windowing::EventWindow;
use serde::{Deserialize, Serialize};

/// Keyboard-derived behavioral features.
///
/// Note: Typing metrics (typing_rate, typing_tap_count, etc.) are computed from
/// typing keys ONLY. Navigation keys (arrows, page up/down, home/end) are tracked
/// separately via keyboard_scroll_rate to avoid inflating typing metrics during
/// navigation-heavy text editing sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyboardFeatures {
    /// Typing keys per second (excludes navigation keys)
    pub typing_rate: f64,
    /// Number of idle gaps (pauses) per window
    pub pause_count: u32,
    /// Average pause duration in milliseconds
    pub mean_pause_ms: f64,
    /// Standard deviation of inter-key intervals
    pub latency_variability: f64,
    /// Average key hold duration in milliseconds
    pub hold_time_mean: f64,
    /// Burstiness index (0-1, higher = more bursty)
    pub burst_index: f64,
    /// Ratio of active typing time to total window time
    pub session_continuity: f64,
    /// Total number of discrete typing tap events (excludes navigation keys)
    pub typing_tap_count: u32,
    /// Normalized rhythmic consistency score (0-1, higher = more regular timing)
    pub typing_cadence_stability: f64,
    /// Proportion of inter-tap intervals classified as gaps
    pub typing_gap_ratio: f64,
    /// Composite metric combining speed, cadence stability, and gap behavior (0-1)
    pub typing_interaction_intensity: f64,
    /// Navigation key events per second (arrow keys, page up/down, home/end)
    /// Tracked separately from typing to distinguish keyboard scrolling from mouse scrolling
    pub keyboard_scroll_rate: f64,
    /// Total navigation key events in the window
    pub navigation_key_count: u32,
}

/// Mouse-derived behavioral features.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MouseFeatures {
    /// Movement events per second
    pub mouse_activity_rate: f64,
    /// Average cursor speed (relative units)
    pub mean_velocity: f64,
    /// Standard deviation of velocity
    pub velocity_variability: f64,
    /// Count of sudden acceleration changes
    pub acceleration_spikes: u32,
    /// Clicks per window
    pub click_rate: f64,
    /// Scroll events per window
    pub scroll_rate: f64,
    /// Ratio of idle time to active time
    pub idle_ratio: f64,
    /// Ratio of small movements to total movements
    pub micro_adjustment_ratio: f64,
}

/// Derived behavioral signals combining keyboard and mouse data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BehavioralSignals {
    /// Overall interaction rhythm (regularity of input)
    pub interaction_rhythm: f64,
    /// Friction indicator (hesitation, corrections)
    pub friction: f64,
    /// Motor stability (consistency of movements)
    pub motor_stability: f64,
    /// Proxy for focus/attention continuity
    pub focus_continuity_proxy: f64,
}

/// All computed features for a window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindowFeatures {
    pub keyboard: KeyboardFeatures,
    pub mouse: MouseFeatures,
    pub behavioral: BehavioralSignals,
}

/// Threshold for considering a gap as a "pause" (in milliseconds).
const PAUSE_THRESHOLD_MS: i64 = 500;

/// Threshold for micro-adjustments (in movement magnitude units).
const MICRO_ADJUSTMENT_THRESHOLD: f64 = 5.0;

/// Threshold for acceleration spikes (change in velocity).
const ACCELERATION_SPIKE_THRESHOLD: f64 = 50.0;

/// Compute all features from an event window.
pub fn compute_features(window: &EventWindow) -> WindowFeatures {
    let keyboard = compute_keyboard_features(&window.keyboard_events, window.duration_secs());
    let mouse = compute_mouse_features(&window.mouse_events, window.duration_secs());
    let behavioral = compute_behavioral_signals(&keyboard, &mouse);

    WindowFeatures {
        keyboard,
        mouse,
        behavioral,
    }
}

/// Compute keyboard features from a list of keyboard events.
///
/// Typing metrics are computed from typing key events ONLY (excludes navigation keys).
/// Navigation keys (arrows, page up/down, home/end) are tracked separately via
/// keyboard_scroll_rate to distinguish keyboard scrolling from mouse scrolling.
fn compute_keyboard_features(events: &[KeyboardEvent], window_duration: f64) -> KeyboardFeatures {
    if events.is_empty() || window_duration <= 0.0 {
        return KeyboardFeatures::default();
    }

    // Separate typing events from navigation events
    let typing_events: Vec<&KeyboardEvent> = events
        .iter()
        .filter(|e| e.event_type == KeyboardEventType::TypingTap)
        .collect();

    let navigation_events: Vec<&KeyboardEvent> = events
        .iter()
        .filter(|e| e.event_type == KeyboardEventType::NavigationKey)
        .collect();

    // Count navigation key presses (key down events only)
    let navigation_key_presses: Vec<&KeyboardEvent> = navigation_events
        .iter()
        .filter(|e| e.is_key_down)
        .copied()
        .collect();
    let navigation_key_count = navigation_key_presses.len() as u32;
    let keyboard_scroll_rate = navigation_key_count as f64 / window_duration;

    // Count typing key presses (key down events only) - EXCLUDES navigation keys
    let typing_key_presses: Vec<&KeyboardEvent> = typing_events
        .iter()
        .filter(|e| e.is_key_down)
        .copied()
        .collect();
    let typing_tap_count = typing_key_presses.len() as u32;

    // Typing rate (typing keys only)
    let typing_rate = typing_tap_count as f64 / window_duration;

    // Compute inter-key intervals for typing key presses only
    let intervals: Vec<i64> = typing_key_presses
        .windows(2)
        .map(|pair| (pair[1].timestamp - pair[0].timestamp).num_milliseconds())
        .collect();

    // Pause count and mean pause duration
    let pauses: Vec<i64> = intervals
        .iter()
        .filter(|&&i| i > PAUSE_THRESHOLD_MS)
        .copied()
        .collect();
    let pause_count = pauses.len() as u32;
    let mean_pause_ms = if pauses.is_empty() {
        0.0
    } else {
        pauses.iter().sum::<i64>() as f64 / pauses.len() as f64
    };

    // Latency variability (std dev of intervals)
    let latency_variability = std_dev(&intervals.iter().map(|&i| i as f64).collect::<Vec<_>>());

    // Hold time computation (requires matching key down/up pairs)
    // Only compute from typing events to avoid navigation key hold times
    let hold_times = compute_hold_times(&typing_events);
    let hold_time_mean = if hold_times.is_empty() {
        0.0
    } else {
        hold_times.iter().sum::<f64>() / hold_times.len() as f64
    };

    // Burst index: ratio of short intervals to all intervals
    // Short interval = less than 100ms (fast typing burst)
    let short_interval_count = intervals.iter().filter(|&&i| i < 100).count();
    let burst_index = if intervals.is_empty() {
        0.0
    } else {
        short_interval_count as f64 / intervals.len() as f64
    };

    // Session continuity: ratio of active time to total window time
    // Active time is sum of intervals (excluding long pauses)
    let active_intervals: Vec<i64> = intervals
        .iter()
        .filter(|&&i| i <= PAUSE_THRESHOLD_MS * 2) // Allow some breathing room
        .copied()
        .collect();
    let active_time_ms: i64 = active_intervals.iter().sum();
    let session_continuity = (active_time_ms as f64 / 1000.0) / window_duration;

    // Typing cadence stability: normalized rhythmic consistency (0-1, higher = more regular)
    // Inverse relationship with latency variability
    let typing_cadence_stability = 1.0 / (1.0 + latency_variability / 100.0);

    // Typing gap ratio: proportion of inter-tap intervals classified as gaps
    let typing_gap_ratio = if intervals.is_empty() {
        0.0
    } else {
        pause_count as f64 / intervals.len() as f64
    };

    // Typing interaction intensity: composite metric (0-1)
    // Combines normalized speed, cadence stability, and inverse gap ratio
    let normalized_speed = (typing_rate / 10.0).min(1.0); // Normalize to ~10 keys/sec max
    let typing_interaction_intensity = (normalized_speed * 0.4
        + typing_cadence_stability * 0.3
        + (1.0 - typing_gap_ratio) * 0.3)
        .clamp(0.0, 1.0);

    KeyboardFeatures {
        typing_rate,
        pause_count,
        mean_pause_ms,
        latency_variability,
        hold_time_mean,
        burst_index,
        session_continuity: session_continuity.min(1.0), // Cap at 1.0
        typing_tap_count,
        typing_cadence_stability,
        typing_gap_ratio,
        typing_interaction_intensity,
        keyboard_scroll_rate,
        navigation_key_count,
    }
}

/// Estimate hold times from event sequence.
fn compute_hold_times(events: &[&KeyboardEvent]) -> Vec<f64> {
    let mut hold_times = Vec::new();
    let mut last_down: Option<&KeyboardEvent> = None;

    for event in events {
        if event.is_key_down {
            last_down = Some(event);
        } else if let Some(down) = last_down {
            let hold_ms = (event.timestamp - down.timestamp).num_milliseconds() as f64;
            // Filter out unreasonable hold times (< 20ms or > 2000ms)
            if (20.0..=2000.0).contains(&hold_ms) {
                hold_times.push(hold_ms);
            }
            last_down = None;
        }
    }

    hold_times
}

/// Compute mouse features from a list of mouse events.
fn compute_mouse_features(events: &[MouseEvent], window_duration: f64) -> MouseFeatures {
    if events.is_empty() || window_duration <= 0.0 {
        return MouseFeatures::default();
    }

    // Categorize events
    let move_events: Vec<&MouseEvent> = events
        .iter()
        .filter(|e| e.event_type == MouseEventType::Move)
        .collect();

    let click_events: Vec<&MouseEvent> = events
        .iter()
        .filter(|e| {
            e.event_type == MouseEventType::LeftClick || e.event_type == MouseEventType::RightClick
        })
        .collect();

    let scroll_events: Vec<&MouseEvent> = events
        .iter()
        .filter(|e| e.event_type == MouseEventType::Scroll)
        .collect();

    // Mouse activity rate (movements per second)
    let mouse_activity_rate = move_events.len() as f64 / window_duration;

    // Velocity statistics
    let velocities: Vec<f64> = move_events
        .iter()
        .filter_map(|e| e.delta_magnitude)
        .collect();

    let mean_velocity = if velocities.is_empty() {
        0.0
    } else {
        velocities.iter().sum::<f64>() / velocities.len() as f64
    };

    let velocity_variability = std_dev(&velocities);

    // Acceleration spikes (large changes in velocity)
    let acceleration_spikes = velocities
        .windows(2)
        .filter(|pair| (pair[1] - pair[0]).abs() > ACCELERATION_SPIKE_THRESHOLD)
        .count() as u32;

    // Click and scroll rates
    let click_rate = click_events.len() as f64 / window_duration;
    let scroll_rate = scroll_events.len() as f64 / window_duration;

    // Idle ratio: estimate based on gaps in movement events
    let idle_ratio = estimate_idle_ratio(&move_events, window_duration);

    // Micro-adjustment ratio: small movements vs all movements
    let micro_count = velocities
        .iter()
        .filter(|&&v| v < MICRO_ADJUSTMENT_THRESHOLD)
        .count();
    let micro_adjustment_ratio = if velocities.is_empty() {
        0.0
    } else {
        micro_count as f64 / velocities.len() as f64
    };

    MouseFeatures {
        mouse_activity_rate,
        mean_velocity,
        velocity_variability,
        acceleration_spikes,
        click_rate,
        scroll_rate,
        idle_ratio,
        micro_adjustment_ratio,
    }
}

/// Estimate idle ratio from movement event gaps.
fn estimate_idle_ratio(move_events: &[&MouseEvent], window_duration: f64) -> f64 {
    if move_events.len() < 2 {
        return 1.0; // No movement = all idle
    }

    // Consider gaps > 1 second as "idle"
    const IDLE_THRESHOLD_MS: i64 = 1000;

    let mut idle_time_ms: i64 = 0;
    for pair in move_events.windows(2) {
        let gap = (pair[1].timestamp - pair[0].timestamp).num_milliseconds();
        if gap > IDLE_THRESHOLD_MS {
            idle_time_ms += gap - IDLE_THRESHOLD_MS; // Count only the excess as idle
        }
    }

    let idle_secs = idle_time_ms as f64 / 1000.0;
    (idle_secs / window_duration).min(1.0)
}

/// Compute derived behavioral signals from keyboard and mouse features.
fn compute_behavioral_signals(
    keyboard: &KeyboardFeatures,
    mouse: &MouseFeatures,
) -> BehavioralSignals {
    // Interaction rhythm: combines typing regularity and mouse consistency
    // Lower variability = more rhythmic
    let typing_rhythm = 1.0 / (1.0 + keyboard.latency_variability / 100.0);
    let mouse_rhythm = 1.0 / (1.0 + mouse.velocity_variability / 50.0);
    let interaction_rhythm = (typing_rhythm + mouse_rhythm) / 2.0;

    // Friction: indicates hesitation, uncertainty
    // High pause rate, low burst index, many micro-adjustments
    let friction = (keyboard.pause_count as f64 * 0.1)
        + (1.0 - keyboard.burst_index) * 0.3
        + mouse.micro_adjustment_ratio * 0.3;

    // Motor stability: consistency of physical movements
    // Low variability in both keyboard and mouse
    let motor_stability = 1.0
        - (keyboard.latency_variability / 200.0).min(0.5)
        - (mouse.velocity_variability / 100.0).min(0.5);

    // Focus continuity proxy: sustained activity patterns
    // High session continuity, low idle ratio
    let focus_continuity_proxy = keyboard.session_continuity * 0.5 + (1.0 - mouse.idle_ratio) * 0.5;

    BehavioralSignals {
        interaction_rhythm: interaction_rhythm.clamp(0.0, 1.0),
        friction: friction.clamp(0.0, 1.0),
        motor_stability: motor_stability.clamp(0.0, 1.0),
        focus_continuity_proxy: focus_continuity_proxy.clamp(0.0, 1.0),
    }
}

/// Compute standard deviation of a slice of values.
fn std_dev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_keyboard_event(is_down: bool, offset_ms: i64) -> KeyboardEvent {
        KeyboardEvent {
            timestamp: Utc::now() + Duration::milliseconds(offset_ms),
            is_key_down: is_down,
            event_type: KeyboardEventType::TypingTap,
        }
    }

    fn make_navigation_event(is_down: bool, offset_ms: i64) -> KeyboardEvent {
        KeyboardEvent {
            timestamp: Utc::now() + Duration::milliseconds(offset_ms),
            is_key_down: is_down,
            event_type: KeyboardEventType::NavigationKey,
        }
    }

    #[test]
    fn test_keyboard_features_empty() {
        let features = compute_keyboard_features(&[], 10.0);
        assert_eq!(features.typing_rate, 0.0);
    }

    #[test]
    fn test_keyboard_features_basic() {
        let events = vec![
            make_keyboard_event(true, 0),
            make_keyboard_event(false, 50),
            make_keyboard_event(true, 100),
            make_keyboard_event(false, 150),
            make_keyboard_event(true, 200),
            make_keyboard_event(false, 250),
        ];

        let features = compute_keyboard_features(&events, 1.0);
        assert_eq!(features.typing_rate, 3.0); // 3 key presses in 1 second
    }

    #[test]
    fn test_std_dev() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = std_dev(&values);
        assert!((sd - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_behavioral_signals_bounds() {
        let keyboard = KeyboardFeatures::default();
        let mouse = MouseFeatures::default();
        let signals = compute_behavioral_signals(&keyboard, &mouse);

        // All signals should be between 0 and 1
        assert!(signals.interaction_rhythm >= 0.0 && signals.interaction_rhythm <= 1.0);
        assert!(signals.friction >= 0.0 && signals.friction <= 1.0);
        assert!(signals.motor_stability >= 0.0 && signals.motor_stability <= 1.0);
        assert!(signals.focus_continuity_proxy >= 0.0 && signals.focus_continuity_proxy <= 1.0);
    }

    #[test]
    fn test_typing_tap_count() {
        let events = vec![
            make_keyboard_event(true, 0),
            make_keyboard_event(false, 50),
            make_keyboard_event(true, 100),
            make_keyboard_event(false, 150),
            make_keyboard_event(true, 200),
            make_keyboard_event(false, 250),
        ];

        let features = compute_keyboard_features(&events, 1.0);
        assert_eq!(features.typing_tap_count, 3); // 3 key presses
    }

    #[test]
    fn test_typing_cadence_stability_bounds() {
        // Empty events should give default (which uses 0 variability)
        let features_empty = compute_keyboard_features(&[], 10.0);
        assert!(
            features_empty.typing_cadence_stability >= 0.0
                && features_empty.typing_cadence_stability <= 1.0
        );

        // Regular typing should have high cadence stability
        let events = vec![
            make_keyboard_event(true, 0),
            make_keyboard_event(false, 50),
            make_keyboard_event(true, 100),
            make_keyboard_event(false, 150),
            make_keyboard_event(true, 200),
            make_keyboard_event(false, 250),
        ];
        let features = compute_keyboard_features(&events, 1.0);
        assert!(
            features.typing_cadence_stability >= 0.0 && features.typing_cadence_stability <= 1.0
        );
        // Regular intervals should yield high stability
        assert!(features.typing_cadence_stability > 0.5);
    }

    #[test]
    fn test_typing_gap_ratio_bounds() {
        let features_empty = compute_keyboard_features(&[], 10.0);
        assert_eq!(features_empty.typing_gap_ratio, 0.0);

        // Fast typing with no pauses
        let events = vec![
            make_keyboard_event(true, 0),
            make_keyboard_event(false, 50),
            make_keyboard_event(true, 100),
            make_keyboard_event(false, 150),
        ];
        let features = compute_keyboard_features(&events, 1.0);
        assert!(features.typing_gap_ratio >= 0.0 && features.typing_gap_ratio <= 1.0);
        assert_eq!(features.typing_gap_ratio, 0.0); // No gaps in fast typing

        // Typing with pauses (>500ms gaps)
        let events_with_gaps = vec![
            make_keyboard_event(true, 0),
            make_keyboard_event(false, 50),
            make_keyboard_event(true, 600), // 600ms gap = pause
            make_keyboard_event(false, 650),
        ];
        let features_gaps = compute_keyboard_features(&events_with_gaps, 1.0);
        assert!(features_gaps.typing_gap_ratio > 0.0); // Should have gaps
    }

    #[test]
    fn test_typing_interaction_intensity_bounds() {
        let features_empty = compute_keyboard_features(&[], 10.0);
        assert!(
            features_empty.typing_interaction_intensity >= 0.0
                && features_empty.typing_interaction_intensity <= 1.0
        );

        // High intensity: fast, regular, no gaps
        let fast_events = vec![
            make_keyboard_event(true, 0),
            make_keyboard_event(false, 30),
            make_keyboard_event(true, 60),
            make_keyboard_event(false, 90),
            make_keyboard_event(true, 120),
            make_keyboard_event(false, 150),
            make_keyboard_event(true, 180),
            make_keyboard_event(false, 210),
            make_keyboard_event(true, 240),
            make_keyboard_event(false, 270),
        ];
        let features = compute_keyboard_features(&fast_events, 1.0);
        assert!(
            features.typing_interaction_intensity >= 0.0
                && features.typing_interaction_intensity <= 1.0
        );
        // Fast regular typing should have moderate to high intensity
        assert!(features.typing_interaction_intensity > 0.3);
    }

    #[test]
    fn test_navigation_key_separation() {
        // Mix of typing and navigation events
        let events = vec![
            make_keyboard_event(true, 0),      // typing
            make_keyboard_event(false, 50),    // typing
            make_navigation_event(true, 100),  // navigation (arrow key)
            make_navigation_event(false, 150), // navigation
            make_keyboard_event(true, 200),    // typing
            make_keyboard_event(false, 250),   // typing
            make_navigation_event(true, 300),  // navigation
            make_navigation_event(false, 350), // navigation
        ];

        let features = compute_keyboard_features(&events, 1.0);

        // Should only count typing key presses (2 typing events)
        assert_eq!(features.typing_tap_count, 2);
        assert_eq!(features.typing_rate, 2.0);

        // Should count navigation key presses separately (2 navigation events)
        assert_eq!(features.navigation_key_count, 2);
        assert_eq!(features.keyboard_scroll_rate, 2.0);
    }

    #[test]
    fn test_navigation_keys_dont_inflate_typing_metrics() {
        // Only navigation events - typing metrics should be zero/default
        let nav_only_events = vec![
            make_navigation_event(true, 0),
            make_navigation_event(false, 50),
            make_navigation_event(true, 100),
            make_navigation_event(false, 150),
            make_navigation_event(true, 200),
            make_navigation_event(false, 250),
        ];

        let features = compute_keyboard_features(&nav_only_events, 1.0);

        // Typing metrics should be zero
        assert_eq!(features.typing_tap_count, 0);
        assert_eq!(features.typing_rate, 0.0);

        // Navigation metrics should be counted
        assert_eq!(features.navigation_key_count, 3);
        assert_eq!(features.keyboard_scroll_rate, 3.0);
    }

    #[test]
    fn test_keyboard_scroll_rate_bounds() {
        let features_empty = compute_keyboard_features(&[], 10.0);
        assert_eq!(features_empty.keyboard_scroll_rate, 0.0);
        assert_eq!(features_empty.navigation_key_count, 0);

        // Navigation-heavy session
        let nav_events = vec![
            make_navigation_event(true, 0),
            make_navigation_event(false, 30),
            make_navigation_event(true, 60),
            make_navigation_event(false, 90),
            make_navigation_event(true, 120),
            make_navigation_event(false, 150),
        ];
        let features = compute_keyboard_features(&nav_events, 1.0);
        assert_eq!(features.navigation_key_count, 3);
        assert!(features.keyboard_scroll_rate > 0.0);
    }
}
