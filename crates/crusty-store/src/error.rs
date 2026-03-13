//! Store error types.

use thiserror::Error;

/// Errors that can occur during storage operations.
#[derive(Debug, Error)]
pub enum StoreError {
    /// SQLite error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Item not found.
    #[error("{kind} not found: {id}")]
    NotFound {
        /// What kind of item was not found.
        kind: String,
        /// The ID that was looked up.
        id: String,
    },
}

/// Alias for `Result<T, StoreError>`.
pub type Result<T> = std::result::Result<T, StoreError>;
