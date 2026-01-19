# Synheart Flux Integration

This document explains how to use synheart-flux with synheart-sensor-agent for baseline tracking and HSI-compliant metrics enrichment.

## Overview

The synheart-sensor-agent can optionally integrate with synheart-flux to provide:
- Rolling baseline tracking across sessions
- HSI-compliant behavioral metrics (distraction score, focus hint, burstiness)
- Cross-session deviation analysis
- Enriched output with both sensor features and flux-computed metrics

## Enabling Flux Integration

### Compile-time Feature Flag

The flux integration is behind a feature flag. Enable it when building:

```bash
# Build with flux support
cargo build --release --features flux

# Run with flux enabled
./target/release/synheart-sensor start --flux
```

### Runtime Flag

Even with the flux feature compiled in, you must enable it at runtime:

```bash
# Start with flux baseline tracking (20 session window)
synheart-sensor start --flux

# Custom baseline window size
synheart-sensor start --flux --baseline-window 30
```

## Output Files

When flux is enabled, you get additional output files:

```
~/.synheart-sensor/export/
├── session_20240115_143000.json          # Standard HSI snapshots
├── session_20240115_143000_enriched.json # Enriched snapshots with flux metrics

~/.synheart-sensor/data/
├── flux_baselines.json                   # Persisted baseline data
├── transparency.json                     # Collection statistics
```

## Enriched Output Format

When flux is enabled, each window produces an HSI 1.0 compliant enriched snapshot:

```json
{
  "hsi_version": "1.0",
  "observed_at_utc": "2024-01-15T14:32:10+00:00",
  "computed_at_utc": "2024-01-15T14:32:10+00:00",
  "producer": {
    "name": "synheart-sensor-agent",
    "version": "0.1.0",
    "instance_id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "window_ids": ["w_1705327930000"],
  "windows": {
    "w_1705327930000": {
      "start": "2024-01-15T14:32:00+00:00",
      "end": "2024-01-15T14:32:10+00:00"
    }
  },
  "source_ids": ["s_keyboard_mouse_550e8400", "s_flux_behavioral"],
  "sources": {
    "s_keyboard_mouse_550e8400": {
      "type": "sensor",
      "quality": 0.85,
      "degraded": false
    },
    "s_flux_behavioral": {
      "type": "derived",
      "quality": 0.95,
      "degraded": false,
      "notes": "Flux behavioral metrics derived from sensor data"
    }
  },
  "axes": {
    "behavior": {
      "readings": [
        { "axis": "typing_rate", "score": 0.45, "confidence": 0.85, "window_id": "w_1705327930000", "direction": "higher_is_more", "unit": "normalized", "evidence_source_ids": ["s_keyboard_mouse_550e8400"] },
        { "axis": "focus_continuity", "score": 0.79, "confidence": 0.85, "window_id": "w_1705327930000", "direction": "higher_is_more", "evidence_source_ids": ["s_keyboard_mouse_550e8400"] },
        { "axis": "idle_ratio", "score": 0.15, "confidence": 0.85, "window_id": "w_1705327930000", "direction": "higher_is_more", "unit": "ratio", "evidence_source_ids": ["s_keyboard_mouse_550e8400"] },
        { "axis": "distraction", "score": 0.35, "confidence": 0.95, "window_id": "w_1705327930000", "direction": "higher_is_more", "evidence_source_ids": ["s_flux_behavioral"] },
        { "axis": "focus", "score": 0.65, "confidence": 0.95, "window_id": "w_1705327930000", "direction": "higher_is_more", "evidence_source_ids": ["s_flux_behavioral"] },
        { "axis": "burstiness", "score": 0.55, "confidence": 0.95, "window_id": "w_1705327930000", "direction": "bidirectional", "unit": "barabasi_index", "evidence_source_ids": ["s_flux_behavioral"] },
        { "axis": "interaction_intensity", "score": 0.78, "confidence": 0.95, "window_id": "w_1705327930000", "direction": "higher_is_more", "unit": "normalized", "evidence_source_ids": ["s_flux_behavioral"] }
      ]
    }
  },
  "privacy": {
    "contains_pii": false,
    "raw_biosignals_allowed": false,
    "derived_metrics_allowed": true,
    "notes": "No key content or coordinates captured - timing and magnitude only"
  },
  "meta": {
    "keyboard_events": 45,
    "mouse_events": 234,
    "duration_secs": 10.0,
    "baseline_distraction": 0.38,
    "sessions_in_baseline": 15
  }
}
```

## Programmatic Usage

### Basic Usage

```rust
use synheart_sensor_agent::flux::SensorFluxProcessor;
use synheart_sensor_agent::{compute_features, HsiBuilder, WindowManager};

// Create processor with 20-session baseline window
let mut processor = SensorFluxProcessor::new(20);

// Process windows as they complete
for window in window_manager.take_completed_windows() {
    let features = compute_features(&window);
    let base_snapshot = hsi_builder.build(&window, &features);

    // Get enriched snapshot with flux metrics
    let enriched = processor.process_window(&window, &features, base_snapshot)?;

    println!("Distraction: {:.2}", enriched.flux_behavior.unwrap().distraction_score);
    if let Some(baseline) = &enriched.baseline {
        println!("Deviation from baseline: {:.1}%", baseline.distraction_deviation_pct.unwrap_or(0.0));
    }
}
```

### Baseline Persistence

```rust
// Save baselines at end of session
let baselines_json = processor.save_baselines()?;
std::fs::write("baselines.json", baselines_json)?;

// Load baselines on next startup
let mut processor = SensorFluxProcessor::new(20);
let baselines_json = std::fs::read_to_string("baselines.json")?;
processor.load_baselines(&baselines_json)?;
```

### Custom Device ID

```rust
// Create processor with custom device ID
let processor = SensorFluxProcessor::with_device_id(20, "my-macbook-pro");
```

## Flux Metrics Explained

| Metric | Description | Range |
|--------|-------------|-------|
| `distraction_score` | Composite distraction indicator | 0.0 - 1.0 |
| `focus_hint` | Inverse of distraction (1.0 = fully focused) | 0.0 - 1.0 |
| `task_switch_rate` | Rate of context switches (exponential saturation) | 0.0 - 1.0 |
| `burstiness` | Temporal clustering of events (Barabasi formula) | 0.0 - 1.0 |
| `scroll_jitter_rate` | Direction reversals in scrolling | 0.0 - 1.0 |
| `interaction_intensity` | Events per second normalized | 0.0+ |
| `deep_focus_blocks` | Count of sustained engagement periods (≥120s) | 0+ |
| `idle_ratio` | Time spent idle / total time | 0.0 - 1.0 |
| `fragmented_idle_ratio` | Idle segments / duration (fragmentation) | 0.0 - 1.0 |

## Baseline Information

The baseline tracks rolling statistics across sessions:

| Field | Description |
|-------|-------------|
| `distraction` | Mean distraction score across baseline window |
| `focus` | Mean focus score across baseline window |
| `distraction_deviation_pct` | Current deviation from baseline (%) |
| `sessions_in_baseline` | Number of sessions in rolling window |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Synheart Sensor Agent                     │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐       │
│  │  Collector  │──▶│  Windowing  │──▶│  Features   │       │
│  │   (macOS)   │   │  (10s bins) │   │ (compute)   │       │
│  └─────────────┘   └─────────────┘   └──────┬──────┘       │
│                                              │              │
│                                              ▼              │
│                                       ┌─────────────┐       │
│                                       │ HSI Builder │       │
│                                       └──────┬──────┘       │
│                                              │              │
│  ┌───────────────────────────────────────────┼──────────────│
│  │ FLUX INTEGRATION (optional)               │              │
│  ├───────────────────────────────────────────┼──────────────│
│  │  ┌─────────────┐   ┌─────────────┐       │              │
│  │  │   Adapter   │◀──┤   Window    │◀──────┘              │
│  │  │ (sensor→beh)│   │   Data      │                      │
│  │  └──────┬──────┘   └─────────────┘                      │
│  │         │                                                │
│  │         ▼                                                │
│  │  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   │
│  │  │  Behavior   │──▶│  Baseline   │──▶│  Enriched   │   │
│  │  │  Processor  │   │   Store     │   │  Snapshot   │   │
│  │  └─────────────┘   └─────────────┘   └─────────────┘   │
│  └──────────────────────────────────────────────────────────│
└─────────────────────────────────────────────────────────────┘
```

## Event Mapping

Sensor events are mapped to behavioral events for flux processing:

| Sensor Event | Behavioral Event | Notes |
|--------------|------------------|-------|
| Keyboard (key down) | `typing` | Captures typing rhythm |
| Mouse Move | `scroll` | Captures cursor movement velocity |
| Left/Right Click | `tap` | Captures click events |
| Scroll | `scroll` | Captures scroll velocity |
| Drag | `swipe` | Captures drag operations |

## Building from Source

```bash
# Build sensor agent with flux support
cargo build --release --features flux

# Run tests
cargo test --features flux
```

## Troubleshooting

### "flux feature not enabled at compile time"

Build with the flux feature:
```bash
cargo build --release --features flux
```

### Baselines not persisting

Check that the data directory exists and is writable:
```bash
ls -la ~/.synheart-sensor/data/
```

### Flux metrics showing 0.0

Ensure there are enough events in the window. Flux requires multiple events to compute meaningful metrics. Very short or idle windows may produce zero values.
