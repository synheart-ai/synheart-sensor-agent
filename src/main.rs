//! Synheart Sensor Agent CLI
//!
//! Privacy-first behavioral sensor for research.

use chrono::Utc;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use synheart_sensor_agent::{
    collector::{check_permission, Collector, CollectorConfig, SensorEvent},
    config::{Config, SourceConfig},
    core::{compute_features, HsiBuilder, HsiSnapshot, WindowManager},
    transparency::create_shared_log_with_persistence,
    PRIVACY_DECLARATION, VERSION,
};

#[cfg(feature = "gateway")]
use synheart_sensor_agent::{BlockingGatewayClient, GatewayConfig};

#[derive(Parser)]
#[command(name = "synheart-sensor")]
#[command(author = "Synheart")]
#[command(version = VERSION)]
#[command(about = "Privacy-first behavioral sensor for research", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start capturing behavioral data
    Start {
        /// Input sources to capture (keyboard, mouse, or all)
        #[arg(long, default_value = "all")]
        sources: String,

        /// Run in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,

        /// Enable synheart-flux baseline tracking (requires flux feature)
        #[arg(long)]
        flux: bool,

        /// Baseline window size (number of sessions for rolling baseline)
        #[arg(long, default_value = "20")]
        baseline_window: usize,

        /// Enable gateway sync (requires gateway feature)
        #[arg(long)]
        gateway: bool,

        /// Gateway port (auto-detected from runtime dir if not specified)
        #[arg(long)]
        gateway_port: Option<u16>,

        /// Gateway token (auto-detected from runtime dir if not specified)
        #[arg(long)]
        gateway_token: Option<String>,

        /// Sync interval in seconds (how often to sync to gateway)
        #[arg(long, default_value = "10")]
        sync_interval: u64,
    },

    /// Pause data collection
    Pause,

    /// Resume data collection
    Resume,

    /// Show current collection status
    Status,

    /// Display privacy declaration
    Privacy,

    /// Export collected HSI snapshots
    Export {
        /// Output directory for snapshots
        #[arg(long, short)]
        output: Option<PathBuf>,

        /// Export format (json or jsonl)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Show configuration
    Config,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            sources,
            foreground,
            flux,
            baseline_window,
            gateway,
            gateway_port,
            gateway_token,
            sync_interval,
        } => {
            cmd_start(
                &sources,
                foreground,
                flux,
                baseline_window,
                gateway,
                gateway_port,
                gateway_token,
                sync_interval,
            );
        }
        Commands::Pause => {
            cmd_pause();
        }
        Commands::Resume => {
            cmd_resume();
        }
        Commands::Status => {
            cmd_status();
        }
        Commands::Privacy => {
            cmd_privacy();
        }
        Commands::Export { output, format } => {
            cmd_export(output, &format);
        }
        Commands::Config => {
            cmd_config();
        }
    }
}

#[allow(unused_variables)]
fn cmd_start(
    sources: &str,
    _foreground: bool,
    enable_flux: bool,
    baseline_window: usize,
    enable_gateway: bool,
    gateway_port: Option<u16>,
    gateway_token: Option<String>,
    sync_interval: u64,
) {
    println!("Synheart Sensor Agent v{VERSION}");
    println!();

    // Check for Input Monitoring permission
    if !check_permission() {
        eprintln!("Error: Input Monitoring permission not granted.");
        eprintln!();
        eprintln!("To grant permission:");
        eprintln!("1. Open System Preferences > Security & Privacy > Privacy");
        eprintln!("2. Select 'Input Monitoring' in the left sidebar");
        eprintln!("3. Add this application to the allowed list");
        eprintln!("4. Restart the application");
        std::process::exit(1);
    }

    // Parse source configuration
    let source_config = SourceConfig::from_csv(sources);
    if !source_config.any_enabled() {
        eprintln!("Error: At least one source must be enabled (keyboard or mouse)");
        std::process::exit(1);
    }

    // Load or create configuration
    let config = Config::load().unwrap_or_default();
    if let Err(e) = config.ensure_directories() {
        eprintln!("Warning: Could not create directories: {e}");
    }

    println!("Starting collection...");
    println!(
        "  Keyboard: {}",
        if source_config.keyboard {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  Mouse: {}",
        if source_config.mouse {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("  Window duration: {}s", config.window_duration.as_secs());

    // Show flux status
    #[cfg(feature = "flux")]
    if enable_flux {
        println!("  Flux baseline tracking: enabled (window: {} sessions)", baseline_window);
    } else {
        println!("  Flux baseline tracking: disabled");
    }
    #[cfg(not(feature = "flux"))]
    if enable_flux {
        eprintln!("Warning: --flux flag ignored (flux feature not enabled at compile time)");
    }

    // Show gateway status
    #[cfg(feature = "gateway")]
    let gateway_client = if enable_gateway {
        match create_gateway_client(gateway_port, gateway_token) {
            Ok(client) => {
                println!("  Gateway sync: enabled (interval: {}s)", sync_interval);
                println!("  Device ID: {}", client.device_id());

                // Test connection
                match client.test_connection() {
                    Ok(true) => println!("  Gateway connection: OK"),
                    Ok(false) => {
                        eprintln!("Warning: Gateway health check failed");
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not connect to gateway: {e}");
                    }
                }
                Some(client)
            }
            Err(e) => {
                eprintln!("Warning: Gateway initialization failed: {e}");
                eprintln!("Continuing without gateway sync.");
                None
            }
        }
    } else {
        println!("  Gateway sync: disabled");
        None
    };

    #[cfg(not(feature = "gateway"))]
    if enable_gateway {
        eprintln!("Warning: --gateway flag ignored (gateway feature not enabled at compile time)");
    }

    println!();
    println!("Press Ctrl+C to stop");
    println!();

    // Set up transparency log
    let transparency_log =
        create_shared_log_with_persistence(config.data_path.join("transparency.json"));

    // Create collector
    let collector_config = CollectorConfig {
        capture_keyboard: source_config.keyboard,
        capture_mouse: source_config.mouse,
    };
    let mut collector = Collector::new(collector_config);

    // Create window manager
    let mut window_manager = WindowManager::new(
        config.window_duration.as_secs(),
        config.session_gap_threshold_secs,
    );

    // Create HSI builder
    let hsi_builder = HsiBuilder::new();
    println!("Instance ID: {}", hsi_builder.instance_id());

    // Storage for completed snapshots
    let mut snapshots: Vec<HsiSnapshot> = Vec::new();

    // Initialize flux processor if enabled
    #[cfg(feature = "flux")]
    let mut flux_processor = if enable_flux {
        let mut processor = synheart_sensor_agent::flux::SensorFluxProcessor::new(baseline_window);

        // Try to load existing baselines
        let baselines_path = config.data_path.join("flux_baselines.json");
        if baselines_path.exists() {
            if let Ok(baselines_json) = std::fs::read_to_string(&baselines_path) {
                match processor.load_baselines(&baselines_json) {
                    Ok(_) => println!("Loaded existing baselines from {:?}", baselines_path),
                    Err(e) => eprintln!("Warning: Could not load baselines: {e}"),
                }
            }
        }

        Some(processor)
    } else {
        None
    };

    // Storage for enriched snapshots (when flux is enabled)
    #[cfg(feature = "flux")]
    let mut enriched_snapshots: Vec<synheart_sensor_agent::flux::EnrichedSnapshot> = Vec::new();

    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc_handler(r);

    // Support pause/resume from another process by polling the config file.
    // If paused at startup, wait until resumed before starting the collector.
    let mut paused = config.paused;
    let mut last_config_check = std::time::Instant::now();

    if paused {
        println!("Collection is currently paused.");
        println!("Run `synheart-sensor resume` to start collecting.");
        println!();
    } else if let Err(e) = collector.start() {
        eprintln!("Error starting collector: {e}");
        std::process::exit(1);
    }

    // Gateway sync state
    #[cfg(feature = "gateway")]
    let mut pending_sync_snapshots: Vec<HsiSnapshot> = Vec::new();
    #[cfg(feature = "gateway")]
    let mut last_gateway_sync = std::time::Instant::now();
    #[cfg(feature = "gateway")]
    let session_id = format!("SESS-{}", Utc::now().timestamp_millis());

    // Main event loop
    let receiver = collector.receiver().clone();
    let mut last_window_check = std::time::Instant::now();

    while running.load(Ordering::SeqCst) {
        // Periodically reload config so `synheart-sensor pause/resume` can control a running agent.
        if last_config_check.elapsed() >= Duration::from_secs(1) {
            if let Ok(cfg) = Config::load() {
                if cfg.paused != paused {
                    paused = cfg.paused;

                    if paused {
                        println!();
                        println!("Pausing collection...");
                        collector.stop();

                        // Flush any in-progress window and drop partial data.
                        window_manager.flush();
                        let _ = window_manager.take_completed_windows();

                        // Drain any queued events.
                        while receiver.try_recv().is_ok() {}
                    } else {
                        println!();
                        println!("Resuming collection...");
                        if let Err(e) = collector.start() {
                            eprintln!("Error resuming collector: {e}");
                            std::process::exit(1);
                        }
                    }
                }
            }
            last_config_check = std::time::Instant::now();
        }

        if paused {
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        // Process events with timeout
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                // Update transparency log
                match &event {
                    SensorEvent::Keyboard(_) => transparency_log.record_keyboard_event(),
                    SensorEvent::Mouse(_) => transparency_log.record_mouse_event(),
                }

                // Add to window
                window_manager.process_event(event);
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Check for window expiry periodically
                if last_window_check.elapsed() >= Duration::from_secs(1) {
                    window_manager.check_window_expiry();
                    last_window_check = std::time::Instant::now();
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                eprintln!("Collector disconnected unexpectedly");
                break;
            }
        }

        // Process completed windows
        for window in window_manager.take_completed_windows() {
            let features = compute_features(&window);
            let snapshot = hsi_builder.build(&window, &features);

            transparency_log.record_window_completed();

            // Process with flux if enabled
            #[cfg(feature = "flux")]
            if let Some(ref mut processor) = flux_processor {
                match processor.process_window(&window, &features, snapshot.clone()) {
                    Ok(enriched) => {
                        let baseline_info = if let Some(ref baseline) = enriched.baseline {
                            format!(
                                " | baseline: {} sessions, dev: {:.1}%",
                                baseline.sessions_in_baseline,
                                baseline.distraction_deviation_pct.unwrap_or(0.0)
                            )
                        } else {
                            String::new()
                        };

                        let flux_info = if let Some(ref flux) = enriched.flux_behavior {
                            format!(
                                " | distraction: {:.2}, focus: {:.2}",
                                flux.distraction_score, flux.focus_hint
                            )
                        } else {
                            String::new()
                        };

                        println!(
                            "[{}] Window completed: {} keyboard, {} mouse events{}{}",
                            window.end.format("%H:%M:%S"),
                            window.keyboard_events.len(),
                            window.mouse_events.len(),
                            flux_info,
                            baseline_info
                        );
                        enriched_snapshots.push(enriched);
                    }
                    Err(e) => {
                        eprintln!("Warning: Flux processing failed: {e}");
                        println!(
                            "[{}] Window completed: {} keyboard, {} mouse events",
                            window.end.format("%H:%M:%S"),
                            window.keyboard_events.len(),
                            window.mouse_events.len()
                        );
                    }
                }
            } else {
                println!(
                    "[{}] Window completed: {} keyboard, {} mouse events",
                    window.end.format("%H:%M:%S"),
                    window.keyboard_events.len(),
                    window.mouse_events.len()
                );
            }

            #[cfg(not(feature = "flux"))]
            println!(
                "[{}] Window completed: {} keyboard, {} mouse events",
                window.end.format("%H:%M:%S"),
                window.keyboard_events.len(),
                window.mouse_events.len()
            );

            snapshots.push(snapshot.clone());

            // Add to gateway sync buffer
            #[cfg(feature = "gateway")]
            if gateway_client.is_some() {
                pending_sync_snapshots.push(snapshot);
            }
        }

        // Sync to gateway if enabled and interval has passed
        #[cfg(feature = "gateway")]
        if let Some(ref client) = gateway_client {
            if last_gateway_sync.elapsed() >= Duration::from_secs(sync_interval)
                && !pending_sync_snapshots.is_empty()
            {
                match client.sync_snapshots(&pending_sync_snapshots, &session_id) {
                    Ok(response) => {
                        if let Some(state) = response.state {
                            println!(
                                "[Gateway] Synced {} snapshots | HSI: {}",
                                pending_sync_snapshots.len(),
                                state
                            );
                        } else {
                            println!(
                                "[Gateway] Synced {} snapshots",
                                pending_sync_snapshots.len()
                            );
                        }
                        pending_sync_snapshots.clear();
                    }
                    Err(e) => {
                        eprintln!("[Gateway] Sync failed: {e}");
                        // Keep snapshots for retry
                    }
                }
                last_gateway_sync = std::time::Instant::now();
            }
        }
    }

    // Final gateway sync before exit
    #[cfg(feature = "gateway")]
    if let Some(ref client) = gateway_client {
        if !pending_sync_snapshots.is_empty() {
            println!("Syncing remaining {} snapshots to gateway...", pending_sync_snapshots.len());
            match client.sync_snapshots(&pending_sync_snapshots, &session_id) {
                Ok(response) => {
                    if let Some(state) = response.state {
                        println!("[Gateway] Final sync complete | HSI: {}", state);
                    } else {
                        println!("[Gateway] Final sync complete");
                    }
                }
                Err(e) => {
                    eprintln!("[Gateway] Final sync failed: {e}");
                }
            }
        }
    }

    // Stop collection
    println!();
    println!("Stopping collection...");
    collector.stop();

    // Flush remaining window
    window_manager.flush();
    for window in window_manager.take_completed_windows() {
        let features = compute_features(&window);
        let snapshot = hsi_builder.build(&window, &features);
        transparency_log.record_window_completed();
        snapshots.push(snapshot);
    }

    // Save transparency log
    if let Err(e) = transparency_log.save() {
        eprintln!("Warning: Could not save transparency log: {e}");
    }

    // Export snapshots
    if !snapshots.is_empty() {
        let export_path = config.export_path.join(format!(
            "session_{}.json",
            Utc::now().format("%Y%m%d_%H%M%S")
        ));

        if let Some(parent) = export_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match serde_json::to_string_pretty(&snapshots) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&export_path, json) {
                    eprintln!("Error writing snapshots: {e}");
                } else {
                    println!(
                        "Exported {} snapshots to {:?}",
                        snapshots.len(),
                        export_path
                    );
                    for _ in &snapshots {
                        transparency_log.record_snapshot_exported();
                    }
                }
            }
            Err(e) => {
                eprintln!("Error serializing snapshots: {e}");
            }
        }
    }

    // Export enriched snapshots if flux was enabled
    #[cfg(feature = "flux")]
    if !enriched_snapshots.is_empty() {
        let enriched_path = config.export_path.join(format!(
            "session_{}_enriched.json",
            Utc::now().format("%Y%m%d_%H%M%S")
        ));

        if let Some(parent) = enriched_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match serde_json::to_string_pretty(&enriched_snapshots) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&enriched_path, json) {
                    eprintln!("Error writing enriched snapshots: {e}");
                } else {
                    println!(
                        "Exported {} enriched snapshots to {:?}",
                        enriched_snapshots.len(),
                        enriched_path
                    );
                }
            }
            Err(e) => {
                eprintln!("Error serializing enriched snapshots: {e}");
            }
        }

        // Save baselines for next session
        if let Some(ref processor) = flux_processor {
            let baselines_path = config.data_path.join("flux_baselines.json");
            match processor.save_baselines() {
                Ok(baselines_json) => {
                    if let Err(e) = std::fs::write(&baselines_path, baselines_json) {
                        eprintln!("Error saving baselines: {e}");
                    } else {
                        println!("Saved baselines to {:?}", baselines_path);
                    }
                }
                Err(e) => {
                    eprintln!("Error serializing baselines: {e}");
                }
            }
        }
    }

    // Final stats
    println!();
    println!("{}", transparency_log.summary());
}

fn cmd_pause() {
    let mut config = Config::load().unwrap_or_default();
    config.paused = true;
    if let Err(e) = config.save() {
        eprintln!("Error saving config: {e}");
        std::process::exit(1);
    }
    println!("Collection paused. Use 'synheart-sensor resume' to continue.");
}

fn cmd_resume() {
    let mut config = Config::load().unwrap_or_default();
    config.paused = false;
    if let Err(e) = config.save() {
        eprintln!("Error saving config: {e}");
        std::process::exit(1);
    }
    println!("Collection resumed.");
}

fn cmd_status() {
    let config = Config::load().unwrap_or_default();

    println!("Synheart Sensor Agent Status");
    println!("============================");
    println!();

    // Check permission
    let has_permission = check_permission();
    println!(
        "Input Monitoring Permission: {}",
        if has_permission {
            "Granted ✓"
        } else {
            "Not Granted ✗"
        }
    );
    println!();

    // Show config
    println!("Configuration:");
    println!(
        "  Keyboard capture: {}",
        if config.sources.keyboard {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  Mouse capture: {}",
        if config.sources.mouse {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("  Window duration: {}s", config.window_duration.as_secs());
    println!("  Paused: {}", config.paused);
    println!();

    // Load and show transparency stats if available
    let stats_path = config.data_path.join("transparency.json");
    if stats_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&stats_path) {
            if let Ok(stats) = serde_json::from_str::<serde_json::Value>(&content) {
                println!("Cumulative Statistics:");
                if let Some(kb) = stats.get("keyboard_events") {
                    println!("  Keyboard events: {kb}");
                }
                if let Some(mouse) = stats.get("mouse_events") {
                    println!("  Mouse events: {mouse}");
                }
                if let Some(windows) = stats.get("windows_completed") {
                    println!("  Windows completed: {windows}");
                }
                if let Some(snapshots) = stats.get("snapshots_exported") {
                    println!("  Snapshots exported: {snapshots}");
                }
            }
        }
    } else {
        println!("No previous session data found.");
    }
}

fn cmd_privacy() {
    println!("{PRIVACY_DECLARATION}");
}

fn cmd_export(output: Option<PathBuf>, format: &str) {
    let config = Config::load().unwrap_or_default();
    let export_dir = output.unwrap_or(config.export_path.clone());

    // Find all session files
    let session_files: Vec<PathBuf> = std::fs::read_dir(&export_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|e| e == "json").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();

    if session_files.is_empty() {
        println!("No session data found in {export_dir:?}");
        println!("Run 'synheart-sensor start' to begin collecting data.");
        return;
    }

    println!(
        "Found {} session file(s) in {:?}",
        session_files.len(),
        export_dir
    );

    // Combine all snapshots
    let mut all_snapshots: Vec<HsiSnapshot> = Vec::new();
    for file in &session_files {
        if let Ok(content) = std::fs::read_to_string(file) {
            if let Ok(snapshots) = serde_json::from_str::<Vec<HsiSnapshot>>(&content) {
                all_snapshots.extend(snapshots);
            }
        }
    }

    println!("Total snapshots: {}", all_snapshots.len());

    // Export based on format
    let output_path = export_dir.join(format!(
        "export_{}.{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        if format == "jsonl" { "jsonl" } else { "json" }
    ));

    let result = if format == "jsonl" {
        // JSON Lines format
        let lines: Vec<String> = all_snapshots
            .iter()
            .filter_map(|s| serde_json::to_string(s).ok())
            .collect();
        std::fs::write(&output_path, lines.join("\n"))
    } else {
        // Pretty JSON format
        match serde_json::to_string_pretty(&all_snapshots) {
            Ok(json) => std::fs::write(&output_path, json),
            Err(e) => {
                eprintln!("Error serializing: {e}");
                return;
            }
        }
    };

    match result {
        Ok(_) => println!("Exported to {output_path:?}"),
        Err(e) => eprintln!("Error writing export: {e}"),
    }
}

fn cmd_config() {
    let config = Config::load().unwrap_or_default();

    println!("Configuration");
    println!("=============");
    println!();
    println!("Config file: {:?}", Config::config_path());
    println!();
    println!(
        "{}",
        serde_json::to_string_pretty(&config).unwrap_or_else(|_| "Error".to_string())
    );
}

/// Set up Ctrl+C handler.
fn ctrlc_handler(running: Arc<AtomicBool>) {
    ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");
}

/// Create gateway client from CLI args or runtime directory.
#[cfg(feature = "gateway")]
fn create_gateway_client(
    port: Option<u16>,
    token: Option<String>,
) -> Result<BlockingGatewayClient, synheart_sensor_agent::GatewayError> {
    // If both port and token are provided, use them directly
    if let (Some(p), Some(t)) = (port, token.clone()) {
        let config = GatewayConfig::new("127.0.0.1", p, t);
        return BlockingGatewayClient::new(config);
    }

    // Try to load from runtime directory
    match BlockingGatewayClient::from_runtime() {
        Ok(client) => Ok(client),
        Err(e) => {
            // If partial args provided, try to fill in the gaps
            if port.is_some() || token.is_some() {
                eprintln!("Warning: Partial gateway config provided, trying runtime directory...");
            }
            Err(e)
        }
    }
}
