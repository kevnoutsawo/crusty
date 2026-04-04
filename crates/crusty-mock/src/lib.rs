#![warn(missing_docs)]

//! Mock server engine for Crusty.
//!
//! Create mock HTTP endpoints with configurable responses,
//! request matching, and delay simulation.

pub mod endpoint;
mod error;
pub mod server;

pub use error::MockError;
