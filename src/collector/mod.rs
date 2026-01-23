//! Event collection module for the Synheart Sensor Agent.
//!
//! This module provides platform-specific implementations for capturing
//! keyboard and mouse events in a privacy-preserving manner.

pub mod types;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(not(target_os = "macos"))]
pub mod noop;

// Re-export commonly used types
pub use types::{
    KeyboardEvent, MouseEvent, MouseEventType, ScrollDirection, ScrollMagnitude, SensorEvent,
};

#[cfg(target_os = "macos")]
pub use macos::{check_permission, CollectorConfig, CollectorError, MacOSCollector};

/// Platform-agnostic collector type alias
#[cfg(target_os = "macos")]
pub type Collector = MacOSCollector;

#[cfg(not(target_os = "macos"))]
pub use noop::{check_permission, CollectorConfig, CollectorError, NoopCollector};

/// Platform-agnostic collector type alias
#[cfg(not(target_os = "macos"))]
pub type Collector = NoopCollector;
