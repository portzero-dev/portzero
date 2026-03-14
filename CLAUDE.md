# CLAUDE.md

Guidelines for AI agents working on the PortZero codebase.

## Project Overview

PortZero is a local development reverse proxy, process manager, and HTTP traffic inspector. Single Rust binary powered by Cloudflare Pingora. Includes a Tauri v2 desktop app and a React web dashboard.

## Repository Structure

```
crates/
  portzero-core/    # Core library: proxy, router, recorder, store, config, types, mock engine, network sim, schema inference, tunnel, certs
  portzero-cli/     # CLI binary (clap). Commands in src/commands/
  portzero-api/     # HTTP API (axum) + WebSocket server
  portzero-mcp/     # MCP server for AI agents (stdio JSON-RPC)
apps/
  desktop/          # Tauri v2 desktop app + React frontend (Vite, TanStack Query, Tailwind CSS v4)
  web/              # Landing page + docs site (Next.js 15)
```

## Build & Test Commands

```sh
# Rust
cargo build --workspace                            # Debug build
cargo build --release -p portzero-cli              # Release CLI
cargo build -p portzero-cli --features tunnel      # With tunnel support
cargo test --workspace                             # All tests
cargo test -p portzero-core                        # Specific crate
cargo fmt --check                                  # Check formatting
cargo clippy --workspace -- -D warnings            # Lint (warnings = errors)

# Frontend (apps/desktop/)
pnpm install && pnpm build                         # Build dashboard
pnpm dev                                           # Dev server

# Web/Docs (apps/web/)
pnpm install && pnpm build                         # Build site
```

## Rust Conventions

- **Error handling**: `anyhow::Result<T>` for application code. `anyhow::bail!()` for fatal errors. `.unwrap()` only on locks (considered unrecoverable). Pingora code uses `pingora_core::Result`.
- **Module layout**: Flat files in `portzero-core/src/` (one file per concern). CLI commands in `portzero-cli/src/commands/` (one file per command).
- **Shared types**: All cross-crate types live in `portzero-core/src/types.rs`. Section separators use `// ---------------------------------------------------------------------------`.
- **Re-exports**: `portzero-core/src/lib.rs` re-exports key types. Consumers use `portzero_core::Router` directly.
- **Logging**: `tracing` crate only. Use structured fields: `tracing::info!(app = %name, status = %code, "message")`.
- **Async**: Tokio runtime. `Arc<T>` for sharing across tasks. `tokio::sync::mpsc` for background work. `std::sync::RwLock` for read-heavy data (router). `DashMap` for concurrent maps on hot paths.
- **Derives**: `Debug, Clone, Serialize, Deserialize` on all data types. Serde attributes: `#[serde(tag = "type")]`, `#[serde(default)]`, `#[serde(skip_serializing_if = "Option::is_none")]`.
- **Tests**: Inline `#[cfg(test)] mod tests` at bottom of source files. Async tests use `#[tokio::test]`. Use `tempfile` for filesystem tests, `axum-test` for API tests, `Store::in_memory()` for store tests.
- **Doc comments**: `//!` module-level at top of each file. `///` on public functions.

## Feature Flags

| Feature | Effect |
|---------|--------|
| `tunnel` | Enables LocalUp tunnel support (opt-in on all crates) |

Default features are empty. Use `--features tunnel` to enable.

## Frontend Conventions (apps/desktop/src/)

- **React 19** functional components, PascalCase filenames matching component name.
- **TanStack Query** for data fetching with string array keys (`["apps"]`, `["requests"]`).
- **Dual transport**: API client checks `isTauri()` for IPC, falls back to HTTP REST.
- **Types** in `lib/types.ts` mirror Rust `types.rs`. Use `interface` for objects, `type` for unions.
- **Styling**: Tailwind CSS v4, zinc-based dark palette, violet accents, lucide-react icons.

## Commit Messages

Conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`, `ci:`.

## What NOT to Do

- Do not use `println!` for logging in library code (use `tracing`).
- Do not add `any` types in TypeScript.
- Do not commit `.env` files, SQLite databases, or build artifacts.
- Do not modify `dashboard-dist/` directly (it's a build output from `pnpm build:web`).
- Do not use `tokio::sync::Mutex` where `std::sync::Mutex` suffices (SQLite is sync).
