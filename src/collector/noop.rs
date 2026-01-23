//! Non-macOS (noop) implementation of event collection.
//!
//! This exists so the crate (and binary) can compile on non-Apple targets
//! without pulling in CoreGraphics/CoreFoundation dependencies.

use crate::collector::types::SensorEvent;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Configuration for which event sources to capture.
///
/// On non-macOS platforms this is accepted but no system events are captured.
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    pub capture_keyboard: bool,
    pub capture_mouse: bool,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            capture_keyboard: true,
            capture_mouse: true,
        }
    }
}

/// Errors that can occur during event collection.
#[derive(Debug)]
pub enum CollectorError {
    AlreadyRunning,
}

impl std::fmt::Display for CollectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollectorError::AlreadyRunning => write!(f, "Collector is already running"),
        }
    }
}

impl std::error::Error for CollectorError {}

/// A noop collector that never emits events.
pub struct NoopCollector {
    _config: CollectorConfig,
    _sender: Sender<SensorEvent>,
    receiver: Receiver<SensorEvent>,
    running: Arc<AtomicBool>,
}

impl NoopCollector {
    /// Create a new noop collector.
    pub fn new(config: CollectorConfig) -> Self {
        let (sender, receiver) = bounded(10_000);
        Self {
            _config: config,
            _sender: sender,
            receiver,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start capturing events.
    ///
    /// On non-macOS platforms, this simply marks the collector as running.
    pub fn start(&mut self) -> Result<(), CollectorError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(CollectorError::AlreadyRunning);
        }
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Stop capturing events.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if the collector is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the receiver for sensor events.
    pub fn receiver(&self) -> &Receiver<SensorEvent> {
        &self.receiver
    }

    /// Try to receive an event without blocking.
    pub fn try_recv(&self) -> Option<SensorEvent> {
        self.receiver.try_recv().ok()
    }
}

/// On non-macOS platforms there is no Input Monitoring permission gate.
pub fn check_permission() -> bool {
    true
}
