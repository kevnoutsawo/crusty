#![warn(missing_docs)]

//! Pre/post-request scripting engine for Crusty.
//!
//! Uses Rhai for embedded scripting. Scripts can:
//! - Set and read environment variables
//! - Access request and response data
//! - Assert on response values
//! - Log messages

pub mod context;
pub mod engine;
mod error;

pub use error::ScriptError;
