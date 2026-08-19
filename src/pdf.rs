use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use lopdf::{Document as PdfDocument, Object, ObjectId};
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

/// Compile Typst source and return the serialized PDF bytes plus page count.
///
/// Relative images and other local assets are resolved from `source_dir`.
/// Virtual `assets` (for example rendered Mermaid SVGs) are resolved in memory.
pub fn render_to_bytes(
    source: &str,
    source_dir: &Path,
    assets: &[(String, Vec<u8>)],
) -> Result<(Vec<u8>, usize)> {
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
    Ok((bytes, page_count))
}

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
    let (bytes, page_count) = render_to_bytes(source, source_dir, assets)?;
    write_bytes(output, &bytes)?;
    Ok(page_count)
}

/// Write raw bytes to `output`, creating parent directories as needed.
pub fn write_bytes(output: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(output, bytes).map_err(|source| Error::Write {
        path: output.to_path_buf(),
        source,
    })
}

/// Concatenate PDF documents in order into a single PDF.
pub fn merge_pdfs(parts: &[Vec<u8>]) -> Result<Vec<u8>> {
    let parts = parts
        .iter()
        .map(|bytes| MergePart {
            bytes,
            source_path: None,
        })
        .collect::<Vec<_>>();
    merge_pdf_parts(&parts)
}

/// Concatenate PDFs rendered from Markdown and turn links between included
/// source files into internal destinations in the merged document.
pub fn merge_markdown_pdfs(parts: &[(&Path, &[u8])]) -> Result<Vec<u8>> {
    let parts = parts
        .iter()
        .map(|(source_path, bytes)| MergePart {
            bytes,
            source_path: Some(*source_path),
        })
        .collect::<Vec<_>>();
    merge_pdf_parts(&parts)
}

struct MergePart<'a> {
    bytes: &'a [u8],
    source_path: Option<&'a Path>,
}

fn merge_pdf_parts(parts: &[MergePart<'_>]) -> Result<Vec<u8>> {
    if parts.is_empty() {
        return Err(Error::Pdf("no PDF parts to merge".to_owned()));
    }
    if parts.len() == 1 {
        return Ok(parts[0].bytes.to_vec());
    }

    let mut max_id = 1u32;
    let mut documents_pages = BTreeMap::new();
    let mut documents_objects = BTreeMap::new();
    let mut page_sources = HashMap::new();
    let mut target_pages = HashMap::new();
    let mut document = PdfDocument::with_version("1.5");

    for part in parts {
        let mut doc = PdfDocument::load_mem(part.bytes)
            .map_err(|error| Error::Pdf(format!("failed to parse PDF part: {error}")))?;
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        let pages = doc.get_pages();
        let mut page_numbers: Vec<_> = pages.keys().copied().collect();
        page_numbers.sort_unstable();
        let source_path = part.source_path.map(absolute_source_path);
        if let (Some(source_path), Some(page_number)) = (source_path.as_ref(), page_numbers.first())
        {
            target_pages.insert(normalize_source_path(source_path), pages[page_number]);
        }
        for page_number in page_numbers {
            let object_id = pages[&page_number];
            let object = doc
                .get_object(object_id)
                .map_err(|error| Error::Pdf(format!("failed to read PDF page: {error}")))?
                .clone();
            documents_pages.insert(object_id, object);
            if let Some(source_path) = &source_path {
                page_sources.insert(object_id, source_path.clone());
            }
        }
        documents_objects.extend(doc.objects);
    }

    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in documents_objects {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" => {
                catalog_object = Some((catalog_object.map_or(object_id, |(id, _)| id), object));
            }
            b"Pages" => {
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref existing)) = pages_object
                        && let Ok(old_dictionary) = existing.as_dict()
                    {
                        dictionary.extend(old_dictionary);
                    }
                    pages_object = Some((
                        pages_object.map_or(object_id, |(id, _)| id),
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            b"Page" | b"Outlines" | b"Outline" => {}
            _ => {
                document.objects.insert(object_id, object);
            }
        }
    }

    let (pages_id, pages_obj) = pages_object
        .ok_or_else(|| Error::Pdf("Pages root not found while merging PDFs".to_owned()))?;
    let (catalog_id, catalog_obj) = catalog_object
        .ok_or_else(|| Error::Pdf("Catalog root not found while merging PDFs".to_owned()))?;

    for (object_id, object) in &documents_pages {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_id);
            document
                .objects
                .insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    rewrite_internal_markdown_links(&mut document, &page_sources, &target_pages);

    if let Ok(dictionary) = pages_obj.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Count", documents_pages.len() as u32);
        dictionary.set(
            "Kids",
            documents_pages
                .into_keys()
                .map(Object::Reference)
                .collect::<Vec<_>>(),
        );
        document
            .objects
            .insert(pages_id, Object::Dictionary(dictionary));
    }

    if let Ok(dictionary) = catalog_obj.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_id);
        dictionary.remove(b"Outlines");
        document
            .objects
            .insert(catalog_id, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_id);
    document.max_id = document.objects.len() as u32;
    document.renumber_objects();

    let mut output = Cursor::new(Vec::new());
    document
        .save_to(&mut output)
        .map_err(|error| Error::Pdf(format!("failed to serialize merged PDF: {error}")))?;
    Ok(output.into_inner())
}

fn rewrite_internal_markdown_links(
    document: &mut PdfDocument,
    page_sources: &HashMap<ObjectId, PathBuf>,
    target_pages: &HashMap<PathBuf, ObjectId>,
) {
    for (page_id, source_path) in page_sources {
        let annotation_ids = page_annotation_ids(document, *page_id);
        for annotation_id in annotation_ids {
            let Some(uri) = annotation_uri(document, annotation_id) else {
                continue;
            };
            let Some(target_page) = markdown_link_target(&uri, source_path, target_pages) else {
                continue;
            };
            let Ok(annotation) = document.get_dictionary_mut(annotation_id) else {
                continue;
            };
            let mut action = lopdf::Dictionary::new();
            action.set("S", "GoTo");
            action.set(
                "D",
                vec![
                    Object::Reference(target_page),
                    Object::Name(b"Fit".to_vec()),
                ],
            );
            annotation.set("A", action);
            annotation.remove(b"Contents");
        }
    }
}

fn page_annotation_ids(document: &PdfDocument, page_id: ObjectId) -> Vec<ObjectId> {
    let Ok(page) = document.get_dictionary(page_id) else {
        return Vec::new();
    };
    let Ok(annotations) = page.get(b"Annots") else {
        return Vec::new();
    };
    let annotations = match annotations {
        Object::Array(annotations) => annotations,
        Object::Reference(id) => match document.get_object(*id).and_then(Object::as_array) {
            Ok(annotations) => annotations,
            Err(_) => return Vec::new(),
        },
        _ => return Vec::new(),
    };
    annotations
        .iter()
        .filter_map(|annotation| annotation.as_reference().ok())
        .collect()
}

fn annotation_uri(document: &PdfDocument, annotation_id: ObjectId) -> Option<String> {
    let annotation = document.get_dictionary(annotation_id).ok()?;
    let action = match annotation.get(b"A").ok()? {
        Object::Dictionary(action) => action,
        Object::Reference(id) => document.get_dictionary(*id).ok()?,
        _ => return None,
    };
    if action.get(b"S").and_then(Object::as_name).ok()? != b"URI" {
        return None;
    }
    String::from_utf8(action.get(b"URI").and_then(Object::as_str).ok()?.to_vec()).ok()
}

fn markdown_link_target(
    uri: &str,
    source_path: &Path,
    target_pages: &HashMap<PathBuf, ObjectId>,
) -> Option<ObjectId> {
    let path = uri.split(['#', '?']).next()?;
    if path.is_empty() || has_uri_scheme(path) {
        return None;
    }
    let path = percent_decode_path(path)?;
    if !Path::new(&path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        return None;
    }
    let target_path = source_path.parent()?.join(path);
    target_pages
        .get(&normalize_source_path(&target_path))
        .copied()
}

fn has_uri_scheme(value: &str) -> bool {
    let Some(colon) = value.find(':') else {
        return false;
    };
    let scheme = &value[..colon];
    !scheme.is_empty()
        && scheme.as_bytes()[0].is_ascii_alphabetic()
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

fn percent_decode_path(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        let high = hex_value(*bytes.get(index + 1)?)?;
        let low = hex_value(*bytes.get(index + 2)?)?;
        decoded.push((high << 4) | low);
        index += 3;
    }
    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn normalize_source_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    })
}

fn absolute_source_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
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
