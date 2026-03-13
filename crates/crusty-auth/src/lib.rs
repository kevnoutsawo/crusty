#![warn(missing_docs)]

//! Authentication flows for Crusty.
//!
//! Supports Bearer, Basic, API Key, OAuth 2.0, and more.

mod error;
mod provider;

pub use error::AuthError;
pub use provider::{ApiKeyLocation, AuthConfig, AuthProvider};
