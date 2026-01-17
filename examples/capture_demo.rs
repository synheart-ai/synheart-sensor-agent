//! Demonstration of the Synheart Sensor Agent event capture.
//!
//! This example shows how to:
//! 1. Check for Input Monitoring permission
//! 2. Create and start a collector
//! 3. Receive and process events
//! 4. Compute features from event windows
//! 5. Generate HSI snapshots
//!
//! Run with: cargo run --example capture_demo
//!
//! Note: Requires Input Monitoring permission on macOS.
//! Grant permission in System Preferences > Security & Privacy > Privacy > Input Monitoring

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use synheart_sensor_agent::{
    collector::{check_permission, Collector, CollectorConfig, SensorEvent},
    core::{compute_features, HsiBuilder, WindowManager},
    transparency::TransparencyLog,
    PRIVACY_DECLARATION,
};

fn main() {
    println!("Synheart Sensor Agent - Capture Demo");
    println!("=====================================");
    println!();

    // Display privacy declaration
    println!("{PRIVACY_DECLARATION}");
    println!();

    // Check for permission
    print!("Checking Input Monitoring permission... ");
    if check_permission() {
        println!("OK ✓");
    } else {
        println!("FAILED ✗");
        println!();
        println!("Please grant Input Monitoring permission:");
        println!("1. Open System Preferences");
        println!("2. Go to Security & Privacy > Privacy > Input Monitoring");
        println!("3. Add this application");
        println!("4. Restart this demo");
        return;
    }
    println!();

    // Create components
    let config = CollectorConfig {
        capture_keyboard: true,
        capture_mouse: true,
    };

    let mut collector = Collector::new(config);
    let mut window_manager = WindowManager::new(10, 300); // 10s windows, 5min session gap
    let hsi_builder = HsiBuilder::new();
    let transparency_log = TransparencyLog::new();

    println!("Instance ID: {}", hsi_builder.instance_id());
    println!();
    println!("Starting capture for 30 seconds...");
    println!("Try typing and moving your mouse!");
    println!();

    // Start collection
    if let Err(e) = collector.start() {
        eprintln!("Error starting collector: {e}");
        return;
    }

    // Set up stop flag
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    // Run for 30 seconds
    let start = std::time::Instant::now();
    let receiver = collector.receiver().clone();
    let mut event_count = 0;

    while running.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(30) {
        // Receive events with timeout
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                event_count += 1;

                // Log event type
                match &event {
                    SensorEvent::Keyboard(e) => {
                        transparency_log.record_keyboard_event();
                        if event_count <= 10 || event_count % 50 == 0 {
                            println!(
                                "  Keyboard event: {} at {}",
                                if e.is_key_down { "down" } else { "up" },
                                e.timestamp.format("%H:%M:%S%.3f")
                            );
                        }
                    }
                    SensorEvent::Mouse(e) => {
                        transparency_log.record_mouse_event();
                        if event_count <= 10 || event_count % 100 == 0 {
                            println!(
                                "  Mouse event: {:?} at {}",
                                e.event_type,
                                e.timestamp.format("%H:%M:%S%.3f")
                            );
                        }
                    }
                }

                // Process in window manager
                window_manager.process_event(event);
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Check for window expiry
                window_manager.check_window_expiry();
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                break;
            }
        }

        // Process completed windows
        for window in window_manager.take_completed_windows() {
            transparency_log.record_window_completed();

            let features = compute_features(&window);
            let snapshot = hsi_builder.build(&window, &features);

            println!();
            println!("=== Window Completed ===");
            println!("  Duration: {:.1}s", window.duration_secs());
            println!("  Keyboard events: {}", window.keyboard_events.len());
            println!("  Mouse events: {}", window.mouse_events.len());
            println!();
            println!("  Keyboard Features:");
            println!(
                "    Typing rate: {:.2} keys/sec",
                features.keyboard.typing_rate
            );
            println!("    Pause count: {}", features.keyboard.pause_count);
            println!("    Burst index: {:.3}", features.keyboard.burst_index);
            println!();
            println!("  Mouse Features:");
            println!(
                "    Activity rate: {:.2} moves/sec",
                features.mouse.mouse_activity_rate
            );
            println!("    Mean velocity: {:.2}", features.mouse.mean_velocity);
            println!(
                "    Click rate: {:.2} clicks/sec",
                features.mouse.click_rate
            );
            println!();
            println!("  Behavioral Signals:");
            println!(
                "    Interaction rhythm: {:.3}",
                features.behavioral.interaction_rhythm
            );
            println!("    Friction: {:.3}", features.behavioral.friction);
            println!(
                "    Motor stability: {:.3}",
                features.behavioral.motor_stability
            );
            println!(
                "    Focus continuity: {:.3}",
                features.behavioral.focus_continuity_proxy
            );
            println!();

            // Show snippet of HSI JSON
            let json = serde_json::to_string_pretty(&snapshot).unwrap();
            println!("  HSI Snapshot (truncated):");
            for line in json.lines().take(20) {
                println!("    {line}");
            }
            println!("    ...");
            println!();
        }

        // Show progress
        if event_count > 0 && event_count % 200 == 0 {
            let elapsed = start.elapsed().as_secs();
            println!("  [{elapsed}/30s] Processed {event_count} events...");
        }
    }

    // Stop collection
    println!();
    println!("Stopping capture...");
    collector.stop();

    // Flush remaining window
    window_manager.flush();
    for window in window_manager.take_completed_windows() {
        println!("Final window: {} events", window.event_count());
        transparency_log.record_window_completed();
    }

    // Final statistics
    println!();
    println!("{}", transparency_log.summary());
    println!();
    println!("Demo complete!");
}
