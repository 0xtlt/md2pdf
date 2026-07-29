//! Download and validate remote Markdown images for embedding in Typst.

use std::io::Read;
use std::time::Duration;

const MAX_BYTES: u64 = 5 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_REDIRECTS: u32 = 5;

/// Image formats Typst can embed from downloaded bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageKind {
    /// Portable Network Graphics.
    Png,
    /// JPEG / JFIF.
    Jpeg,
    /// Graphics Interchange Format.
    Gif,
    /// Scalable Vector Graphics.
    Svg,
}

impl ImageKind {
    /// File extension used for the virtual Typst asset name.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::Svg => "svg",
        }
    }
}

/// Returns true when `url` is an absolute HTTP(S) or protocol-relative image URL.
#[must_use]
pub fn is_remote_image_url(url: &str) -> bool {
    normalize_remote_url(url).is_some()
}

/// Returns true when the URL uses cleartext `http://` (after normalization).
#[must_use]
pub fn is_insecure_http_url(url: &str) -> bool {
    normalize_remote_url(url)
        .map(|normalized| normalized.to_ascii_lowercase().starts_with("http://"))
        .unwrap_or(false)
}

/// Normalize a remote image URL, rewriting `//host/path` to `https://host/path`.
#[must_use]
pub fn normalize_remote_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("//") {
        return Some(format!("https:{trimmed}"));
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        Some(trimmed.to_owned())
    } else {
        None
    }
}

/// Prepare a remote URL for download, rejecting cleartext HTTP unless allowed.
pub fn prepare_download_url(url: &str, allow_http: bool) -> Result<String, String> {
    let normalized =
        normalize_remote_url(url).ok_or_else(|| "not a remote http(s) URL".to_owned())?;
    if !allow_http && normalized.to_ascii_lowercase().starts_with("http://") {
        return Err(
            "insecure http:// images are blocked; use https:// or pass --allow-http".to_owned(),
        );
    }
    Ok(normalized)
}

/// Infer an image kind from magic bytes, optional Content-Type, and URL path.
#[must_use]
pub fn detect_image_kind(bytes: &[u8], content_type: Option<&str>, url: &str) -> Option<ImageKind> {
    if let Some(kind) = detect_from_bytes(bytes) {
        return Some(kind);
    }
    if let Some(kind) = content_type.and_then(detect_from_content_type) {
        return Some(kind);
    }
    detect_from_url(url)
}

/// Download a remote image and return `(extension, bytes)` or a short error message.
///
/// Cleartext `http://` URLs are rejected unless `allow_http` is true. Redirects to
/// `http://` are also rejected when `allow_http` is false.
pub fn download_image(url: &str, allow_http: bool) -> Result<(String, Vec<u8>), String> {
    let url = prepare_download_url(url, allow_http)?;
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(TIMEOUT)
        .timeout_read(TIMEOUT)
        .redirects(MAX_REDIRECTS)
        .build();
    let response = agent
        .get(&url)
        .set("User-Agent", "md2pdf")
        .call()
        .map_err(|error| format!("download failed: {error}"))?;
    let final_url = response.get_url().to_owned();
    if !allow_http && final_url.to_ascii_lowercase().starts_with("http://") {
        return Err(
            "insecure http:// redirect is blocked; use https:// or pass --allow-http".to_owned(),
        );
    }
    let status = response.status();
    if !(200..300).contains(&status) {
        return Err(format!("download failed: HTTP {status}"));
    }
    let content_type = response.header("Content-Type").map(str::to_owned);
    let mut limited = response.into_reader().take(MAX_BYTES + 1);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|error| format!("download failed: {error}"))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(format!(
            "download failed: image larger than {} bytes",
            MAX_BYTES
        ));
    }
    if bytes.is_empty() {
        return Err("download failed: empty response".to_owned());
    }
    let kind = detect_image_kind(&bytes, content_type.as_deref(), &final_url).ok_or_else(|| {
        "download failed: response is not a supported image (png, jpeg, gif, svg)".to_owned()
    })?;
    Ok((kind.extension().to_owned(), bytes))
}

fn detect_from_bytes(bytes: &[u8]) -> Option<ImageKind> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']) {
        return Some(ImageKind::Png);
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some(ImageKind::Jpeg);
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(ImageKind::Gif);
    }
    let trimmed = trim_bom_and_whitespace(bytes);
    let prefix = std::str::from_utf8(trimmed.get(..256.min(trimmed.len()))?).ok()?;
    let lower = prefix.to_ascii_lowercase();
    if lower.contains("<svg") {
        return Some(ImageKind::Svg);
    }
    None
}

fn detect_from_content_type(content_type: &str) -> Option<ImageKind> {
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    match mime.as_str() {
        "image/png" => Some(ImageKind::Png),
        "image/jpeg" | "image/jpg" => Some(ImageKind::Jpeg),
        "image/gif" => Some(ImageKind::Gif),
        "image/svg+xml" => Some(ImageKind::Svg),
        _ => None,
    }
}

fn detect_from_url(url: &str) -> Option<ImageKind> {
    let path = url.split('?').next().unwrap_or(url);
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some(ImageKind::Png),
        "jpg" | "jpeg" => Some(ImageKind::Jpeg),
        "gif" => Some(ImageKind::Gif),
        "svg" => Some(ImageKind::Svg),
        _ => None,
    }
}

fn trim_bom_and_whitespace(bytes: &[u8]) -> &[u8] {
    let bytes = if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        &bytes[3..]
    } else {
        bytes
    };
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[start..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_protocol_relative_urls() {
        assert_eq!(
            normalize_remote_url("//img.shields.io/badge/x.svg").as_deref(),
            Some("https://img.shields.io/badge/x.svg")
        );
        assert!(normalize_remote_url("docs/a.png").is_none());
        assert!(is_remote_image_url("HTTPS://example.com/a.PNG"));
        assert!(is_insecure_http_url("http://example.com/a.png"));
        assert!(!is_insecure_http_url("https://example.com/a.png"));
        assert!(!is_insecure_http_url("//example.com/a.png"));
    }

    #[test]
    fn rejects_cleartext_http_unless_allowed() {
        assert!(
            prepare_download_url("http://example.com/a.png", false)
                .unwrap_err()
                .contains("--allow-http")
        );
        assert_eq!(
            prepare_download_url("http://example.com/a.png", true).as_deref(),
            Ok("http://example.com/a.png")
        );
        assert_eq!(
            prepare_download_url("https://example.com/a.png", false).as_deref(),
            Ok("https://example.com/a.png")
        );
    }

    #[test]
    fn detects_image_kinds_from_magic_bytes() {
        assert_eq!(
            detect_image_kind(b"\x89PNG\r\n\x1a\nrest", None, "x.bin"),
            Some(ImageKind::Png)
        );
        assert_eq!(
            detect_image_kind(b"\xff\xd8\xff\xe0rest", None, "x.bin"),
            Some(ImageKind::Jpeg)
        );
        assert_eq!(
            detect_image_kind(b"GIF89a....", None, "x.bin"),
            Some(ImageKind::Gif)
        );
        assert_eq!(
            detect_image_kind(
                b"  <?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"/>",
                None,
                "x.bin"
            ),
            Some(ImageKind::Svg)
        );
    }
}
