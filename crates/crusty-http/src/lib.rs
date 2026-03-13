#![warn(missing_docs)]

//! HTTP engine for Crusty.
//!
//! Wraps `reqwest` for native targets. Uses conditional compilation
//! for WASM transport switching.
