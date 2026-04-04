#![warn(missing_docs)]

//! Proxy interception and request capture for Crusty.
//!
//! Provides an HTTP proxy server that captures all traffic passing
//! through it, logging requests and responses for inspection.

pub mod capture;
mod error;

pub use error::ProxyError;
