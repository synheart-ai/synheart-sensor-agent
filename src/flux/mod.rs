//! Synheart Flux integration module.
//!
//! This module provides integration with synheart-flux for:
//! - Baseline tracking across sessions
//! - HSI-compliant behavioral metrics enrichment
//! - Cross-session deviation analysis
//!
//! # Feature Flag
//!
//! This module is only available when the `flux` feature is enabled:
//!
//! ```toml
//! [dependencies]
//! synheart-sensor-agent = { version = "0.1", features = ["flux"] }
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use synheart_sensor_agent::flux::SensorFluxProcessor;
//!
//! // Create processor with baseline tracking
//! let mut processor = SensorFluxProcessor::new(20); // 20 session baseline window
//!
//! // Process a window and get enriched output
//! // processor.process_window(&window, &features, base_snapshot)
//! ```

mod adapter;
mod processor;

pub use adapter::{convert_to_behavior_session, SensorBehaviorAdapter};
pub use processor::{EnrichedSnapshot, SensorFluxProcessor};
