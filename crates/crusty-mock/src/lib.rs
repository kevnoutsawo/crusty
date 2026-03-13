#![warn(missing_docs)]

//! Mock server engine for Crusty.
//!
//! Create mock HTTP endpoints with configurable responses,
//! request matching, and delay simulation.

mod error;
pub mod endpoint;
pub mod server;

pub use error::MockError;
