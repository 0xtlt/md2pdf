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
    /// Standard input cannot be combined with batch conversion features.
    #[error("standard input cannot be combined with directories, --grep, or multiple sources")]
    StdinBatchUnsupported,
    /// `--output-mode merge` or `zip` was used without `--output`.
    #[error("--output is required for --output-mode {0}")]
    OutputRequiredForMode(&'static str),
    /// Per-file mode received an `--output` path that is not a directory.
    #[error("--output must be a directory in --output-mode files: {0}")]
    OutputMustBeDirectory(PathBuf),
    /// No Markdown files matched the provided sources / patterns.
    #[error("no Markdown files matched the given sources")]
    NoMatchingInputs,
    /// A positional or `--grep` glob pattern could not be parsed.
    #[error("invalid glob pattern: {0}")]
    InvalidGrep(String),
    /// The `--name-format` template contains an unknown placeholder.
    #[error("invalid --name-format: {0}")]
    InvalidNameFormat(String),
    /// Two inputs expanded to the same output path or zip entry name.
    #[error("name-format collision for `{0}`; include {{index}} to disambiguate")]
    NameCollision(String),
    /// `--jobs` was zero.
    #[error("--jobs must be at least 1")]
    InvalidJobs,
    /// The requested page margin is outside the supported range.
    #[error("--margin must be between 8 and 45 mm")]
    InvalidMargin,
    /// The accent color is not a hexadecimal `#RRGGBB` value.
    #[error("--accent must be a valid #RRGGBB color")]
    InvalidAccent,
    /// Loading a bundled grammar or highlighting a code block failed.
    #[error("syntax highlighting failed: {0}")]
    Highlight(String),
    /// Rendering a fenced Mermaid diagram failed.
    #[error("Mermaid diagram failed: {0}")]
    Mermaid(String),
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
    /// Building a zip archive failed.
    #[error("failed to create zip archive {path}: {message}")]
    Zip {
        /// Archive path that could not be written.
        path: PathBuf,
        /// Underlying error message.
        message: String,
    },
    /// Typst compilation or PDF serialization failed.
    #[error("PDF generation failed: {0}")]
    Pdf(String),
}

/// Convenient result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
