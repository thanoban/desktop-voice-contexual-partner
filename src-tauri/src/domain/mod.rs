//! Stable domain contracts used by the desktop shell, providers, and integrations.
//!
//! These types intentionally do not depend on Tauri so they can be reused by
//! adapters, tests, workers, and a future authenticated local API.

pub mod events;
pub mod execution;
pub mod integration;
pub mod provider;
pub mod session;

pub const CONTRACT_SCHEMA_VERSION: u16 = 1;

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
