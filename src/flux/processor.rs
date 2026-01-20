//! Sensor-aware Flux processor for baseline tracking and HSI enrichment.
//!
//! This module wraps the synheart-flux BehaviorProcessor to provide
//! baseline tracking and HSI enrichment for sensor agent data.

use crate::core::features::WindowFeatures;
use crate::core::hsi::HsiSnapshot;
use crate::core::windowing::EventWindow;
use crate::flux::adapter::SensorBehaviorAdapter;
use serde::{Deserialize, Serialize};
use synheart_flux::behavior::BehaviorProcessor;
use synheart_flux::ComputeError;

/// Enriched snapshot with baseline-adjusted metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedSnapshot {
    /// Original sensor snapshot
    pub base: HsiSnapshot,
    /// Flux-computed behavioral metrics (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flux_behavior: Option<FluxBehaviorMetrics>,
    /// Baseline information (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<BaselineInfo>,
}

/// Flux-computed behavioral metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluxBehaviorMetrics {
    /// Distraction score (0.0 to 1.0)
    pub distraction_score: f64,
    /// Focus hint (inverse of distraction)
    pub focus_hint: f64,
    /// Task switch rate (normalized)
    pub task_switch_rate: f64,
    /// Notification load (normalized)
    pub notification_load: f64,
    /// Burstiness index (Barabasi formula)
    pub burstiness: f64,
    /// Scroll jitter rate
    pub scroll_jitter_rate: f64,
    /// Interaction intensity
    pub interaction_intensity: f64,
    /// Deep focus block count
    pub deep_focus_blocks: u32,
}

/// Baseline information for the current session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineInfo {
    /// Baseline distraction score
    pub distraction: Option<f64>,
    /// Baseline focus score
    pub focus: Option<f64>,
    /// Deviation from baseline (percentage)
    pub distraction_deviation_pct: Option<f64>,
    /// Number of sessions in the baseline
    pub sessions_in_baseline: u32,
}

/// Sensor-aware Flux processor for baseline tracking and HSI enrichment.
pub struct SensorFluxProcessor {
    /// Internal behavior processor
    processor: BehaviorProcessor,
    /// Adapter for converting sensor events
    adapter: SensorBehaviorAdapter,
    /// Session counter
    session_count: usize,
}

impl SensorFluxProcessor {
    /// Create a new processor with the specified baseline window size.
    ///
    /// # Arguments
    ///
    /// * `baseline_window_sessions` - Number of sessions to include in rolling baseline (default: 20)
    pub fn new(baseline_window_sessions: usize) -> Self {
        Self {
            processor: BehaviorProcessor::with_baseline_window(baseline_window_sessions),
            adapter: SensorBehaviorAdapter::with_defaults(),
            session_count: 0,
        }
    }

    /// Create with a custom device ID.
    pub fn with_device_id(baseline_window_sessions: usize, device_id: &str) -> Self {
        Self {
            processor: BehaviorProcessor::with_baseline_window(baseline_window_sessions),
            adapter: SensorBehaviorAdapter::new(device_id.to_string(), "UTC".to_string()),
            session_count: 0,
        }
    }

    /// Process a window and return an enriched snapshot with flux metrics.
    ///
    /// This converts the sensor window to a behavior session, processes it
    /// through the flux pipeline, and returns an enriched snapshot.
    pub fn process_window(
        &mut self,
        window: &EventWindow,
        _features: &WindowFeatures,
        base_snapshot: HsiSnapshot,
    ) -> Result<EnrichedSnapshot, ComputeError> {
        self.session_count += 1;
        let session_id = format!("sensor-{}", self.session_count);

        // Convert window to behavior session
        let session = self.adapter.convert(&session_id, window);

        // Serialize session to JSON for flux processing
        let session_json = serde_json::to_string(&session)
            .map_err(|e| ComputeError::EncodingError(e.to_string()))?;

        // Process through flux
        let hsi_json = self.processor.process(&session_json)?;

        // Extract metrics from HSI JSON
        let (flux_behavior, baseline) = extract_flux_metrics_from_json(&hsi_json)?;

        Ok(EnrichedSnapshot {
            base: base_snapshot,
            flux_behavior,
            baseline,
        })
    }

    /// Process a window without enrichment (just baseline update).
    ///
    /// This updates the baseline without returning enriched output.
    pub fn update_baseline(&mut self, window: &EventWindow) -> Result<(), ComputeError> {
        self.session_count += 1;
        let session_id = format!("sensor-{}", self.session_count);
        let session = self.adapter.convert(&session_id, window);

        let session_json = serde_json::to_string(&session)
            .map_err(|e| ComputeError::EncodingError(e.to_string()))?;

        let _ = self.processor.process(&session_json)?;
        Ok(())
    }

    /// Save baselines to JSON for persistence.
    pub fn save_baselines(&self) -> Result<String, ComputeError> {
        self.processor.save_baselines()
    }

    /// Load baselines from JSON.
    pub fn load_baselines(&mut self, json: &str) -> Result<(), ComputeError> {
        self.processor.load_baselines(json)
    }

    /// Get the number of sessions processed.
    pub fn session_count(&self) -> usize {
        self.session_count
    }
}

/// Extract flux metrics from HSI JSON string.
fn extract_flux_metrics_from_json(hsi_json: &str) -> Result<(Option<FluxBehaviorMetrics>, Option<BaselineInfo>), ComputeError> {
    let payload: serde_json::Value = serde_json::from_str(hsi_json)
        .map_err(|e| ComputeError::ParseError(e.to_string()))?;

    let windows = payload.get("behavior_windows")
        .and_then(|w| w.as_array());

    let window = match windows {
        Some(w) if !w.is_empty() => &w[0],
        _ => return Ok((None, None)),
    };

    // Extract behavior metrics
    let behavior = window.get("behavior");
    let flux_behavior = behavior.map(|b| FluxBehaviorMetrics {
        distraction_score: b.get("distraction_score").and_then(|v| v.as_f64()).unwrap_or(0.0),
        focus_hint: b.get("focus_hint").and_then(|v| v.as_f64()).unwrap_or(0.0),
        task_switch_rate: b.get("task_switch_rate").and_then(|v| v.as_f64()).unwrap_or(0.0),
        notification_load: b.get("notification_load").and_then(|v| v.as_f64()).unwrap_or(0.0),
        burstiness: b.get("burstiness").and_then(|v| v.as_f64()).unwrap_or(0.0),
        scroll_jitter_rate: b.get("scroll_jitter_rate").and_then(|v| v.as_f64()).unwrap_or(0.0),
        interaction_intensity: b.get("interaction_intensity").and_then(|v| v.as_f64()).unwrap_or(0.0),
        deep_focus_blocks: b.get("deep_focus_blocks").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
    });

    // Extract baseline
    let baseline_json = window.get("baseline");
    let baseline = baseline_json.map(|b| BaselineInfo {
        distraction: b.get("distraction").and_then(|v| v.as_f64()),
        focus: b.get("focus").and_then(|v| v.as_f64()),
        distraction_deviation_pct: b.get("distraction_deviation_pct").and_then(|v| v.as_f64()),
        sessions_in_baseline: b.get("sessions_in_baseline").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
    });

    Ok((flux_behavior, baseline))
}

impl Default for SensorFluxProcessor {
    fn default() -> Self {
        Self::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let processor = SensorFluxProcessor::new(20);
        assert_eq!(processor.session_count(), 0);
    }

    #[test]
    fn test_processor_with_device_id() {
        let processor = SensorFluxProcessor::with_device_id(20, "test-device");
        assert_eq!(processor.session_count(), 0);
    }
}
