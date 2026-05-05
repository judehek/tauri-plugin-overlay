//! Plugin error type.
//!
//! Plugin commands return `Result<T, Error>`; serde-serialized as a
//! string for the JS side (Tauri auto-stringifies command errors via
//! `Display`). All variants flatten to a human-readable message.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("overlay plugin not initialized; call `overlay::init()` from your tauri::Builder")]
    NotInitialized,

    #[error("overlay is already attached; detach before reattaching")]
    AlreadyAttached,

    #[error("overlay is not attached; call `overlay::attach(pid)` first")]
    NotAttached,

    #[cfg(target_os = "windows")]
    #[error("overlay engine error: {0}")]
    Engine(#[from] overlay_engine::Error),

    #[error("overlay platform unsupported: {0}")]
    Unsupported(&'static str),

    #[error(transparent)]
    Tauri(#[from] tauri::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Other(err.to_string())
    }
}
