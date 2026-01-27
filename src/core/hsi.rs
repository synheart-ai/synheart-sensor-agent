//! HSI (Human State Interface) 1.0 compliant snapshot builder.
//!
//! This module creates JSON snapshots according to the HSI 1.0 specification.
//! Each snapshot represents a single time window of behavioral data.

use crate::core::features::WindowFeatures;
use crate::core::windowing::EventWindow;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// The current HSI format version.
pub const HSI_VERSION: &str = "1.0";

/// The name of this producer.
pub const PRODUCER_NAME: &str = "synheart-sensor-agent";

// ============================================================================
// HSI 1.0 Compliant Types
// ============================================================================

/// HSI 1.0 axis reading direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HsiDirection {
    HigherIsMore,
    HigherIsLess,
    Bidirectional,
}

/// HSI 1.0 source type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HsiSourceType {
    Sensor,
    App,
    SelfReport,
    Observer,
    Derived,
    Other,
}

/// HSI 1.0 producer metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsiProducer {
    /// Name of the producing software
    pub name: String,
    /// Version of the producing software
    pub version: String,
    /// Unique instance identifier (UUID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

/// HSI 1.0 window definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsiWindow {
    /// Window start time (RFC3339)
    pub start: String,
    /// Window end time (RFC3339)
    pub end: String,
    /// Optional label for the window
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// HSI 1.0 axis reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsiAxisReading {
    /// Axis name (lower_snake_case)
    pub axis: String,
    /// Score value (0-1) or null if unavailable
    pub score: Option<f64>,
    /// Confidence in the score (0-1)
    pub confidence: f64,
    /// Window ID this reading belongs to
    pub window_id: String,
    /// Direction semantics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<HsiDirection>,
    /// Unit of measurement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Source IDs that contributed to this reading
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_source_ids: Option<Vec<String>>,
    /// Notes about this reading
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// HSI 1.0 axes domain (contains readings array)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsiAxesDomain {
    /// Axis readings
    pub readings: Vec<HsiAxisReading>,
}

/// HSI 1.0 axes container
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HsiAxes {
    /// Affect domain readings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affect: Option<HsiAxesDomain>,
    /// Engagement domain readings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engagement: Option<HsiAxesDomain>,
    /// Behavior domain readings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<HsiAxesDomain>,
}

/// HSI 1.0 source definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsiSource {
    /// Source type
    #[serde(rename = "type")]
    pub source_type: HsiSourceType,
    /// Quality of the source (0-1)
    pub quality: f64,
    /// Whether the source is degraded
    pub degraded: bool,
    /// Optional notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// HSI 1.0 privacy declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsiPrivacy {
    /// Must be false - HSI payloads must not contain PII
    pub contains_pii: bool,
    /// Whether raw biosignals are allowed
    pub raw_biosignals_allowed: bool,
    /// Whether derived metrics are allowed
    pub derived_metrics_allowed: bool,
    /// Notes about privacy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl Default for HsiPrivacy {
    fn default() -> Self {
        Self {
            contains_pii: false,
            raw_biosignals_allowed: false,
            derived_metrics_allowed: true,
            notes: Some(
                "No key content or coordinates captured - timing and magnitude only".to_string(),
            ),
        }
    }
}

/// HSI 1.0 compliant snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsiSnapshot {
    /// HSI schema version (must be "1.0")
    pub hsi_version: String,
    /// When the human state was observed (RFC3339)
    pub observed_at_utc: String,
    /// When this payload was computed (RFC3339)
    pub computed_at_utc: String,
    /// Producer metadata
    pub producer: HsiProducer,
    /// Window identifiers
    pub window_ids: Vec<String>,
    /// Window definitions keyed by ID
    pub windows: HashMap<String, HsiWindow>,
    /// Source identifiers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ids: Option<Vec<String>>,
    /// Source definitions keyed by ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<HashMap<String, HsiSource>>,
    /// Axis readings by domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub axes: Option<HsiAxes>,
    /// Privacy declaration
    pub privacy: HsiPrivacy,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, serde_json::Value>>,
}

/// Builder for creating HSI 1.0 compliant snapshots.
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

    /// Build an HSI 1.0 compliant snapshot from a window and its computed features.
    pub fn build(&self, window: &EventWindow, features: &WindowFeatures) -> HsiSnapshot {
        let computed_at = Utc::now();

        // Generate window ID
        let window_id = format!("w_{}", computed_at.timestamp_millis());

        // Build windows map
        let mut windows = HashMap::new();
        windows.insert(
            window_id.clone(),
            HsiWindow {
                start: window.start.to_rfc3339(),
                end: window.end.to_rfc3339(),
                label: if window.is_session_start {
                    Some("session_start".to_string())
                } else {
                    None
                },
            },
        );

        // Build source
        let source_id = format!("s_keyboard_mouse_{}", self.instance_id);
        let mut sources = HashMap::new();

        // Calculate quality based on event count
        let event_count = window.event_count();
        let quality = if event_count == 0 {
            0.0
        } else if event_count < 10 {
            0.5
        } else if event_count < 50 {
            0.75
        } else {
            0.95
        };

        sources.insert(
            source_id.clone(),
            HsiSource {
                source_type: HsiSourceType::Sensor,
                quality,
                degraded: event_count < 10,
                notes: if event_count < 10 {
                    Some("Low event count in window".to_string())
                } else {
                    None
                },
            },
        );

        // Calculate confidence based on data availability
        let confidence = quality * 0.9; // Slightly lower than quality

        // Build behavioral axis readings
        let behavior_readings = vec![
            // Typing rate (normalized to 0-1 by clamping to max 10 keys/sec)
            HsiAxisReading {
                axis: "typing_rate".to_string(),
                score: Some((features.keyboard.typing_rate / 10.0).min(1.0)),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::HigherIsMore),
                unit: Some("keys_per_sec_normalized".to_string()),
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: None,
            },
            // Burst index (already 0-1)
            HsiAxisReading {
                axis: "typing_burstiness".to_string(),
                score: Some(features.keyboard.burst_index),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::Bidirectional),
                unit: None,
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: Some("Clustering of keystrokes".to_string()),
            },
            // Session continuity (already 0-1)
            HsiAxisReading {
                axis: "session_continuity".to_string(),
                score: Some(features.keyboard.session_continuity),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::HigherIsMore),
                unit: None,
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: None,
            },
            // Idle ratio (already 0-1)
            HsiAxisReading {
                axis: "idle_ratio".to_string(),
                score: Some(features.mouse.idle_ratio),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::HigherIsLess),
                unit: Some("ratio".to_string()),
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: None,
            },
            // Focus continuity proxy (already 0-1)
            HsiAxisReading {
                axis: "focus_continuity".to_string(),
                score: Some(features.behavioral.focus_continuity_proxy),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::HigherIsMore),
                unit: None,
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: Some("Derived from typing and mouse patterns".to_string()),
            },
            // Interaction rhythm (already 0-1)
            HsiAxisReading {
                axis: "interaction_rhythm".to_string(),
                score: Some(features.behavioral.interaction_rhythm),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::HigherIsMore),
                unit: None,
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: None,
            },
            // Motor stability (already 0-1)
            HsiAxisReading {
                axis: "motor_stability".to_string(),
                score: Some(features.behavioral.motor_stability),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::HigherIsMore),
                unit: None,
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: None,
            },
            // Friction (already 0-1)
            HsiAxisReading {
                axis: "friction".to_string(),
                score: Some(features.behavioral.friction),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::HigherIsMore),
                unit: None,
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: Some("Micro-adjustments and hesitation".to_string()),
            },
            // Typing cadence stability (already 0-1)
            HsiAxisReading {
                axis: "typing_cadence_stability".to_string(),
                score: Some(features.keyboard.typing_cadence_stability),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::HigherIsMore),
                unit: None,
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: Some("Rhythmic consistency of typing".to_string()),
            },
            // Typing gap ratio (already 0-1)
            HsiAxisReading {
                axis: "typing_gap_ratio".to_string(),
                score: Some(features.keyboard.typing_gap_ratio),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::HigherIsLess),
                unit: Some("ratio".to_string()),
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: Some("Proportion of inter-tap intervals classified as gaps".to_string()),
            },
            // Typing interaction intensity (already 0-1)
            HsiAxisReading {
                axis: "typing_interaction_intensity".to_string(),
                score: Some(features.keyboard.typing_interaction_intensity),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::HigherIsMore),
                unit: None,
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: Some("Composite of speed, cadence stability, and gap behavior".to_string()),
            },
            // Keyboard scroll rate (normalized to 0-1, capped at 5 keys/sec)
            HsiAxisReading {
                axis: "keyboard_scroll_rate".to_string(),
                score: Some((features.keyboard.keyboard_scroll_rate / 5.0).min(1.0)),
                confidence,
                window_id: window_id.clone(),
                direction: Some(HsiDirection::HigherIsMore),
                unit: Some("nav_keys_per_sec_normalized".to_string()),
                evidence_source_ids: Some(vec![source_id.clone()]),
                notes: Some(
                    "Navigation keys (arrows, page up/down) - separate from mouse scroll"
                        .to_string(),
                ),
            },
        ];

        // Build axes
        let axes = HsiAxes {
            affect: None,
            engagement: None,
            behavior: Some(HsiAxesDomain {
                readings: behavior_readings,
            }),
        };

        // Build metadata
        let mut meta = HashMap::new();
        meta.insert(
            "keyboard_events".to_string(),
            serde_json::Value::Number(serde_json::Number::from(window.keyboard_events.len())),
        );
        meta.insert(
            "mouse_events".to_string(),
            serde_json::Value::Number(serde_json::Number::from(window.mouse_events.len())),
        );
        meta.insert(
            "duration_secs".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(window.duration_secs())
                    .unwrap_or(serde_json::Number::from(0)),
            ),
        );
        meta.insert(
            "is_session_start".to_string(),
            serde_json::Value::Bool(window.is_session_start),
        );
        if let Some(ref session_id) = self.session_id {
            meta.insert(
                "session_id".to_string(),
                serde_json::Value::String(session_id.clone()),
            );
        }
        // Include raw feature values in meta for transparency
        meta.insert(
            "raw_typing_rate".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(features.keyboard.typing_rate)
                    .unwrap_or(serde_json::Number::from(0)),
            ),
        );
        meta.insert(
            "raw_mean_velocity".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(features.mouse.mean_velocity)
                    .unwrap_or(serde_json::Number::from(0)),
            ),
        );
        meta.insert(
            "raw_click_rate".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(features.mouse.click_rate)
                    .unwrap_or(serde_json::Number::from(0)),
            ),
        );
        meta.insert(
            "typing_tap_count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(features.keyboard.typing_tap_count)),
        );
        meta.insert(
            "navigation_key_count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(
                features.keyboard.navigation_key_count,
            )),
        );
        meta.insert(
            "keyboard_scroll_rate".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(features.keyboard.keyboard_scroll_rate)
                    .unwrap_or(serde_json::Number::from(0)),
            ),
        );

        HsiSnapshot {
            hsi_version: HSI_VERSION.to_string(),
            observed_at_utc: window.end.to_rfc3339(),
            computed_at_utc: computed_at.to_rfc3339(),
            producer: HsiProducer {
                name: PRODUCER_NAME.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                instance_id: Some(self.instance_id.to_string()),
            },
            window_ids: vec![window_id],
            windows,
            source_ids: Some(vec![source_id]),
            sources: Some(sources),
            axes: Some(axes),
            privacy: HsiPrivacy::default(),
            meta: Some(meta),
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
        assert!(!snapshot.privacy.contains_pii);
        assert!(snapshot.privacy.derived_metrics_allowed);
    }

    #[test]
    fn test_hsi_1_0_compliance() {
        let builder = HsiBuilder::new();
        let window = EventWindow::new(Utc::now(), Duration::seconds(10));
        let features = compute_features(&window);

        let snapshot = builder.build(&window, &features);

        // Check required top-level fields
        assert_eq!(snapshot.hsi_version, "1.0");
        assert!(!snapshot.observed_at_utc.is_empty());
        assert!(!snapshot.computed_at_utc.is_empty());
        assert!(!snapshot.window_ids.is_empty());
        assert!(!snapshot.windows.is_empty());

        // Check window_ids match windows keys
        for id in &snapshot.window_ids {
            assert!(snapshot.windows.contains_key(id));
        }

        // Check privacy constraints
        assert!(!snapshot.privacy.contains_pii);

        // Check axes structure
        let axes = snapshot.axes.as_ref().unwrap();
        let behavior = axes.behavior.as_ref().unwrap();
        assert!(!behavior.readings.is_empty());

        // Check each reading has required fields
        for reading in &behavior.readings {
            assert!(!reading.axis.is_empty());
            assert!(reading.confidence >= 0.0 && reading.confidence <= 1.0);
            assert!(!reading.window_id.is_empty());
            if let Some(score) = reading.score {
                assert!((0.0..=1.0).contains(&score), "score out of range: {score}");
            }
        }
    }

    #[test]
    fn test_hsi_json_serialization() {
        let builder = HsiBuilder::new();
        let window = EventWindow::new(Utc::now(), Duration::seconds(10));
        let features = compute_features(&window);

        let json = builder.build_json(&window, &features);

        // Verify JSON contains required fields
        assert!(json.contains("hsi_version"));
        assert!(json.contains("observed_at_utc"));
        assert!(json.contains("computed_at_utc"));
        assert!(json.contains("producer"));
        assert!(json.contains("window_ids"));
        assert!(json.contains("windows"));
        assert!(json.contains("privacy"));
        assert!(json.contains("contains_pii"));
    }

    #[test]
    fn test_source_quality_calculation() {
        let builder = HsiBuilder::new();
        let window = EventWindow::new(Utc::now(), Duration::seconds(10));
        let features = compute_features(&window);

        let snapshot = builder.build(&window, &features);

        let sources = snapshot.sources.as_ref().unwrap();
        let source = sources.values().next().unwrap();

        // Empty window should have low quality and be degraded
        assert!(source.quality < 0.5);
        assert!(source.degraded);
    }
}
