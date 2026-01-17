//! Core functionality for the Synheart Sensor Agent.
//!
//! This module contains:
//! - Window management for collecting events into time windows
//! - Feature computation from event windows
//! - HSI snapshot building for export

pub mod features;
pub mod hsi;
pub mod windowing;

// Re-export commonly used types
pub use features::{
    compute_features, BehavioralSignals, KeyboardFeatures, MouseFeatures, WindowFeatures,
};
pub use hsi::{HsiBuilder, HsiSnapshot, HSI_VERSION, PRODUCER_NAME};
pub use windowing::{EventWindow, WindowManager};
