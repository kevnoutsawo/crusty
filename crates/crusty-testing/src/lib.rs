#![warn(missing_docs)]

//! Test runner, assertions, and CI mode for Crusty.
//!
//! Run collections of requests with test scripts, collect results,
//! and generate reports in JUnit XML or JSON format.

pub mod assertion;
mod error;
pub mod report;
pub mod runner;

pub use error::TestError;
