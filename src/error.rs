use std::path::PathBuf;

/// Errors returned while reading Markdown, validating options, or writing PDF.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The requested Markdown source does not exist.
    #[error("fichier Markdown introuvable : {0}")]
    InputNotFound(PathBuf),
    /// The Markdown source contains no meaningful content.
    #[error("le document Markdown est vide")]
    EmptyInput,
    /// Standard input was used without an explicit output path.
    #[error("--output est obligatoire avec l'entrée standard")]
    OutputRequiredForStdin,
    /// Positional input and the legacy `--input` option were both supplied.
    #[error("utilisez soit SOURCE, soit --input, pas les deux")]
    ConflictingInputs,
    /// No Markdown source was supplied.
    #[error("indiquez un fichier Markdown source")]
    MissingInput,
    /// The requested page margin is outside the supported range.
    #[error("--margin doit être compris entre 8 et 45 mm")]
    InvalidMargin,
    /// The accent color is not a hexadecimal `#RRGGBB` value.
    #[error("--accent doit être une couleur #RRGGBB valide")]
    InvalidAccent,
    /// Reading the Markdown source failed.
    #[error("lecture impossible de {path}: {source}")]
    Read {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying filesystem error.
        source: std::io::Error,
    },
    /// Creating the output directory or writing the PDF failed.
    #[error("écriture impossible de {path}: {source}")]
    Write {
        /// Path that could not be written.
        path: PathBuf,
        /// Underlying filesystem error.
        source: std::io::Error,
    },
    /// Typst compilation or PDF serialization failed.
    #[error("génération PDF impossible : {0}")]
    Pdf(String),
}

/// Convenient result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
