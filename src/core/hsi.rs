//! HSI (Human Sensor Interface) snapshot builder.
//!
//! This module creates JSON snapshots according to the HSI RFC format.
//! Each snapshot represents a single time window of behavioral data.

use crate::core::features::WindowFeatures;
use crate::core::windowing::EventWindow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The current HSI format version.
pub const HSI_VERSION: &str = "1.0";

/// The name of this producer.
pub const PRODUCER_NAME: &str = "synheart-sensor-agent";

/// HSI snapshot containing all behavioral data for a time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsiSnapshot {
    /// HSI format version
    pub hsi_version: String,
    /// Producer metadata
    pub producer: ProducerInfo,
    /// Time window information
    pub window: WindowInfo,
    /// Extracted behavioral signals
    pub signals: Signals,
    /// Privacy declaration
    pub privacy: PrivacyDeclaration,
    /// Device information
    pub device: DeviceInfo,
    /// Optional session identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Information about the data producer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducerInfo {
    /// Name of the producing software
    pub name: String,
    /// Version of the producing software
    pub version: String,
    /// Unique instance identifier
    pub instance_id: Uuid,
}

/// Time window boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    /// ISO8601 timestamp of window start
    pub start: DateTime<Utc>,
    /// ISO8601 timestamp of window end
    pub end: DateTime<Utc>,
    /// Duration in seconds
    pub duration_secs: f64,
    /// Whether this window starts a new session
    pub is_session_start: bool,
}

/// Container for all signal types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signals {
    /// Behavioral signals extracted from the window
    pub behavior: BehaviorSignals,
    /// Raw event counts (for transparency)
    pub event_counts: EventCounts,
}

/// Behavioral signals from the window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSignals {
    /// Keyboard-derived features
    pub keyboard: KeyboardSignals,
    /// Mouse-derived features
    pub mouse: MouseSignals,
    /// Derived behavioral indicators
    pub derived: DerivedSignals,
}

/// Keyboard behavioral signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardSignals {
    pub typing_rate: f64,
    pub pause_count: u32,
    pub mean_pause_ms: f64,
    pub latency_variability: f64,
    pub hold_time_mean: f64,
    pub burst_index: f64,
    pub session_continuity: f64,
}

/// Mouse behavioral signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseSignals {
    pub mouse_activity_rate: f64,
    pub mean_velocity: f64,
    pub velocity_variability: f64,
    pub acceleration_spikes: u32,
    pub click_rate: f64,
    pub scroll_rate: f64,
    pub idle_ratio: f64,
    pub micro_adjustment_ratio: f64,
}

/// Derived behavioral indicators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedSignals {
    pub interaction_rhythm: f64,
    pub friction: f64,
    pub motor_stability: f64,
    pub focus_continuity_proxy: f64,
}

/// Event counts for transparency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCounts {
    pub keyboard_events: usize,
    pub mouse_events: usize,
    pub total_events: usize,
}

/// Privacy declaration for the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyDeclaration {
    /// Whether any key content was captured
    pub content_captured: bool,
    /// Whether any absolute coordinates were captured
    pub coordinates_captured: bool,
    /// What data categories are included
    pub data_categories: Vec<String>,
}

impl Default for PrivacyDeclaration {
    fn default() -> Self {
        Self {
            content_captured: false,
            coordinates_captured: false,
            data_categories: vec![
                "timing".to_string(),
                "magnitude".to_string(),
                "derived_features".to_string(),
            ],
        }
    }
}

/// Device/environment information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Operating system
    pub os: String,
    /// Agent version
    pub agent_version: String,
}

impl Default for DeviceInfo {
    fn default() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Builder for creating HSI snapshots.
pub struct HsiBuilder {
    instance_id: Uuid,
    session_id: Option<String>,
}

impl HsiBuilder {
    /// Create a new HSI builder with a unique instance ID.
    pub fn new() -> Self {
        Self {
            instance_id: Uuid::new_v4(),
            session_id: None,
        }
    }

    /// Set the session ID for generated snapshots.
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Get the instance ID.
    pub fn instance_id(&self) -> Uuid {
        self.instance_id
    }

    /// Build an HSI snapshot from a window and its computed features.
    pub fn build(&self, window: &EventWindow, features: &WindowFeatures) -> HsiSnapshot {
        HsiSnapshot {
            hsi_version: HSI_VERSION.to_string(),
            producer: ProducerInfo {
                name: PRODUCER_NAME.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                instance_id: self.instance_id,
            },
            window: WindowInfo {
                start: window.start,
                end: window.end,
                duration_secs: window.duration_secs(),
                is_session_start: window.is_session_start,
            },
            signals: Signals {
                behavior: BehaviorSignals {
                    keyboard: KeyboardSignals {
                        typing_rate: features.keyboard.typing_rate,
                        pause_count: features.keyboard.pause_count,
                        mean_pause_ms: features.keyboard.mean_pause_ms,
                        latency_variability: features.keyboard.latency_variability,
                        hold_time_mean: features.keyboard.hold_time_mean,
                        burst_index: features.keyboard.burst_index,
                        session_continuity: features.keyboard.session_continuity,
                    },
                    mouse: MouseSignals {
                        mouse_activity_rate: features.mouse.mouse_activity_rate,
                        mean_velocity: features.mouse.mean_velocity,
                        velocity_variability: features.mouse.velocity_variability,
                        acceleration_spikes: features.mouse.acceleration_spikes,
                        click_rate: features.mouse.click_rate,
                        scroll_rate: features.mouse.scroll_rate,
                        idle_ratio: features.mouse.idle_ratio,
                        micro_adjustment_ratio: features.mouse.micro_adjustment_ratio,
                    },
                    derived: DerivedSignals {
                        interaction_rhythm: features.behavioral.interaction_rhythm,
                        friction: features.behavioral.friction,
                        motor_stability: features.behavioral.motor_stability,
                        focus_continuity_proxy: features.behavioral.focus_continuity_proxy,
                    },
                },
                event_counts: EventCounts {
                    keyboard_events: window.keyboard_events.len(),
                    mouse_events: window.mouse_events.len(),
                    total_events: window.event_count(),
                },
            },
            privacy: PrivacyDeclaration::default(),
            device: DeviceInfo::default(),
            session_id: self.session_id.clone(),
        }
    }

    /// Build and serialize an HSI snapshot to JSON.
    pub fn build_json(&self, window: &EventWindow, features: &WindowFeatures) -> String {
        let snapshot = self.build(window, features);
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
    }
}

impl Default for HsiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::features::compute_features;
    use chrono::Duration;

    #[test]
    fn test_hsi_builder_instance_id() {
        let builder1 = HsiBuilder::new();
        let builder2 = HsiBuilder::new();
        assert_ne!(builder1.instance_id(), builder2.instance_id());
    }

    #[test]
    fn test_hsi_snapshot_creation() {
        let builder = HsiBuilder::new();
        let window = EventWindow::new(Utc::now(), Duration::seconds(10));
        let features = compute_features(&window);

        let snapshot = builder.build(&window, &features);

        assert_eq!(snapshot.hsi_version, HSI_VERSION);
        assert_eq!(snapshot.producer.name, PRODUCER_NAME);
        assert!(!snapshot.privacy.content_captured);
        assert!(!snapshot.privacy.coordinates_captured);
    }

    #[test]
    fn test_hsi_json_serialization() {
        let builder = HsiBuilder::new();
        let window = EventWindow::new(Utc::now(), Duration::seconds(10));
        let features = compute_features(&window);

        let json = builder.build_json(&window, &features);

        assert!(json.contains("hsi_version"));
        assert!(json.contains("producer"));
        assert!(json.contains("privacy"));
        assert!(json.contains("content_captured"));
    }
}
