//! Integration tests for the sensor-agent HTTP server

#[cfg(feature = "server")]
mod server_tests {
    use synheart_sensor_agent::gateway::GatewayConfig;
    use synheart_sensor_agent::server::{run, ServerConfig};
    use std::path::PathBuf;
    use std::time::Duration;

    fn test_state_dir() -> PathBuf {
        std::env::temp_dir().join("synheart-server-test")
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        // Create server config with random port
        let gateway_config = GatewayConfig::new("127.0.0.1", 9999, "test-token".to_string());
        let config = ServerConfig::new(0, gateway_config, test_state_dir());

        // Start server
        let (addr, shutdown_tx) = run(config).await.expect("Failed to start server");

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Test health endpoint
        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .expect("Failed to send request");

        assert!(response.status().is_success());

        let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
        assert_eq!(body["status"], "ok");
        assert!(body["version"].as_str().is_some());

        // Shutdown server
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ingest_endpoint_structure() {
        // Create server config with random port
        let gateway_config = GatewayConfig::new("127.0.0.1", 9999, "test-token".to_string());
        let config = ServerConfig::new(0, gateway_config, test_state_dir());

        // Start server
        let (addr, shutdown_tx) = run(config).await.expect("Failed to start server");

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Test ingest endpoint with sample behavioral session
        let sample_session = serde_json::json!({
            "session": {
                "session_id": "TEST-123",
                "device_id": "chrome-test",
                "timezone": "America/Los_Angeles",
                "start_time": "2024-01-22T10:00:00Z",
                "end_time": "2024-01-22T10:00:10Z",
                "events": [
                    {
                        "timestamp": "2024-01-22T10:00:01Z",
                        "event_type": "typing",
                        "typing": {
                            "key_category": "alphanumeric",
                            "hold_ms": 100,
                            "flight_ms": 50,
                            "is_backspace_burst": false,
                            "burst_count": 0
                        }
                    },
                    {
                        "timestamp": "2024-01-22T10:00:02Z",
                        "event_type": "scroll",
                        "scroll": {
                            "velocity": 150.0,
                            "direction": "down",
                            "direction_reversal": false
                        }
                    }
                ],
                "meta": {
                    "source": "synheart-behavior-chrome",
                    "version": "2.0",
                    "event_count": 2
                }
            }
        });

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{}/ingest", addr))
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer test-token")
            .json(&sample_session)
            .send()
            .await
            .expect("Failed to send request");

        // The request will fail at gateway forwarding since gateway isn't running,
        // but we can verify the server accepted and tried to process the data
        let status = response.status();
        let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

        // Either success (if gateway was running) or gateway error (expected in tests)
        assert!(
            status.is_success() || status == reqwest::StatusCode::BAD_GATEWAY,
            "Unexpected status: {} - body: {:?}",
            status,
            body
        );

        // If it's a gateway error, that means our server processed the request correctly
        // but couldn't forward to gateway (which is expected in tests)
        if status == reqwest::StatusCode::BAD_GATEWAY {
            assert!(body["code"].as_str().unwrap_or("").contains("GATEWAY"));
        }

        // Shutdown server
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_cors_headers() {
        // Create server config with random port
        let gateway_config = GatewayConfig::new("127.0.0.1", 9999, "test-token".to_string());
        let config = ServerConfig::new(0, gateway_config, test_state_dir());

        // Start server
        let (addr, shutdown_tx) = run(config).await.expect("Failed to start server");

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Send OPTIONS request to check CORS
        let client = reqwest::Client::new();
        let response = client
            .request(reqwest::Method::OPTIONS, format!("http://{}/ingest", addr))
            .header("Origin", "http://localhost")
            .header("Access-Control-Request-Method", "POST")
            .send()
            .await
            .expect("Failed to send request");

        // CORS preflight should succeed
        assert!(
            response.status().is_success() || response.status() == reqwest::StatusCode::NO_CONTENT,
            "CORS preflight failed: {}",
            response.status()
        );

        // Shutdown server
        let _ = shutdown_tx.send(());
    }
}
