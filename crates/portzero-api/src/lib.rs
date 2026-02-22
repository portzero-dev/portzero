//! PortZero HTTP API + WebSocket server.
//!
//! This crate provides the REST API and WebSocket event stream that powers
//! the PortZero dashboard (both Tauri desktop and web fallback).
//!
//! All endpoints are served under `_portzero.localhost:1337/api/*`.

pub mod routes;
pub mod server;
pub mod state;
pub mod static_files;
pub mod ws;

pub use server::build_router;
pub use state::AppState;
