//! CLI command implementations for PortZero.
//!
//! Task 1 owns: run, up, list, logs, trust
//! Task 4 owns: mock, throttle, share

// Task 1 commands
pub mod run;
pub mod list;
pub mod up;
pub mod logs;
pub mod trust;

// Task 4 commands
pub mod mock;
pub mod throttle;
#[cfg(feature = "tunnel")]
pub mod share;

// Auth commands
#[cfg(feature = "tunnel")]
pub mod auth;
