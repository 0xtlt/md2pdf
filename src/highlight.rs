use shiki::{FontStyle, Highlighter};

use crate::{Error, Result, cli::CodeTheme};

/// A syntax-highlighted source fragment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StyledToken {
    /// Original source text covered by this token.
    pub content: String,
    /// Resolved foreground color as a CSS-compatible hexadecimal value.
    pub color: String,
    /// Whether the selected theme renders the token in bold.
    pub bold: bool,
    /// Whether the selected theme renders the token in italics.
    pub italic: bool,
    /// Whether the selected theme underlines the token.
    pub underline: bool,
    /// Whether the selected theme strikes through the token.
    pub strikethrough: bool,
}

/// Lazy TextMate highlighter for Liquid and its embedded web languages.
///
/// The precompiled grammar includes Liquid plus its HTML, CSS, JSON, and
/// JavaScript dependencies. Other languages stay on Typst's native Syntect
/// path so highlighted documents remain compact and fast.
pub struct SyntaxHighlighter {
    inner: Highlighter,
    theme_index: usize,
}

impl SyntaxHighlighter {
    /// Build a highlighter for the selected dark or light code theme.
    pub fn new(theme: CodeTheme) -> Result<Self> {
        let engine = shiki_macros::highlighter_engine! {
            languages: ["liquid"],
            themes: [
                ("dark", "github-dark-default"),
                ("light", "github-light-default"),
            ],
        };
        let theme_index = match theme {
            CodeTheme::Dark => 0,
            CodeTheme::Light => 1,
        };
        Ok(Self {
            inner: engine.highlighter(),
            theme_index,
        })
    }

    /// Highlight source code using a canonical language ID or alias.
    ///
    /// Unknown and empty identifiers intentionally fall back to plain text.
    pub fn highlight(&mut self, source: &str, language: &str) -> Result<Vec<Vec<StyledToken>>> {
        let language = normalize_language(language);
        let language = if self
            .inner
            .engine()
            .language_keys()
            .any(|candidate| candidate == language)
        {
            language.as_str()
        } else {
            "text"
        };
        let lines = self
            .inner
            .code_to_tokens_with_themes(source, language)
            .map_err(highlight_error)?;
        Ok(lines
            .into_iter()
            .map(|line| {
                let mut merged: Vec<StyledToken> = Vec::with_capacity(line.len());
                for token in line {
                    let style = token
                        .styles
                        .get(self.theme_index)
                        .expect("precompiled light and dark themes");
                    let styled = StyledToken {
                        content: token.content,
                        color: style.color.to_string(),
                        bold: style.font_style.contains(FontStyle::BOLD),
                        italic: style.font_style.contains(FontStyle::ITALIC),
                        underline: style.font_style.contains(FontStyle::UNDERLINE),
                        strikethrough: style.font_style.contains(FontStyle::STRIKETHROUGH),
                    };
                    if let Some(previous) = merged.last_mut()
                        && same_style(previous, &styled)
                    {
                        previous.content.push_str(&styled.content);
                    } else {
                        merged.push(styled);
                    }
                }
                merged
            })
            .collect())
    }
}

fn normalize_language(language: &str) -> String {
    language
        .trim()
        .trim_start_matches('.')
        .strip_prefix("language-")
        .unwrap_or(language.trim().trim_start_matches('.'))
        .to_ascii_lowercase()
}

fn same_style(left: &StyledToken, right: &StyledToken) -> bool {
    left.color == right.color
        && left.bold == right.bold
        && left.italic == right.italic
        && left.underline == right.underline
        && left.strikethrough == right.strikethrough
}

fn highlight_error(error: shiki::Error) -> Error {
    Error::Highlight(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_web_and_system_languages() {
        let highlighter = SyntaxHighlighter::new(CodeTheme::Dark).expect("bundled catalog");
        assert!(
            highlighter
                .inner
                .engine()
                .language_keys()
                .any(|language| language == "liquid")
        );
        for required in [
            "bash",
            "c",
            "cpp",
            "css",
            "go",
            "html",
            "java",
            "json",
            "python",
            "rust",
            "sql",
            "toml",
            "typescript",
            "yaml",
        ] {
            assert!(
                typst::text::RAW_SYNTAXES
                    .find_syntax_by_token(required)
                    .is_some(),
                "Typst catalog is missing {required}"
            );
        }
    }

    #[test]
    fn highlights_liquid_with_embedded_html() {
        let mut highlighter = SyntaxHighlighter::new(CodeTheme::Dark).expect("bundled catalog");
        let lines = highlighter
            .highlight(
                "<h1 class=\"product\">{{ product.title | escape }}</h1>",
                "liquid",
            )
            .expect("valid Liquid grammar");
        let colors = lines
            .iter()
            .flatten()
            .map(|token| token.color.as_str())
            .collect::<std::collections::HashSet<_>>();
        assert!(colors.len() >= 3);
    }

    #[test]
    fn unknown_languages_fall_back_to_plain_text() {
        let mut highlighter = SyntaxHighlighter::new(CodeTheme::Light).expect("bundled catalog");
        let lines = highlighter
            .highlight("arbitrary <content>", "made-up-language")
            .expect("plain-text fallback");
        assert_eq!(
            lines
                .iter()
                .flatten()
                .map(|token| token.content.as_str())
                .collect::<String>(),
            "arbitrary <content>"
        );
    }
}
