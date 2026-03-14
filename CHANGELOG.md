# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-03-13

### Added

- **Reverse proxy** with subdomain routing via Cloudflare Pingora (`<app>.localhost:1337`). Supports HTTP/1.1, HTTP/2, and WebSocket.
- **Process manager** with auto-restart, deterministic port assignment, and log capture.
- **Traffic inspector** with full HTTP request/response capture and SQLite persistence.
- **Request replay** with optional header/body overrides.
- **Response mocking engine** with per-route synthetic responses.
- **Request interception** to pause, inspect, edit, and forward/drop live requests.
- **Network simulation** with latency injection, packet loss, and bandwidth throttling.
- **Passive OpenAPI schema inference** from observed traffic.
- **REST API + WebSocket server** for real-time dashboard events.
- **MCP server** for AI agent integration via stdio JSON-RPC.
- **CLI** with argument disambiguation (`portzero next dev` or `portzero my-app next dev`).
- **Configuration file** support (`portzero.toml`) with `portzero up` / `portzero down`.
- **Auto-generated TLS certificates** via rcgen + rustls (no OpenSSL dependency).
- **Public tunnel sharing** via LocalUp (`portzero share`).
- **Tauri v2 desktop app** with system tray integration.
- **Web dashboard** fallback at `_portzero.localhost:1337`.
- **Landing page and documentation** site (Next.js 15).
- **CI/CD pipelines** for Rust (check, clippy, test, fmt), web build, CLI release, and desktop release.
- **189 tests** across core, API, and MCP crates.

### Platform Support

| Platform | CLI | Desktop App |
|----------|-----|-------------|
| macOS (Apple Silicon) | Yes | Yes |
| macOS (Intel) | Yes | Yes |
| Linux x86_64 | Yes | Yes (AppImage, deb) |
| Linux aarch64 | Yes | -- |
| Windows | Not yet | Not yet |
