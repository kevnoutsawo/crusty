#![warn(missing_docs)]

//! Persistence layer for Crusty.
//!
//! Provides SQLite-backed storage for collections, requests,
//! environments, history, and settings.

mod error;
mod history;
mod migrations;
mod store;

pub use error::StoreError;
pub use history::HistoryEntry;
pub use store::Store;
