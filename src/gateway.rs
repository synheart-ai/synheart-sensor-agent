//! Gateway client for syncing HSI snapshots to synheart-core-gateway.
//!
//! This module provides integration with the local synheart-core-gateway
//! for real-time HSI processing via synheart-flux.

use crate::core::HsiSnapshot;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Gateway configuration.
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// Gateway host (default: 127.0.0.1)
    pub host: String,
    /// Gateway port
    pub port: u16,
    /// Bearer authentication token
    pub token: String,
}

impl GatewayConfig {
    /// Create a new gateway configuration.
    pub fn new(host: impl Into<String>, port: u16, token: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            token: token.into(),
        }
    }

    /// Load configuration from SyniLife runtime directory.
    ///
    /// Reads port from `~/Library/Application Support/SyniLife/runtime/gateway.port`
    /// and token from `~/Library/Application Support/SyniLife/runtime/gateway.token`
    pub fn from_runtime_dir() -> Result<Self, GatewayError> {
        let state_dir = Self::default_state_dir()?;
        let runtime_dir = state_dir.join("runtime");

        let port_path = runtime_dir.join("gateway.port");
        let token_path = runtime_dir.join("gateway.token");

        let port_str = std::fs::read_to_string(&port_path).map_err(|e| {
            GatewayError::Config(format!(
                "Failed to read gateway port from {port_path:?}: {e}"
            ))
        })?;

        let port: u16 = port_str.trim().parse().map_err(|e| {
            GatewayError::Config(format!("Invalid port number '{}': {}", port_str.trim(), e))
        })?;

        let token = std::fs::read_to_string(&token_path)
            .map_err(|e| {
                GatewayError::Config(format!(
                    "Failed to read gateway token from {token_path:?}: {e}"
                ))
            })?
            .trim()
            .to_string();

        Ok(Self {
            host: "127.0.0.1".to_string(),
            port,
            token,
        })
    }

    /// Get the default SyniLife state directory.
    fn default_state_dir() -> Result<PathBuf, GatewayError> {
        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                return Ok(home.join("Library/Application Support/SyniLife"));
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(data_dir) = dirs::data_dir() {
                return Ok(data_dir.join("SyniLife"));
            }
        }

        Err(GatewayError::Config(
            "Could not determine SyniLife state directory".to_string(),
        ))
    }

    /// Get the full gateway URL.
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    /// Get the ingest endpoint URL (pure relay).
    pub fn ingest_url(&self) -> String {
        format!("{}/v1/ingest", self.url())
    }

    /// Get the health check endpoint URL.
    pub fn health_url(&self) -> String {
        format!("{}/health", self.url())
    }
}

/// Gateway client error types.
#[derive(Debug)]
pub enum GatewayError {
    /// Configuration error
    Config(String),
    /// Network/HTTP error
    Network(String),
    /// Server returned an error response
    Server { status: u16, message: String },
    /// JSON serialization error
    Serialization(String),
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GatewayError::Config(msg) => write!(f, "Gateway config error: {msg}"),
            GatewayError::Network(msg) => write!(f, "Gateway network error: {msg}"),
            GatewayError::Server { status, message } => {
                write!(f, "Gateway server error ({status}): {message}")
            }
            GatewayError::Serialization(msg) => write!(f, "Gateway serialization error: {msg}"),
        }
    }
}

impl std::error::Error for GatewayError {}

/// Session payload for the behavioral ingest endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct BehavioralSession {
    /// Session containing HSI snapshots
    pub session: SessionPayload,
}

/// Session payload structure matching core-gateway expectations.
#[derive(Debug, Clone, Serialize)]
pub struct SessionPayload {
    /// Session identifier
    pub session_id: String,
    /// Device identifier
    pub device_id: String,
    /// Timezone
    pub timezone: String,
    /// Session start time (RFC3339)
    pub start_time: String,
    /// Session end time (RFC3339)
    pub end_time: String,
    /// HSI snapshots as events
    pub snapshots: Vec<HsiSnapshot>,
    /// Metadata
    pub meta: SessionMeta,
}

/// Session metadata.
#[derive(Debug, Clone, Serialize)]
pub struct SessionMeta {
    /// Source identifier
    pub source: String,
    /// Version
    pub version: String,
    /// Snapshot count
    pub snapshot_count: usize,
}

/// Gateway response from the behavioral ingest endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct GatewayResponse {
    /// Timestamp of processing
    pub timestamp: String,
    /// Flux payload (if processed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flux_payload: Option<serde_json::Value>,
    /// HSI state summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<HsiState>,
}

/// HSI state summary from gateway.
#[derive(Debug, Clone, Deserialize)]
pub struct HsiState {
    /// Focus level
    pub focus: Option<String>,
    /// Load level
    pub load: Option<String>,
    /// Recovery level
    pub recovery: Option<String>,
}

impl std::fmt::Display for HsiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let focus = self.focus.as_deref().unwrap_or("unknown");
        let load = self.load.as_deref().unwrap_or("unknown");
        let recovery = self.recovery.as_deref().unwrap_or("unknown");
        write!(f, "focus: {focus}, load: {load}, recovery: {recovery}")
    }
}

/// Gateway client for syncing with synheart-core-gateway.
#[cfg(feature = "gateway")]
pub struct GatewayClient {
    config: GatewayConfig,
    client: reqwest::Client,
    device_id: String,
}

#[cfg(feature = "gateway")]
impl GatewayClient {
    /// Create a new gateway client.
    pub fn new(config: GatewayConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        // Generate device ID from hostname + instance
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let device_id = format!(
            "sensor-{}-{}",
            hostname,
            &uuid::Uuid::new_v4().to_string()[..8]
        );

        Self {
            config,
            client,
            device_id,
        }
    }

    /// Create a new gateway client from runtime directory configuration.
    pub fn from_runtime() -> Result<Self, GatewayError> {
        let config = GatewayConfig::from_runtime_dir()?;
        Ok(Self::new(config))
    }

    /// Test connection to the gateway.
    pub async fn test_connection(&self) -> Result<bool, GatewayError> {
        let response = self
            .client
            .get(self.config.health_url())
            .send()
            .await
            .map_err(|e| GatewayError::Network(e.to_string()))?;

        Ok(response.status().is_success())
    }

    /// Sync HSI snapshots to the gateway.
    pub async fn sync_snapshots(
        &self,
        snapshots: &[HsiSnapshot],
        session_id: &str,
    ) -> Result<GatewayResponse, GatewayError> {
        if snapshots.is_empty() {
            return Err(GatewayError::Config("No snapshots to sync".to_string()));
        }

        // Build session payload
        let start_time = snapshots
            .first()
            .map(|s| s.observed_at_utc.clone())
            .unwrap_or_default();
        let end_time = snapshots
            .last()
            .map(|s| s.computed_at_utc.clone())
            .unwrap_or_default();

        let timezone = chrono_tz::Tz::UTC.to_string();

        let session = BehavioralSession {
            session: SessionPayload {
                session_id: session_id.to_string(),
                device_id: self.device_id.clone(),
                timezone,
                start_time,
                end_time,
                snapshots: snapshots.to_vec(),
                meta: SessionMeta {
                    source: "synheart-sensor-agent".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    snapshot_count: snapshots.len(),
                },
            },
        };

        let response = self
            .client
            .post(self.config.ingest_url())
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Content-Type", "application/json")
            .json(&session)
            .send()
            .await
            .map_err(|e| GatewayError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(GatewayError::Server {
                status: status.as_u16(),
                message,
            });
        }

        let gateway_response: GatewayResponse = response
            .json()
            .await
            .map_err(|e| GatewayError::Serialization(e.to_string()))?;

        Ok(gateway_response)
    }

    /// Get the device ID.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}

/// Blocking gateway client for use in synchronous contexts.
#[cfg(feature = "gateway")]
pub struct BlockingGatewayClient {
    inner: GatewayClient,
    runtime: tokio::runtime::Runtime,
}

#[cfg(feature = "gateway")]
impl BlockingGatewayClient {
    /// Create a new blocking gateway client.
    pub fn new(config: GatewayConfig) -> Result<Self, GatewayError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| GatewayError::Config(format!("Failed to create runtime: {e}")))?;

        Ok(Self {
            inner: GatewayClient::new(config),
            runtime,
        })
    }

    /// Create a new blocking gateway client from runtime directory configuration.
    pub fn from_runtime() -> Result<Self, GatewayError> {
        let config = GatewayConfig::from_runtime_dir()?;
        Self::new(config)
    }

    /// Test connection to the gateway.
    pub fn test_connection(&self) -> Result<bool, GatewayError> {
        self.runtime.block_on(self.inner.test_connection())
    }

    /// Sync HSI snapshots to the gateway.
    pub fn sync_snapshots(
        &self,
        snapshots: &[HsiSnapshot],
        session_id: &str,
    ) -> Result<GatewayResponse, GatewayError> {
        self.runtime
            .block_on(self.inner.sync_snapshots(snapshots, session_id))
    }

    /// Get the device ID.
    pub fn device_id(&self) -> &str {
        self.inner.device_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_config_url() {
        let config = GatewayConfig::new("127.0.0.1", 8080, "test-token");
        assert_eq!(config.url(), "http://127.0.0.1:8080");
        assert_eq!(config.ingest_url(), "http://127.0.0.1:8080/v1/ingest");
        assert_eq!(config.health_url(), "http://127.0.0.1:8080/health");
    }

    #[test]
    fn test_hsi_state_display() {
        let state = HsiState {
            focus: Some("high".to_string()),
            load: Some("moderate".to_string()),
            recovery: None,
        };
        let display = format!("{state}");
        assert!(display.contains("high"));
        assert!(display.contains("moderate"));
    }
}
