# Contributing to PulseCast

Thank you for your interest in contributing to PulseCast! This guide will help you get started.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USER/pulsecast.git`
3. Create a feature branch: `git checkout -b feat/my-feature`
4. Make your changes
5. Run tests: `cargo test --workspace`
6. Run clippy: `cargo clippy --workspace -- -D warnings`
7. Commit with a clear message
8. Push and open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.77+ (with clippy)
- Node.js 18+ (for Tauri dev)
- Platform-specific Tauri dependencies (see [Tauri docs](https://tauri.app/start/prerequisites/))

### Running Locally

```bash
# Server only (fastest iteration)
cargo run -p pulsecast-server -- --webroot .

# With BLE
cargo run -p pulsecast-server --features ble -- --webroot . --real

# Full Tauri app
npm install && npm run tauri dev
```

## Code Style

- **Rust**: Follow `cargo clippy` with `-D warnings`. Use `rustfmt` defaults.
- **C (OBS plugin)**: Follow existing style in `obs-pulsecast/src/`. 4-space indent, snake_case.
- **HTML/CSS/JS**: Vanilla only, no frameworks. Keep it simple and readable.
- **Commits**: One logical change per commit. Message format: `type: description` (e.g., `feat: add BLE auto-reconnect toast`).

## Project Structure

| Area | Path | Notes |
|------|------|-------|
| Rust server | `server/src/` | axum + tokio, BLE behind `ble` feature flag |
| Tauri app | `src-tauri/src/` | Thin wrapper, server runs embedded |
| Frontend | `ftue.html`, `dist/settings.html`, `obs-overlay.html` | Self-contained HTML files |
| OBS plugin | `obs-pulsecast/src/` | C, requires OBS dev headers to build |
| CI | `.github/workflows/` | GitHub Actions |

## Testing

- Unit tests: `cargo test --workspace`
- BLE tests require `--features ble` and a Bluetooth adapter
- Frontend: open `http://localhost:4567/` and verify manually

## Good First Issues

Look for issues labeled [`good-first-issue`](../../labels/good-first-issue). These are:
- Well-scoped with clear acceptance criteria
- Don't require deep knowledge of the codebase
- Have pointers to relevant files

## Reporting Bugs

Use the [Bug Report](.github/ISSUE_TEMPLATE/bug_report.md) template. Include:
- OS + version
- How you're running PulseCast (Release binary / from source / Tauri dev)
- Steps to reproduce
- Expected vs actual behavior

## Feature Requests

Use the [Feature Request](.github/ISSUE_TEMPLATE/feature_request.md) template. Describe:
- The problem you're trying to solve
- Your proposed solution (if any)
- Alternatives you've considered

## Code of Conduct

Be respectful and constructive. We're a small open-source project — every contribution matters.
