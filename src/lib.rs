//! Synheart Sensor Agent - Privacy-first behavioral sensor for research.
//!
//! This library provides tools for capturing keyboard and mouse interaction
//! timing for behavioral research, with strong privacy guarantees.
//!
//! # Privacy Guarantees
//!
//! - **No key content**: We never capture which keys are pressed, only timing
//! - **No coordinates**: We never capture cursor position, only movement magnitude
//! - **No raw storage**: Raw events are not stored beyond the current window
//! - **Transparency**: All collection is logged and auditable
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Synheart Sensor Agent                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐       │
//! │  │  Collector  │──▶│  Windowing  │──▶│  Features   │       │
//! │  │   (macOS)   │   │  (10s bins) │   │ (compute)   │       │
//! │  └─────────────┘   └─────────────┘   └─────────────┘       │
//! │         │                                    │              │
//! │         ▼                                    ▼              │
//! │  ┌─────────────┐                     ┌─────────────┐       │
//! │  │Transparency │                     │    HSI      │       │
//! │  │    Log      │                     │  Snapshot   │       │
//! │  └─────────────┘                     └─────────────┘       │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use synheart_sensor_agent::{collector, core, transparency};
//!
//! // Create a collector (requires Input Monitoring permission)
//! let config = collector::CollectorConfig::default();
//! let mut collector = collector::Collector::new(config);
//!
//! // Start collection
//! collector.start().expect("Failed to start collector");
//!
//! // Events can be received from collector.receiver()
//! ```

pub mod collector;
pub mod config;
pub mod core;
pub mod transparency;

#[cfg(feature = "flux")]
pub mod flux;

#[cfg(feature = "gateway")]
pub mod gateway;

// Re-export key types at crate root for convenience
pub use collector::{Collector, CollectorConfig, CollectorError, SensorEvent};
pub use config::{Config, SourceConfig};
pub use core::{compute_features, HsiBuilder, HsiSnapshot, WindowFeatures, WindowManager};
pub use transparency::{SharedTransparencyLog, TransparencyLog, TransparencyStats};

// Flux re-exports (when enabled)
#[cfg(feature = "flux")]
pub use flux::{EnrichedSnapshot, SensorFluxProcessor};

// Gateway re-exports (when enabled)
#[cfg(feature = "gateway")]
pub use gateway::{
    BlockingGatewayClient, GatewayClient, GatewayConfig, GatewayError, GatewayResponse, HsiState,
};

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Privacy declaration that can be displayed to users.
pub const PRIVACY_DECLARATION: &str = r#"
╔══════════════════════════════════════════════════════════════════╗
║           SYNHEART SENSOR AGENT - PRIVACY DECLARATION            ║
╠══════════════════════════════════════════════════════════════════╣
║                                                                  ║
║  This agent captures behavioral timing data for research.        ║
║                                                                  ║
║  ✓ WHAT WE CAPTURE:                                              ║
║    • When keys are pressed (timing only)                         ║
║    • How fast the mouse moves (speed only)                       ║
║    • When clicks and scrolls occur (timing only)                 ║
║                                                                  ║
║  ✗ WHAT WE NEVER CAPTURE:                                        ║
║    • Which keys you press (no passwords, messages, etc.)         ║
║    • Where your cursor is (no screen position tracking)          ║
║    • What applications you use                                   ║
║    • Any screen content                                          ║
║                                                                  ║
║  All data is processed locally. Raw events are discarded         ║
║  after feature extraction (every 10 seconds).                    ║
║                                                                  ║
║  You can view collection statistics anytime with:                ║
║    synheart-sensor status                                        ║
║                                                                  ║
╚══════════════════════════════════════════════════════════════════╝
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_declaration_contents() {
        assert!(PRIVACY_DECLARATION.contains("PRIVACY"));
        assert!(PRIVACY_DECLARATION.contains("NEVER CAPTURE"));
        assert!(PRIVACY_DECLARATION.contains("keys you press"));
    }
}
