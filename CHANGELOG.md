# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release

## [0.0.1] - 2026-01-15

### Added
- Initial V0 (Research Preview) release
- macOS support using Core Graphics event taps
- Keyboard event collection (timing only, no key content)
- Mouse event collection (magnitude only, no coordinates)
- 10-second windowing system for event aggregation
- 16 behavioral features:
  - Keyboard: typing_rate, pause_count, mean_pause_ms, latency_variability, hold_time_mean, burst_index, session_continuity
  - Mouse: mouse_activity_rate, mean_velocity, velocity_variability, acceleration_spikes, click_rate, scroll_rate, idle_ratio, micro_adjustment_ratio
  - Derived: interaction_rhythm, friction, motor_stability, focus_continuity_proxy
- HSI (Human Sensor Interface) v1.0 JSON export format
- Transparency logging for data collection visibility
- CLI commands: start, pause, resume, status, privacy, export, config
- Privacy declaration display
- Session boundary detection
- Configurable window duration and session gap threshold

### Security
- Strict privacy-by-design: no key content, no coordinates, no app context
- All data processed locally
- Raw events discarded after each 10-second window

[Unreleased]: https://github.com/synheart-ai/synheart-sensor-agent/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/synheart-ai/synheart-sensor-agent/releases/tag/v0.1.0
