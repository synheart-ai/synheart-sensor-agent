# Contributing to Synheart Sensor Agent

Thank you for your interest in contributing to Synheart Sensor Agent! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Privacy Guidelines](#privacy-guidelines)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Reporting Issues](#reporting-issues)

## Code of Conduct

This project follows a Code of Conduct that all contributors are expected to adhere to. Please be respectful, inclusive, and constructive in all interactions.

### Our Standards

- Use welcoming and inclusive language
- Be respectful of differing viewpoints and experiences
- Gracefully accept constructive criticism
- Focus on what is best for the community
- Show empathy towards other community members

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR-USERNAME/synheart-sensor-agent.git
   cd synheart-sensor-agent
   ```
3. **Add the upstream remote**:
   ```bash
   git remote add upstream https://github.com/synheart-ai/synheart-sensor-agent.git
   ```
4. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- **Rust**: 1.70 or later ([install](https://rustup.rs/))
- **macOS**: 10.15 (Catalina) or later (for testing event capture)
- **Git**: Latest version

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Check formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings
```

### Running Locally

```bash
# Run the CLI
cargo run -- --help

# Run with debug logging
RUST_LOG=debug cargo run -- start

# Run the demo example
cargo run --example capture_demo
```

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feature/add-linux-support` - New features
- `fix/window-timing-bug` - Bug fixes
- `docs/update-readme` - Documentation updates
- `refactor/simplify-features` - Code refactoring
- `test/add-windowing-tests` - Test additions

### Commit Messages

Follow conventional commit format:

```
type(scope): short description

Longer description if needed.

Fixes #123
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, no code change
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat(collector): add Linux evdev support

Implements event collection using the evdev subsystem for Linux.
Supports keyboard and mouse events with the same privacy guarantees.

Closes #45
```

```
fix(windowing): correct session boundary detection

Session boundaries were not being detected correctly when the gap
threshold was exactly equal to the time difference between events.

Fixes #67
```

## Privacy Guidelines

**Privacy is paramount in this project.** All contributions must adhere to our strict privacy requirements:

### Absolute Requirements

1. **Never capture key content**
   - No key codes, scan codes, or characters
   - Only timing information (when a key was pressed/released)

2. **Never capture cursor coordinates**
   - No X/Y positions or screen locations
   - Only movement magnitude (how far, not where)

3. **Never capture application context**
   - No window titles, process names, or URLs
   - No information about what the user is doing

4. **Never store raw events beyond the current window**
   - Events must be aggregated into features
   - Raw data must be discarded after processing

### Code Review for Privacy

All PRs will be reviewed with privacy as the primary concern:

- Any code that accesses event content will be rejected
- Any code that stores identifiable information will be rejected
- New features must include privacy analysis in the PR description

### Privacy Checklist

Before submitting a PR, verify:

- [ ] No key codes or characters are captured
- [ ] No absolute coordinates are captured
- [ ] No application or window information is captured
- [ ] Raw events are not persisted
- [ ] New features maintain existing privacy guarantees
- [ ] Privacy declaration is updated if needed

## Pull Request Process

1. **Update documentation** for any user-facing changes
2. **Add tests** for new functionality
3. **Run the full test suite** locally:
   ```bash
   cargo test
   cargo fmt --check
   cargo clippy -- -D warnings
   ```
4. **Update CHANGELOG.md** if applicable
5. **Create the PR** with a clear description:
   - What changes were made
   - Why the changes were necessary
   - How the changes were tested
   - Privacy implications (if any)

### PR Template

```markdown
## Description
[Describe your changes]

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Refactoring
- [ ] Other (describe)

## Privacy Checklist
- [ ] No key content is captured
- [ ] No coordinates are captured
- [ ] No application context is captured
- [ ] Raw events are not persisted
- [ ] Privacy guarantees are maintained

## Testing
[Describe how you tested your changes]

## Related Issues
Closes #[issue number]
```

### Review Process

1. At least one maintainer must approve the PR
2. All CI checks must pass
3. Privacy review must be completed
4. No merge conflicts with main branch

## Coding Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting (default configuration)
- Use `clippy` for linting with `-D warnings`

### Code Organization

```rust
// Imports grouped and ordered:
// 1. Standard library
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crates
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// 3. Internal modules
use crate::core::features::WindowFeatures;
use crate::collector::types::SensorEvent;
```

### Documentation

- All public items must have doc comments
- Include examples where helpful
- Document privacy implications

```rust
/// Computes behavioral features from an event window.
///
/// # Privacy
///
/// This function only processes timing and magnitude data.
/// No key content or coordinates are accessed.
///
/// # Example
///
/// ```
/// let features = compute_features(&window);
/// assert!(features.keyboard.typing_rate >= 0.0);
/// ```
pub fn compute_features(window: &EventWindow) -> WindowFeatures {
    // ...
}
```

### Error Handling

- Use `Result` for operations that can fail
- Provide meaningful error messages
- Don't panic in library code

```rust
pub fn start(&mut self) -> Result<(), CollectorError> {
    if self.running.load(Ordering::SeqCst) {
        return Err(CollectorError::AlreadyRunning);
    }
    // ...
}
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_keyboard_features

# Run tests with output
cargo test -- --nocapture

# Run tests in release mode
cargo test --release
```

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_bounds() {
        let features = compute_features(&window);

        // All normalized features should be between 0 and 1
        assert!(features.behavioral.interaction_rhythm >= 0.0);
        assert!(features.behavioral.interaction_rhythm <= 1.0);
    }

    #[test]
    fn test_privacy_no_content() {
        let event = KeyboardEvent::new(true);

        // Verify no content fields exist
        // (This is enforced by the type system, but good to document)
        assert!(event.is_key_down); // Only timing data
    }
}
```

### Test Categories

- **Unit tests**: Test individual functions and modules
- **Integration tests**: Test module interactions
- **Privacy tests**: Verify privacy guarantees

## Documentation

### Updating Documentation

- Update README.md for user-facing changes
- Update doc comments for API changes
- Add examples for new features

### Building Documentation

```bash
# Generate and open documentation
cargo doc --open

# Generate documentation with private items
cargo doc --document-private-items
```

## Reporting Issues

### Bug Reports

Include:
1. **Description**: Clear description of the bug
2. **Steps to Reproduce**: Minimal steps to reproduce
3. **Expected Behavior**: What should happen
4. **Actual Behavior**: What actually happens
5. **Environment**: OS version, Rust version, etc.
6. **Logs**: Any relevant error messages or logs

### Feature Requests

Include:
1. **Description**: Clear description of the feature
2. **Use Case**: Why this feature is needed
3. **Privacy Analysis**: How this affects privacy guarantees
4. **Proposed Implementation**: Optional technical approach

### Security Issues

For security vulnerabilities, please do NOT create a public issue. Instead, email security@synheart.io with:

1. Description of the vulnerability
2. Steps to reproduce
3. Potential impact
4. Suggested fix (if any)

## Questions?

- Open a [GitHub Discussion](https://github.com/synheart-ai/synheart-sensor-agent/discussions) for questions
- Join our community chat (link TBD)
- Email contributors@synheart.io

Thank you for contributing to Synheart Sensor Agent!
