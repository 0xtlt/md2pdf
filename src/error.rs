use std::path::PathBuf;

/// Errors returned while reading Markdown, validating options, or writing PDF.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The requested Markdown source does not exist.
    #[error("Markdown file not found: {0}")]
    InputNotFound(PathBuf),
    /// The Markdown source contains no meaningful content.
    #[error("the Markdown document is empty")]
    EmptyInput,
    /// Standard input was used without an explicit output path.
    #[error("--output is required when reading from standard input")]
    OutputRequiredForStdin,
    /// Positional input and the legacy `--input` option were both supplied.
    #[error("use either SOURCE or --input, not both")]
    ConflictingInputs,
    /// No Markdown source was supplied.
    #[error("provide a Markdown source file")]
    MissingInput,
    /// The requested page margin is outside the supported range.
    #[error("--margin must be between 8 and 45 mm")]
    InvalidMargin,
    /// The accent color is not a hexadecimal `#RRGGBB` value.
    #[error("--accent must be a valid #RRGGBB color")]
    InvalidAccent,
    /// Reading the Markdown source failed.
    #[error("failed to read {path}: {source}")]
    Read {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying filesystem error.
        source: std::io::Error,
    },
    /// Creating the output directory or writing the PDF failed.
    #[error("failed to write {path}: {source}")]
    Write {
        /// Path that could not be written.
        path: PathBuf,
        /// Underlying filesystem error.
        source: std::io::Error,
    },
    /// Typst compilation or PDF serialization failed.
    #[error("PDF generation failed: {0}")]
    Pdf(String),
}

/// Convenient result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
