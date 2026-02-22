# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Reverse proxy with subdomain routing via Cloudflare Pingora (`<app>.localhost:1337`)
- Process manager with auto-restart, deterministic port assignment, and log capture
- Full HTTP request/response traffic capture with SQLite persistence
- Request replay with optional overrides
- Response mocking engine with per-route rules
- Network simulation (latency injection, packet loss, bandwidth throttling)
- Passive OpenAPI schema inference from observed traffic
- REST API + WebSocket server for real-time events
- MCP server for AI agent integration (stdio JSON-RPC)
- CLI with argument disambiguation (`portzero next dev` or `portzero my-app next dev`)
- Configuration file support (`portzero.toml`) with `portzero up` / `portzero down`
- Auto-generated TLS certificates (rcgen + rustls, no OpenSSL)
- Public tunnel sharing via LocalUp (`portzero share`)
- Tauri v2 desktop app with system tray
- Web dashboard fallback at `_portzero.localhost:1337`
