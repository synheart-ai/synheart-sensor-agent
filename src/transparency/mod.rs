//! Transparency module for the Synheart Sensor Agent.
//!
//! This module provides tools for tracking and exposing what data
//! the agent collects, supporting user trust and regulatory compliance.

pub mod log;

// Re-export commonly used types
pub use log::{
    create_shared_log, create_shared_log_with_persistence, SharedTransparencyLog, TransparencyLog,
    TransparencyStats,
};
