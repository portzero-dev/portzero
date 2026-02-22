//! CLI command implementations for PortZero.
//!
//! Task 1 owns: run, up, list, logs, trust
//! Task 4 owns: mock, throttle, share

// Task 1 commands
pub mod list;
pub mod logs;
pub mod run;
pub mod trust;
pub mod up;

// Task 4 commands
pub mod mock;
#[cfg(feature = "tunnel")]
pub mod share;
pub mod throttle;

// Auth commands
#[cfg(feature = "tunnel")]
pub mod auth;
