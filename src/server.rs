//! HTTP server for receiving behavioral data from Chrome extension.
//!
//! This module provides an HTTP server that:
//! - Accepts raw behavioral data from Chrome extension via POST /ingest
//! - Processes it through synheart-flux's BehaviorProcessor
//! - Sends processed HSI to synheart-core-gateway
//!
//! # Architecture
//!
//! ```text
//! Chrome Extension ──→ POST /ingest ──→ sensor-agent ──→ gateway ──→ Syni Life
//!                                           ↓
//!                                    [Flux Processing]
//! ```

use crate::gateway::GatewayConfig;
use axum::{
    extract::State,
    http::{HeaderValue, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use crate::core::HsiSnapshot;
use crate::gateway::{
    BehavioralSession as GatewayBehavioralSession, SessionMeta, SessionPayload,
};
use synheart_flux::BehaviorProcessor;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Port to bind to (0 for random)
    pub port: u16,
    /// Gateway configuration for forwarding processed HSI
    pub gateway_config: GatewayConfig,
    /// State directory for baselines
    pub state_dir: PathBuf,
}

impl ServerConfig {
    /// Create a new server configuration
    pub fn new(port: u16, gateway_config: GatewayConfig, state_dir: PathBuf) -> Self {
        Self {
            port,
            gateway_config,
            state_dir,
        }
    }
}

/// Shared server state
pub struct ServerState {
    /// Flux behavior processor
    processor: RwLock<BehaviorProcessor>,
    /// Gateway configuration
    gateway_config: GatewayConfig,
    /// HTTP client for gateway
    http_client: reqwest::Client,
    /// State directory
    state_dir: PathBuf,
}

impl ServerState {
    /// Create new server state
    pub fn new(config: &ServerConfig) -> Self {
        let mut processor = BehaviorProcessor::new();

        // Load baselines if they exist
        let baseline_path = config.state_dir.join("state").join("behavior_baselines.json");
        if baseline_path.exists() {
            if let Ok(json) = std::fs::read_to_string(&baseline_path) {
                if let Err(e) = processor.load_baselines(&json) {
                    tracing::warn!("Failed to load baselines: {}", e);
                }
            }
        }

        Self {
            processor: RwLock::new(processor),
            gateway_config: config.gateway_config.clone(),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            state_dir: config.state_dir.clone(),
        }
    }

    /// Save baselines to disk
    async fn save_baselines(&self) {
        let baseline_dir = self.state_dir.join("state");
        let _ = std::fs::create_dir_all(&baseline_dir);

        let processor = self.processor.read().await;
        if let Ok(json) = processor.save_baselines() {
            let path = baseline_dir.join("behavior_baselines.json");
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!("Failed to save baselines: {}", e);
            }
        }
    }
}

/// Behavioral session data from Chrome extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralSession {
    /// The behavioral session to process
    pub session: serde_json::Value,
}

/// Response from ingest endpoint
#[derive(Debug, Clone, Serialize)]
pub struct IngestResponse {
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hsi_payload: Option<serde_json::Value>,
}

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Error response
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// GET /health
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// POST /ingest
///
/// Accepts raw behavioral data from Chrome extension, processes with flux,
/// and forwards to gateway.
async fn ingest(
    State(state): State<Arc<ServerState>>,
    Json(data): Json<BehavioralSession>,
) -> Result<Json<IngestResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Serialize session for flux processing
    let session_json = serde_json::to_string(&data.session).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid session data: {}", e),
                code: "INVALID_SESSION".to_string(),
            }),
        )
    })?;

    // Process through flux
    let hsi_json = {
        let mut processor = state.processor.write().await;
        processor.process(&session_json).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Flux processing failed: {}", e),
                    code: "FLUX_ERROR".to_string(),
                }),
            )
        })?
    };

    // Parse HSI payload (we forward as a snapshot to core-gateway)
    let hsi_snapshot: HsiSnapshot = serde_json::from_str(&hsi_json).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse HSI output: {}", e),
                code: "PARSE_ERROR".to_string(),
            }),
        )
    })?;

    // Extract session fields from the inbound payload for gateway session envelope.
    // (If the Chrome extension omits fields, fall back to safe defaults.)
    let session_obj = data.session.as_object();
    let get_str = |key: &str| -> Option<String> {
        session_obj
            .and_then(|m| m.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };

    let session_id = get_str("session_id").unwrap_or_else(|| "unknown-session".to_string());
    let device_id = get_str("device_id").unwrap_or_else(|| "unknown-device".to_string());
    let timezone = get_str("timezone").unwrap_or_else(|| "UTC".to_string());
    let start_time = get_str("start_time").unwrap_or_else(|| hsi_snapshot.observed_at_utc.clone());
    let end_time = get_str("end_time").unwrap_or_else(|| hsi_snapshot.computed_at_utc.clone());

    // Forward to core-gateway behavioral ingest endpoint.
    let gateway_url = state.gateway_config.ingest_url();
    let gateway_payload = GatewayBehavioralSession {
        session: SessionPayload {
            session_id,
            device_id,
            timezone,
            start_time,
            end_time,
            snapshots: vec![hsi_snapshot.clone()],
            meta: SessionMeta {
                source: "synheart-sensor-agent-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                snapshot_count: 1,
            },
        },
    };

    let response = state
        .http_client
        .post(&gateway_url)
        .header(
            "Authorization",
            format!("Bearer {}", state.gateway_config.token),
        )
        .header("Content-Type", "application/json")
        .json(&gateway_payload)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to forward to gateway: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: format!("Gateway forwarding failed: {}", e),
                    code: "GATEWAY_ERROR".to_string(),
                }),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        tracing::error!("Gateway returned error {}: {}", status, body);
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: format!("Gateway returned error: {}", body),
                code: "GATEWAY_ERROR".to_string(),
            }),
        ));
    }

    // Save baselines periodically
    state.save_baselines().await;

    Ok(Json(IngestResponse {
        status: "ok".to_string(),
        message: "Processed and forwarded to gateway".to_string(),
        hsi_payload: serde_json::to_value(&hsi_snapshot).ok(),
    }))
}

/// Run the HTTP server
pub async fn run(config: ServerConfig) -> anyhow::Result<(SocketAddr, tokio::sync::oneshot::Sender<()>)> {
    let state = Arc::new(ServerState::new(&config));

    let app = Router::new()
        .route("/health", get(health))
        .route("/ingest", post(ingest))
        .layer(
            CorsLayer::new()
                .allow_origin([
                    HeaderValue::from_static("http://localhost"),
                    HeaderValue::from_static("http://127.0.0.1"),
                    // Allow chrome-extension origins
                    HeaderValue::from_static("chrome-extension://"),
                ])
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    let listener = TcpListener::bind(addr).await?;
    let actual_addr = listener.local_addr()?;

    tracing::info!("Sensor agent server listening on http://{}", actual_addr);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
                tracing::info!("Server shutdown signal received");
            })
            .await
        {
            tracing::error!("Server error: {}", e);
        }
    });

    Ok((actual_addr, shutdown_tx))
}
