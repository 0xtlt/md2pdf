//! Core library for the `md2pdf` command-line application.
//!
//! The crate converts Markdown events into a self-contained Typst document,
//! then compiles that document into a PDF with embedded fonts and syntax
//! highlighting assets.

#![warn(missing_docs)]

/// Command-line arguments and value enums.
pub mod cli;
/// Application error types.
pub mod error;
/// Markdown parsing and Typst source generation.
pub mod markdown;
/// Embedded Typst compilation and PDF serialization.
pub mod pdf;

pub use error::{Error, Result};
