#![warn(missing_docs)]

//! HTTP engine for Crusty.
//!
//! Wraps `reqwest` for native targets. Uses conditional compilation
//! for WASM transport switching. Provides timing instrumentation for
//! request/response cycles.

mod client;
mod error;

pub use client::HttpClient;
pub use error::HttpError;
