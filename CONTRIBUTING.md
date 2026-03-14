# Contributing to PortZero

Thank you for your interest in contributing to PortZero! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) 20+ and [pnpm](https://pnpm.io/) (for the dashboard frontend)
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) (for the desktop app, optional)

### Setup

```sh
git clone https://github.com/portzero-dev/portzero.git
cd portzero

# Build and test the Rust workspace
cargo build --workspace
cargo test --workspace

# Set up the dashboard frontend
cd apps/desktop
pnpm install
```

### Project Structure

```
crates/
  portzero-core/    # Core library (proxy, router, recorder, etc.)
  portzero-cli/     # CLI binary
  portzero-api/     # HTTP API + WebSocket server
  portzero-mcp/     # MCP server for AI agents
apps/
  desktop/          # Tauri v2 desktop app + React frontend
```

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the full design document.

## Development Workflow

### Running Tests

```sh
# Run all Rust tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p portzero-core
cargo test -p portzero-api

# Run with logging output
RUST_LOG=debug cargo test --workspace -- --nocapture
```

### Code Style

**Rust:**
- Format with `cargo fmt`
- Lint with `cargo clippy --workspace -- -D warnings`
- Follow standard Rust conventions

**TypeScript/React (dashboard):**
- Format with Prettier (via pnpm)
- Follow the existing code patterns in `apps/desktop/src/`

### Building

```sh
# Debug build
cargo build --workspace

# Release build
cargo build --release -p portzero-cli

# Build with tunnel feature
cargo build -p portzero-cli --features tunnel

# Build the desktop app
cd apps/desktop && pnpm tauri build
```

## Making Changes

### Branch Naming

- `feat/description` -- New features
- `fix/description` -- Bug fixes
- `docs/description` -- Documentation changes
- `refactor/description` -- Code refactoring
- `test/description` -- Test additions or changes

### Commit Messages

Write clear, concise commit messages that explain **why** the change was made:

```
feat: add request body size limit configuration

Allow users to configure the max request body size captured by the
recorder via portzero.toml. Defaults to 1MB to prevent memory
exhaustion with large uploads.
```

Use conventional commit prefixes: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`, `ci:`.

### Pull Requests

1. Fork the repository and create your branch from `main`
2. Make your changes with appropriate tests
3. Ensure all tests pass: `cargo test --workspace`
4. Ensure code is formatted: `cargo fmt --check`
5. Ensure clippy is happy: `cargo clippy --workspace -- -D warnings`
6. Submit a pull request with a clear description of the changes

### Adding Tests

- Unit tests go in the same file as the code they test, inside a `#[cfg(test)]` module
- Integration tests for the API go in `crates/portzero-api/tests/`
- Use `tempfile` for tests that need filesystem access
- Use `axum-test` for HTTP endpoint tests

## Reporting Issues

- Use the [bug report template](https://github.com/portzero-dev/portzero/issues/new?template=bug_report.yml) for bugs
- Use the [feature request template](https://github.com/portzero-dev/portzero/issues/new?template=feature_request.yml) for new ideas
- Search existing issues before creating a new one

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](./CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## License

By contributing, you agree that your contributions will be dual-licensed under the MIT and Apache 2.0 licenses. See [LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE](./LICENSE-APACHE).
