//! Event collection module for the Synheart Sensor Agent.
//!
//! This module provides platform-specific implementations for capturing
//! keyboard and mouse events in a privacy-preserving manner.

pub mod types;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub mod noop;

// Re-export commonly used types
pub use types::{
    KeyboardEvent, MouseEvent, MouseEventType, ScrollDirection, ScrollMagnitude, SensorEvent,
};

// macOS exports
#[cfg(target_os = "macos")]
pub use macos::{check_permission, CollectorConfig, CollectorError, MacOSCollector};

/// Platform-agnostic collector type alias
#[cfg(target_os = "macos")]
pub type Collector = MacOSCollector;

// Windows exports
#[cfg(target_os = "windows")]
pub use windows::{check_permission, CollectorConfig, CollectorError, WindowsCollector};

/// Platform-agnostic collector type alias
#[cfg(target_os = "windows")]
pub type Collector = WindowsCollector;

// Fallback for other platforms
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub use noop::{check_permission, CollectorConfig, CollectorError, NoopCollector};

/// Platform-agnostic collector type alias
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub type Collector = NoopCollector;
