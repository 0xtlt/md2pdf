use std::path::PathBuf;

use clap::{Parser, ValueEnum};

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
    about = "Convertit du Markdown en PDF net avec coloration syntaxique."
)]
pub struct Cli {
    /// Markdown source, ou - pour stdin
    pub source: Option<PathBuf>,

    /// Ancienne syntaxe compatible
    #[arg(short = 'i', long = "input", hide = true)]
    pub legacy_source: Option<PathBuf>,

    /// PDF de sortie
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Titre des métadonnées PDF
    #[arg(long)]
    pub title: Option<String>,

    /// Auteur des métadonnées PDF
    #[arg(long, default_value = "")]
    pub author: String,

    /// Texte d'en-tête
    #[arg(long, default_value = "TECHNICAL DOCUMENTATION")]
    pub label: String,

    /// Texte de pied de page
    #[arg(long)]
    pub footer: Option<String>,

    /// Format de page
    #[arg(long, value_enum, default_value_t = PageSize::A4)]
    pub page_size: PageSize,

    /// Orientation paysage
    #[arg(long)]
    pub landscape: bool,

    /// Marges en millimètres
    #[arg(long, default_value_t = 17.0)]
    pub margin: f32,

    /// Couleur d'accentuation
    #[arg(long, default_value = "#C94C35")]
    pub accent: String,

    /// Thème des blocs de code
    #[arg(long, value_enum, default_value_t = CodeTheme::Dark)]
    pub code_theme: CodeTheme,

    /// Numéroter les lignes de code
    #[arg(long)]
    pub line_numbers: bool,

    /// Masquer l'en-tête
    #[arg(long)]
    pub no_header: bool,

    /// Nouvelle page avant les titres ## commençant par PREFIX
    #[arg(long = "page-break-before", value_name = "PREFIX")]
    pub page_break_before: Vec<String>,

    /// Ne rien afficher en cas de succès
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
}
