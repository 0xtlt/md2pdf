//! Core library for the `md2pdf` command-line application.
//!
//! The crate converts Markdown events into a self-contained Typst document,
//! highlights fenced code with TextMate grammars, then compiles the resulting
//! document into a PDF with embedded fonts.

#![warn(missing_docs)]

/// Command-line arguments and value enums.
pub mod cli;
/// Application error types.
pub mod error;
/// TextMate syntax highlighting and theme resolution.
pub mod highlight;
/// Markdown parsing and Typst source generation.
pub mod markdown;
/// Embedded Typst compilation and PDF serialization.
pub mod pdf;

pub use error::{Error, Result};
