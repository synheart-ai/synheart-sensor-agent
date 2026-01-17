# Synheart Sensor Agent

[![CI](https://github.com/synheart-ai/synheart-sensor-agent/actions/workflows/ci.yml/badge.svg)](https://github.com/synheart-ai/synheart-sensor-agent/.github/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

A privacy-first PC background sensor that captures keyboard and mouse interaction timing (not content) for behavioral research.

## Overview

Synheart Sensor Agent collects behavioral timing data from keyboard and mouse interactions while maintaining strict privacy guarantees. It extracts features like typing rhythm, mouse movement patterns, and interaction continuity - all without ever capturing what you type or where you click.

### Key Features

- **Privacy by Design**: Never captures key content, passwords, or screen coordinates
- **Behavioral Features**: Extracts 16+ behavioral signals from interaction patterns
- **HSI Format**: Exports data in the Human Sensor Interface (HSI) JSON format
- **Transparency**: Full visibility into what data is collected via `status` command
- **macOS Support**: Native integration using Core Graphics event taps

## Privacy Guarantees

```
╔══════════════════════════════════════════════════════════════════╗
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
╚══════════════════════════════════════════════════════════════════╝
```

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/synheart/synheart-sensor-agent.git
cd synheart-sensor-agent

# Build in release mode
cargo build --release

# The binary will be at target/release/synheart-sensor
```

### Requirements

- **Rust**: 1.70 or later
- **macOS**: 10.15 (Catalina) or later
- **Permissions**: Input Monitoring permission required

## Usage

### Grant Input Monitoring Permission

Before the agent can capture events, you need to grant Input Monitoring permission:

1. Open **System Preferences** > **Security & Privacy** > **Privacy**
2. Select **Input Monitoring** in the left sidebar
3. Click the lock icon and authenticate
4. Add the `synheart-sensor` application
5. Restart the application

### Commands

```bash
# Start capturing behavioral data
synheart-sensor start

# Start with specific sources
synheart-sensor start --sources keyboard
synheart-sensor start --sources mouse
synheart-sensor start --sources keyboard,mouse

# Pause collection
synheart-sensor pause

# Resume collection
synheart-sensor resume

# Show current status and statistics
synheart-sensor status

# Display privacy declaration
synheart-sensor privacy

# Export collected data
synheart-sensor export
synheart-sensor export --output /path/to/export --format jsonl

# Show configuration
synheart-sensor config
```

### Example Output

When running, the agent displays window completions:

```
Synheart Sensor Agent v0.1.0

Starting collection...
  Keyboard: enabled
  Mouse: enabled
  Window duration: 10s

Press Ctrl+C to stop

Instance ID: 550e8400-e29b-41d4-a716-446655440000
[14:32:10] Window completed: 45 keyboard, 234 mouse events
[14:32:20] Window completed: 52 keyboard, 198 mouse events
[14:32:30] Window completed: 38 keyboard, 256 mouse events
```

## HSI Output Format

The agent exports data in HSI (Human Sensor Interface) JSON format:

```json
{
  "hsi_version": "1.0",
  "producer": {
    "name": "synheart-sensor-agent",
    "version": "0.1.0",
    "instance_id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "window": {
    "start": "2024-01-15T14:32:00Z",
    "end": "2024-01-15T14:32:10Z",
    "duration_secs": 10.0,
    "is_session_start": false
  },
  "signals": {
    "behavior": {
      "keyboard": {
        "typing_rate": 4.5,
        "pause_count": 2,
        "mean_pause_ms": 850.0,
        "latency_variability": 45.2,
        "hold_time_mean": 95.0,
        "burst_index": 0.65,
        "session_continuity": 0.82
      },
      "mouse": {
        "mouse_activity_rate": 23.4,
        "mean_velocity": 125.6,
        "velocity_variability": 89.3,
        "acceleration_spikes": 5,
        "click_rate": 0.3,
        "scroll_rate": 0.8,
        "idle_ratio": 0.15,
        "micro_adjustment_ratio": 0.22
      },
      "derived": {
        "interaction_rhythm": 0.72,
        "friction": 0.25,
        "motor_stability": 0.68,
        "focus_continuity_proxy": 0.79
      }
    },
    "event_counts": {
      "keyboard_events": 45,
      "mouse_events": 234,
      "total_events": 279
    }
  },
  "privacy": {
    "content_captured": false,
    "coordinates_captured": false,
    "data_categories": ["timing", "magnitude", "derived_features"]
  },
  "device": {
    "os": "macos",
    "agent_version": "0.1.0"
  }
}
```

## Behavioral Features

### Keyboard Features

| Feature | Description |
|---------|-------------|
| `typing_rate` | Keys pressed per second |
| `pause_count` | Number of pauses (gaps > 500ms) |
| `mean_pause_ms` | Average pause duration |
| `latency_variability` | Std dev of inter-key intervals |
| `hold_time_mean` | Average key hold duration |
| `burst_index` | Burstiness of typing (0-1) |
| `session_continuity` | Active typing ratio |

### Mouse Features

| Feature | Description |
|---------|-------------|
| `mouse_activity_rate` | Movement events per second |
| `mean_velocity` | Average cursor speed |
| `velocity_variability` | Consistency of movement |
| `acceleration_spikes` | Sudden speed changes |
| `click_rate` | Clicks per second |
| `scroll_rate` | Scroll events per second |
| `idle_ratio` | Idle vs active time |
| `micro_adjustment_ratio` | Small movements ratio |

### Derived Signals

| Signal | Description |
|--------|-------------|
| `interaction_rhythm` | Overall input regularity |
| `friction` | Hesitation/correction indicator |
| `motor_stability` | Movement consistency |
| `focus_continuity_proxy` | Sustained attention indicator |

## Configuration

Configuration is stored at:
- **macOS**: `~/Library/Application Support/synheart-sensor-agent/config.json`

Default configuration:

```json
{
  "window_duration": 10,
  "sources": {
    "keyboard": true,
    "mouse": true
  },
  "paused": false,
  "session_gap_threshold_secs": 300
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Synheart Sensor Agent                     │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐       │
│  │  Collector  │──▶│  Windowing  │──▶│  Features   │       │
│  │   (macOS)   │   │  (10s bins) │   │ (compute)   │       │
│  └─────────────┘   └─────────────┘   └─────────────┘       │
│         │                                    │              │
│         ▼                                    ▼              │
│  ┌─────────────┐                     ┌─────────────┐       │
│  │Transparency │                     │    HSI      │       │
│  │    Log      │                     │  Snapshot   │       │
│  └─────────────┘                     └─────────────┘       │
└─────────────────────────────────────────────────────────────┘
```

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- start
```

### Running the Demo

```bash
cargo run --example capture_demo
```

### Project Structure

```
synheart-sensor-agent/
├── Cargo.toml              # Project manifest
├── src/
│   ├── main.rs             # CLI entry point
│   ├── lib.rs              # Library exports
│   ├── config.rs           # Configuration management
│   ├── core/
│   │   ├── mod.rs          # Core module
│   │   ├── windowing.rs    # Window management
│   │   ├── features.rs     # Feature computation
│   │   └── hsi.rs          # HSI snapshot builder
│   ├── collector/
│   │   ├── mod.rs          # Collector module
│   │   ├── types.rs        # Event types
│   │   └── macos.rs        # macOS implementation
│   └── transparency/
│       ├── mod.rs          # Transparency module
│       └── log.rs          # Privacy log
└── examples/
    └── capture_demo.rs     # Demo application
```

## Troubleshooting

### "Input Monitoring permission not granted"

1. Open System Preferences > Security & Privacy > Privacy
2. Select Input Monitoring
3. Ensure the application is in the list and checked
4. If already checked, remove and re-add the application
5. Restart the application

### No events being captured

- Ensure you're actively typing or moving the mouse
- Check that the correct sources are enabled (`--sources all`)
- Verify permission is granted with `synheart-sensor status`

### High CPU usage

- This is normal during active input
- CPU usage should be minimal when idle
- Consider reducing window duration if needed

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- macOS event capture via [core-graphics](https://crates.io/crates/core-graphics)
- CLI powered by [clap](https://crates.io/crates/clap)
