use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum};

/// Supported output page formats.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PageSize {
    /// ISO A4 (210 × 297 mm).
    A4,
    /// US Letter (8.5 × 11 inches).
    Letter,
}

/// Available syntax-highlighting color schemes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CodeTheme {
    /// Dark background with high-contrast syntax colors.
    Dark,
    /// Light background using Typst's built-in highlighting.
    Light,
}

/// Parsed command-line arguments.
#[derive(Debug, Parser)]
#[command(
    name = "md2pdf",
    version,
    about = "Converts Markdown into a polished PDF with syntax highlighting."
)]
pub struct Cli {
    /// Markdown source, or - for standard input
    pub source: Option<PathBuf>,

    /// Legacy input syntax kept for compatibility
    #[arg(short = 'i', long = "input", hide = true)]
    pub legacy_source: Option<PathBuf>,

    /// Output PDF path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// PDF metadata title
    #[arg(long)]
    pub title: Option<String>,

    /// PDF metadata author
    #[arg(long, default_value = "")]
    pub author: String,

    /// Page header text
    #[arg(long, default_value = "TECHNICAL DOCUMENTATION")]
    pub label: String,

    /// Page footer text
    #[arg(long)]
    pub footer: Option<String>,

    /// Page format
    #[arg(long, value_enum, default_value_t = PageSize::A4)]
    pub page_size: PageSize,

    /// Use landscape orientation
    #[arg(long)]
    pub landscape: bool,

    /// Page margins in millimetres
    #[arg(long, default_value_t = 17.0)]
    pub margin: f32,

    /// Accent color
    #[arg(long, default_value = "#C94C35")]
    pub accent: String,

    /// Code block theme
    #[arg(long, value_enum, default_value_t = CodeTheme::Dark)]
    pub code_theme: CodeTheme,

    /// Add line numbers to code blocks
    #[arg(long)]
    pub line_numbers: bool,

    /// Hide the page header
    #[arg(long)]
    pub no_header: bool,

    /// Start a new page before ## headings beginning with PREFIX
    #[arg(long = "page-break-before", value_name = "PREFIX")]
    pub page_break_before: Vec<String>,

    /// Do not download remote images or other external HTTP(S) data
    #[arg(long = "no-external", action = ArgAction::SetTrue)]
    pub no_external: bool,

    /// Allow cleartext http:// image downloads (https:// only by default)
    #[arg(long = "allow-http", action = ArgAction::SetTrue)]
    pub allow_http: bool,

    /// Suppress success output and non-fatal warnings
    #[arg(short, long)]
    pub quiet: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_numbers_are_disabled_by_default() {
        let cli = Cli::try_parse_from(["md2pdf", "document.md"]).expect("valid arguments");
        assert!(!cli.line_numbers);
    }

    #[test]
    fn external_data_is_allowed_by_default() {
        let cli = Cli::try_parse_from(["md2pdf", "document.md"]).expect("valid arguments");
        assert!(!cli.no_external);
        assert!(!cli.allow_http);
        let denied = Cli::try_parse_from(["md2pdf", "document.md", "--no-external"])
            .expect("valid arguments");
        assert!(denied.no_external);
        let insecure = Cli::try_parse_from(["md2pdf", "document.md", "--allow-http"])
            .expect("valid arguments");
        assert!(insecure.allow_http);
    }
}
