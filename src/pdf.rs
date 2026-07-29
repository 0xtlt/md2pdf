use std::{fs, path::Path};

use typst_as_lib::TypstEngine;
use typst_layout::PagedDocument;

use crate::{Error, Result};

const FONTS: [&[u8]; 6] = [
    include_bytes!("../assets/fonts/DejaVuSans.ttf"),
    include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf"),
    include_bytes!("../assets/fonts/DejaVuSans-Oblique.ttf"),
    include_bytes!("../assets/fonts/DejaVuSans-BoldOblique.ttf"),
    include_bytes!("../assets/fonts/DejaVuSansMono.ttf"),
    include_bytes!("../assets/fonts/DejaVuSansMono-Bold.ttf"),
];
const DARK_THEME: &[u8] = include_bytes!("../assets/themes/md2pdf-dark.tmTheme");

/// Compile Typst source and write the resulting PDF.
///
/// Relative images and other local assets are resolved from `source_dir`.
/// Virtual `assets` (for example rendered Mermaid SVGs) are resolved in memory.
/// The returned value is the generated page count.
pub fn render(
    source: &str,
    output: &Path,
    source_dir: &Path,
    assets: &[(String, Vec<u8>)],
) -> Result<usize> {
    let mut binaries: Vec<(&str, &[u8])> = Vec::with_capacity(assets.len() + 1);
    binaries.push(("md2pdf-dark.tmTheme", DARK_THEME));
    for (name, bytes) in assets {
        binaries.push((name.as_str(), bytes.as_slice()));
    }

    let engine = TypstEngine::builder()
        .main_file(source)
        .fonts(FONTS)
        .with_static_file_resolver(binaries)
        .with_file_system_resolver(source_dir)
        .build();
    let result = engine.compile::<PagedDocument>();
    let document = result
        .output
        .map_err(|error| Error::Pdf(humanize_typst_debug(&format!("{error:?}"))))?;
    drop(engine);
    let page_count = document.pages().len();
    let bytes = typst_pdf::pdf(&document, &Default::default())
        .map_err(|error| Error::Pdf(humanize_typst_debug(&format!("{error:?}"))))?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(output, bytes).map_err(|source| Error::Write {
        path: output.to_path_buf(),
        source,
    })?;
    Ok(page_count)
}

fn humanize_typst_debug(debug: &str) -> String {
    let messages = diagnostic_messages(debug);
    if messages.is_empty() {
        return "unexpected PDF engine failure".to_owned();
    }
    let network_hint = debug.contains("network access is not supported");
    messages
        .into_iter()
        .map(|message| simplify_diagnostic_message(&message, network_hint))
        .collect::<Vec<_>>()
        .join("; ")
}

fn diagnostic_messages(debug: &str) -> Vec<String> {
    let mut messages = Vec::new();
    let mut rest = debug;
    while let Some(start) = rest.find("message: \"") {
        let after = &rest[start + "message: \"".len()..];
        let Some(end) = find_unescaped_quote(after) else {
            break;
        };
        messages.push(unescape_debug_string(&after[..end]));
        rest = &after[end + 1..];
    }
    messages
}

fn find_unescaped_quote(value: &str) -> Option<usize> {
    let bytes = value.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = index.saturating_add(2),
            b'"' => return Some(index),
            _ => index += 1,
        }
    }
    None
}

fn unescape_debug_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some('\\') => output.push('\\'),
            Some('"') => output.push('"'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}

fn simplify_diagnostic_message(message: &str, network_hint: bool) -> String {
    let trimmed = message.trim();
    if let Some(path) = searched_path(trimmed) {
        if network_hint || looks_like_remote_path(path) {
            return format!("cannot load remote image `{path}` (PDF engine has no network access)");
        }
        return format!("file not found: {path}");
    }
    if network_hint || trimmed.contains("network access is not supported") {
        return "cannot load remote image (PDF engine has no network access)".to_owned();
    }
    trimmed.to_owned()
}

fn looks_like_remote_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with("https:") || lower.starts_with("http:") || lower.starts_with("//")
}

fn searched_path(message: &str) -> Option<&str> {
    let start = message.find("(searched at ")? + "(searched at ".len();
    let end = message[start..].find(')')? + start;
    Some(message[start..end].trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanizes_file_not_found_diagnostics() {
        let debug = r#"TypstSource([SourceDiagnostic { severity: Error, message: "file not found (searched at docs/missing.png)", hints: [] }])"#;
        assert_eq!(
            humanize_typst_debug(debug),
            "file not found: docs/missing.png"
        );
    }

    #[test]
    fn humanizes_network_access_diagnostics() {
        let debug = r#"TypstSource([SourceDiagnostic { severity: Error, message: "file not found (searched at https:/example.com/a.png)", hints: ["network access is not supported"] }])"#;
        assert_eq!(
            humanize_typst_debug(debug),
            "cannot load remote image `https:/example.com/a.png` (PDF engine has no network access)"
        );
    }
}
