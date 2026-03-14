# PortZero

[![CI](https://github.com/portzero-dev/portzero/actions/workflows/ci.yml/badge.svg)](https://github.com/portzero-dev/portzero/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE-MIT)
[![GitHub release](https://img.shields.io/github/v/release/portzero-dev/portzero?label=release)](https://github.com/portzero-dev/portzero/releases)
[![GitHub stars](https://img.shields.io/github/stars/portzero-dev/portzero)](https://github.com/portzero-dev/portzero/stargazers)

Local development reverse proxy, process manager, and traffic inspector.

PortZero assigns stable `<name>.localhost` URLs to your dev servers, captures all HTTP traffic for inspection, and provides request replay, mocking, interception, and network simulation -- all from a single Rust binary powered by [Cloudflare Pingora](https://github.com/cloudflare/pingora).

## Features

- **Reverse proxy** -- Route `my-app.localhost:1337` to your dev server's port. HTTP/1.1, HTTP/2, and WebSocket support via Pingora.
- **Process manager** -- Spawn, monitor, and auto-restart child processes with deterministic port assignment. No more port conflicts.
- **Traffic inspector** -- Full request/response capture with filtering, search, and persistence to SQLite.
- **Request replay** -- One-click re-send of captured requests with optional overrides.
- **Response mocking** -- Per-route synthetic responses without hitting your upstream app.
- **Request interception** -- Pause, inspect, edit, and forward/drop live requests.
- **Network simulation** -- Latency injection, packet loss, and bandwidth throttling.
- **API schema inference** -- Passive OpenAPI schema generation from observed traffic.
- **Public tunnels** -- Expose local apps to the internet via [LocalUp](https://github.com/localup-dev/localup) (QUIC/WS/H2).
- **Desktop app** -- Native [Tauri v2](https://v2.tauri.app/) dashboard with system tray.
- **Web dashboard** -- Embedded SPA served by the daemon at `_portzero.localhost:1337`.

## Quick Start

### Install

```sh
# Homebrew (macOS / Linux)
brew install portzero-dev/tap/portzero

# One-liner install (macOS / Linux)
curl -fsSL https://goport0.dev/install.sh | bash

# Or from source
cargo install --path crates/portzero-cli
```

### Run a single app

```sh
# Name inferred from current directory
portzero next dev
# -> http://my-project.localhost:1337

# Explicit name
portzero my-app next dev
# -> http://my-app.localhost:1337
```

### Run multiple apps

Create a `portzero.toml` in your project root (see [portzero.example.toml](portzero.example.toml)):

```toml
[proxy]
port = 1337
https = true

[apps.web]
command = "pnpm dev"
cwd = "./apps/web"
auto_restart = true

[apps.api]
command = "pnpm start"
cwd = "./apps/api"
```

Then start everything:

```sh
portzero up
# -> http://web.localhost:1337
# -> http://api.localhost:1337
```

### Trust the local CA (HTTPS)

```sh
portzero trust
```

This installs the auto-generated CA certificate into your system trust store so browsers don't show certificate warnings.

## CLI Reference

```
portzero <command>                  # Name = basename(cwd), run command
portzero <name> <command>           # Explicit name, run command

portzero up                         # Start all apps from portzero.toml
portzero down                       # Stop all apps
portzero list                       # List active apps + URLs
portzero logs <name>                # Tail logs for an app
portzero start [-d]                 # Start daemon (foreground or background)
portzero stop                       # Stop the daemon
portzero status                     # Show daemon status

portzero mock add <app> <method> <path>  # Create a response mock
portzero mock list                  # List active mocks
portzero mock enable <id>           # Enable a mock rule
portzero mock disable <id>          # Disable a mock rule
portzero mock delete <id>           # Delete a mock rule

portzero throttle set <app> [opts]  # Set network simulation
portzero throttle list              # List active profiles
portzero throttle clear <app>       # Clear simulation

portzero share <app>                # Start public tunnel via LocalUp
portzero trust                      # Install CA into system trust store
portzero untrust                    # Remove CA from system trust store
```

## Architecture

PortZero is a Cargo workspace with the following crates:

| Crate | Description |
|-------|-------------|
| `portzero-core` | Core library: proxy, router, recorder, process manager, mock engine, network sim, schema inference, tunnel, certs |
| `portzero-cli` | CLI binary (`portzero` command) |
| `portzero-api` | HTTP API (axum) + WebSocket server |
| `portzero-mcp` | MCP server for AI agent integration (stdio JSON-RPC) |

The React dashboard lives in `apps/desktop/` and is shared between the Tauri app and the embedded web fallback.

See [ARCHITECTURE.md](ARCHITECTURE.md) for full design documentation.

## API

The daemon exposes a REST API + WebSocket at `_portzero.localhost:1337`:

```
GET    /api/apps                    # List apps
GET    /api/requests                # List captured requests (with filtering)
GET    /api/requests/:id            # Full request/response detail
POST   /api/requests/:id/replay     # Replay a request
GET    /api/mocks                   # List mock rules
POST   /api/mocks                   # Create a mock
PUT    /api/network/:app            # Set network simulation profile
GET    /api/apps/:name/schema       # Get inferred API schema
WS     /api/ws                      # Real-time event stream
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the complete endpoint reference.

## Platform Support

| Platform | CLI | Desktop App |
|----------|-----|-------------|
| macOS (Apple Silicon) | Yes | Yes |
| macOS (Intel) | Yes | Yes |
| Linux x86_64 | Yes | Yes (AppImage, deb) |
| Linux aarch64 | Yes | — |
| Windows | Not yet | Not yet |

## Building from Source

### Prerequisites

- Rust 1.75+ (for the workspace)
- Node.js 18+ and pnpm (for the dashboard frontend)

### Build

```sh
# Build the CLI binary
cargo build --release -p portzero-cli

# Build with tunnel support
cargo build --release -p portzero-cli --features tunnel

# Build the dashboard frontend
cd apps/desktop && pnpm install && pnpm build

# Build the Tauri desktop app
cd apps/desktop && pnpm tauri build
```

### Test

```sh
# Run all Rust tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p portzero-core
cargo test -p portzero-api
```

## Documentation

Full documentation is available at [goport0.dev/docs](https://goport0.dev/docs).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
