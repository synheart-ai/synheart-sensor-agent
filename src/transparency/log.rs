//! Privacy-preserving transparency log.
//!
//! This module tracks and exposes statistics about data collection
//! without storing any personal or identifying information.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Transparency statistics for the current session.
#[derive(Debug)]
pub struct TransparencyLog {
    /// Number of keyboard events processed
    keyboard_events: AtomicU64,
    /// Number of mouse events processed
    mouse_events: AtomicU64,
    /// Number of windows completed
    windows_completed: AtomicU64,
    /// Number of HSI snapshots exported
    snapshots_exported: AtomicU64,
    /// Session start time
    session_start: DateTime<Utc>,
    /// Path for persisting stats
    persist_path: Option<PathBuf>,
}

impl TransparencyLog {
    /// Create a new transparency log.
    pub fn new() -> Self {
        Self {
            keyboard_events: AtomicU64::new(0),
            mouse_events: AtomicU64::new(0),
            windows_completed: AtomicU64::new(0),
            snapshots_exported: AtomicU64::new(0),
            session_start: Utc::now(),
            persist_path: None,
        }
    }

    /// Create a transparency log with persistence.
    pub fn with_persistence(path: PathBuf) -> Self {
        let mut log = Self::new();
        log.persist_path = Some(path);

        // Try to load existing stats
        if let Err(e) = log.load() {
            eprintln!("Note: Could not load previous transparency stats: {e}");
        }

        log
    }

    /// Record a keyboard event.
    pub fn record_keyboard_event(&self) {
        self.keyboard_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Record multiple keyboard events.
    pub fn record_keyboard_events(&self, count: u64) {
        self.keyboard_events.fetch_add(count, Ordering::Relaxed);
    }

    /// Record a mouse event.
    pub fn record_mouse_event(&self) {
        self.mouse_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Record multiple mouse events.
    pub fn record_mouse_events(&self, count: u64) {
        self.mouse_events.fetch_add(count, Ordering::Relaxed);
    }

    /// Record a completed window.
    pub fn record_window_completed(&self) {
        self.windows_completed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an exported snapshot.
    pub fn record_snapshot_exported(&self) {
        self.snapshots_exported.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the current statistics.
    pub fn stats(&self) -> TransparencyStats {
        TransparencyStats {
            keyboard_events: self.keyboard_events.load(Ordering::Relaxed),
            mouse_events: self.mouse_events.load(Ordering::Relaxed),
            windows_completed: self.windows_completed.load(Ordering::Relaxed),
            snapshots_exported: self.snapshots_exported.load(Ordering::Relaxed),
            session_start: self.session_start,
            session_duration_secs: (Utc::now() - self.session_start).num_seconds() as u64,
        }
    }

    /// Get a summary string for display.
    pub fn summary(&self) -> String {
        let stats = self.stats();
        format!(
            "Session Statistics:\n\
             - Keyboard events processed: {}\n\
             - Mouse events processed: {}\n\
             - Windows completed: {}\n\
             - Snapshots exported: {}\n\
             - Session duration: {} seconds\n\
             \n\
             Privacy Guarantee:\n\
             - No key content captured\n\
             - No cursor coordinates captured\n\
             - Only timing and magnitude data retained",
            stats.keyboard_events,
            stats.mouse_events,
            stats.windows_completed,
            stats.snapshots_exported,
            stats.session_duration_secs
        )
    }

    /// Save stats to disk.
    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(ref path) = self.persist_path {
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let stats = self.stats();
            let persisted = PersistedStats {
                keyboard_events: stats.keyboard_events,
                mouse_events: stats.mouse_events,
                windows_completed: stats.windows_completed,
                snapshots_exported: stats.snapshots_exported,
                last_updated: Utc::now(),
            };

            let json = serde_json::to_string_pretty(&persisted).map_err(std::io::Error::other)?;

            std::fs::write(path, json)?;
        }
        Ok(())
    }

    /// Load stats from disk.
    fn load(&mut self) -> Result<(), std::io::Error> {
        if let Some(ref path) = self.persist_path {
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                let persisted: PersistedStats =
                    serde_json::from_str(&content).map_err(std::io::Error::other)?;

                self.keyboard_events
                    .store(persisted.keyboard_events, Ordering::Relaxed);
                self.mouse_events
                    .store(persisted.mouse_events, Ordering::Relaxed);
                self.windows_completed
                    .store(persisted.windows_completed, Ordering::Relaxed);
                self.snapshots_exported
                    .store(persisted.snapshots_exported, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    /// Reset all counters.
    pub fn reset(&self) {
        self.keyboard_events.store(0, Ordering::Relaxed);
        self.mouse_events.store(0, Ordering::Relaxed);
        self.windows_completed.store(0, Ordering::Relaxed);
        self.snapshots_exported.store(0, Ordering::Relaxed);
    }
}

impl Default for TransparencyLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of transparency statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransparencyStats {
    pub keyboard_events: u64,
    pub mouse_events: u64,
    pub windows_completed: u64,
    pub snapshots_exported: u64,
    pub session_start: DateTime<Utc>,
    pub session_duration_secs: u64,
}

/// Stats format for persistence.
#[derive(Debug, Serialize, Deserialize)]
struct PersistedStats {
    keyboard_events: u64,
    mouse_events: u64,
    windows_completed: u64,
    snapshots_exported: u64,
    last_updated: DateTime<Utc>,
}

/// Thread-safe shared transparency log.
pub type SharedTransparencyLog = Arc<TransparencyLog>;

/// Create a new shared transparency log.
pub fn create_shared_log() -> SharedTransparencyLog {
    Arc::new(TransparencyLog::new())
}

/// Create a new shared transparency log with persistence.
pub fn create_shared_log_with_persistence(path: PathBuf) -> SharedTransparencyLog {
    Arc::new(TransparencyLog::with_persistence(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transparency_log_counting() {
        let log = TransparencyLog::new();

        log.record_keyboard_event();
        log.record_keyboard_event();
        log.record_mouse_event();

        let stats = log.stats();
        assert_eq!(stats.keyboard_events, 2);
        assert_eq!(stats.mouse_events, 1);
    }

    #[test]
    fn test_transparency_log_reset() {
        let log = TransparencyLog::new();

        log.record_keyboard_events(100);
        log.record_mouse_events(50);
        log.reset();

        let stats = log.stats();
        assert_eq!(stats.keyboard_events, 0);
        assert_eq!(stats.mouse_events, 0);
    }

    #[test]
    fn test_summary_format() {
        let log = TransparencyLog::new();
        let summary = log.summary();

        assert!(summary.contains("Keyboard events"));
        assert!(summary.contains("Mouse events"));
        assert!(summary.contains("Privacy Guarantee"));
        assert!(summary.contains("No key content captured"));
    }
}
