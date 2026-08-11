use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

use lopdf::Document as PdfDocument;
use tempfile::tempdir;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_md2pdf"))
}

#[test]
fn creates_a_pdf_from_a_positional_source() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("guide.md");
    let output = directory.path().join("result.pdf");
    fs::write(
        &source,
        "# Guide\n\n```rust\nfn main() { println!(\"net\"); }\n```\n",
    )
    .expect("write markdown");

    let result = binary()
        .arg(&source)
        .args(["--line-numbers", "--output"])
        .arg(&output)
        .output()
        .expect("run md2pdf");

    assert!(result.status.success(), "{:?}", result);
    let pdf = fs::read(output).expect("read PDF");
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(pdf.len() > 10_000);
}

#[test]
fn creates_a_three_page_widescreen_slide_deck() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("deck.md");
    let output = directory.path().join("deck.pdf");
    fs::write(
        &source,
        "# Widescreen deck\n\nOpening slide.\n\n---\n\n## Architecture\n\n- Parser\n- Typst\n\n---\n\n## Demo\n\n```rust\nfn main() {}\n```\n",
    )
    .expect("write slide deck");

    let result = binary()
        .arg(&source)
        .args(["--slides", "--output"])
        .arg(&output)
        .output()
        .expect("run md2pdf");

    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!String::from_utf8_lossy(&result.stderr).contains("warning"));
    let pdf = PdfDocument::load(&output).expect("load slide PDF");
    let pages = pdf.get_pages();
    assert_eq!(pages.len(), 3);
    for page_id in pages.values() {
        let page = pdf
            .get_object(*page_id)
            .and_then(lopdf::Object::as_dict)
            .expect("page dictionary");
        let media_box = page
            .get(b"MediaBox")
            .and_then(lopdf::Object::as_array)
            .expect("page media box");
        let width = media_box[2].as_float().expect("page width")
            - media_box[0].as_float().expect("page x origin");
        let height = media_box[3].as_float().expect("page height")
            - media_box[1].as_float().expect("page y origin");
        assert!((width / height - 16.0 / 9.0).abs() < 0.001);
    }
}

#[test]
fn renders_every_builtin_slide_template() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("templates.md");
    fs::write(
        &source,
        "# Template preview\n\nCentered cover copy.\n\n---\n\n## Content\n\n- Clear hierarchy\n- Consistent spacing\n",
    )
    .expect("write slide deck");

    for template in ["modern", "minimal", "dark"] {
        let output = directory.path().join(format!("{template}.pdf"));
        let result = binary()
            .arg(&source)
            .args(["--slides", "--slide-template", template, "--output"])
            .arg(&output)
            .output()
            .expect("run md2pdf");

        assert!(
            result.status.success(),
            "template={template}, stderr={}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            PdfDocument::load(output)
                .expect("load template PDF")
                .get_pages()
                .len(),
            2
        );
    }
}

#[test]
fn keeps_slide_images_within_large_margins() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("image-deck.md");
    let image = directory.path().join("test.svg");
    fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("test.svg"),
        &image,
    )
    .expect("copy image fixture");
    fs::write(
        &source,
        "# Image deck\n\nCover.\n\n---\n\n## Diagram\n\n![Test](test.svg)\n",
    )
    .expect("write slide deck");

    for margin in [35, 40, 45] {
        let output = directory.path().join(format!("margin-{margin}.pdf"));
        let result = binary()
            .arg(&source)
            .arg("--slides")
            .arg("--margin")
            .arg(margin.to_string())
            .arg("--output")
            .arg(&output)
            .output()
            .expect("render image slide");

        assert!(
            result.status.success(),
            "margin={margin}, stderr={}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            PdfDocument::load(output)
                .expect("load image slide PDF")
                .get_pages()
                .len(),
            2,
            "margin={margin}"
        );
    }
}

#[test]
fn keeps_a_tall_mermaid_pipeline_on_its_markdown_slide() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("tall-pipeline.md");
    let output = directory.path().join("tall-pipeline.pdf");
    fs::write(
        &source,
        "## Tall pipeline\n\n```mermaid\nflowchart TD\n    A1 --> A2 --> A3 --> A4 --> A5\n    A5 --> A6 --> A7 --> A8 --> A9 --> A10\n```\n",
    )
    .expect("write tall Mermaid slide");

    let result = binary()
        .arg(&source)
        .args(["--slides", "--output"])
        .arg(&output)
        .output()
        .expect("render tall Mermaid slide");

    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        PdfDocument::load(output)
            .expect("load tall Mermaid slide")
            .get_pages()
            .len(),
        1,
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn warns_when_a_markdown_slide_overflows_onto_extra_pages() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("overflow.md");
    let output = directory.path().join("overflow.pdf");
    let paragraphs = (1..=80)
        .map(|index| format!("Paragraph {index} contains enough text to occupy vertical space."))
        .collect::<Vec<_>>()
        .join("\n\n");
    fs::write(&source, format!("# Overflow\n\n{paragraphs}\n")).expect("write overflowing slide");

    let result = binary()
        .arg(&source)
        .args(["--slides", "--output"])
        .arg(&output)
        .output()
        .expect("run md2pdf");

    assert!(result.status.success(), "{:?}", result);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("Markdown slides"), "stderr: {stderr}");
    assert!(stderr.contains("shorten content"), "stderr: {stderr}");
    assert!(
        PdfDocument::load(output)
            .expect("load PDF")
            .get_pages()
            .len()
            > 1
    );
}

#[test]
fn preserves_an_empty_slide_between_consecutive_separators() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("empty-slide.md");
    let output = directory.path().join("empty-slide.pdf");
    fs::write(&source, "# First slide\n\n---\n\n---\n\n## Third slide\n")
        .expect("write slide deck");

    let result = binary()
        .arg(&source)
        .args(["--slides", "--output"])
        .arg(&output)
        .output()
        .expect("run md2pdf");

    assert!(result.status.success(), "{:?}", result);
    assert!(
        !String::from_utf8_lossy(&result.stderr).contains("warning"),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        PdfDocument::load(output)
            .expect("load PDF")
            .get_pages()
            .len(),
        3
    );
}

#[test]
fn rejects_document_page_options_in_slide_mode() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("deck.md");
    fs::write(&source, "# Deck\n").expect("write slide deck");

    let result = binary()
        .arg(&source)
        .args(["--slides", "--page-size", "letter"])
        .output()
        .expect("run md2pdf");

    assert_eq!(result.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("cannot be used with"), "stderr: {stderr}");
}

#[test]
fn supports_the_legacy_input_option_and_default_output() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("legacy.md");
    fs::write(&source, "# Compatible\n\nText.").expect("write markdown");

    let result = binary()
        .arg("--input")
        .arg(&source)
        .arg("--quiet")
        .output()
        .expect("run md2pdf");

    assert!(result.status.success(), "{:?}", result);
    assert!(source.with_extension("pdf").is_file());
}

#[test]
fn rejects_invalid_options_without_creating_a_pdf() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("invalid.md");
    let output = directory.path().join("invalid.pdf");
    fs::write(&source, "# Test").expect("write markdown");

    let result = binary()
        .arg(&source)
        .args(["--margin", "2", "--output"])
        .arg(&output)
        .output()
        .expect("run md2pdf");

    assert_eq!(result.status.code(), Some(2));
    assert!(!output.exists());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--margin"));
}

#[test]
fn creates_a_pdf_from_standard_input() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("stdin.pdf");
    let mut child = binary()
        .arg("-")
        .arg("--output")
        .arg(&output)
        .arg("--quiet")
        .stdin(Stdio::piped())
        .spawn()
        .expect("run md2pdf");

    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(b"# Standard input\n\nGenerated without a temporary Markdown file.")
        .expect("write stdin");

    assert!(child.wait().expect("wait for md2pdf").success());
    assert!(fs::read(output).expect("read PDF").starts_with(b"%PDF-"));
}

#[test]
fn renders_an_image_relative_to_the_markdown_source() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("features.pdf");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("features.md");

    let result = binary()
        .arg(fixture)
        .arg("--output")
        .arg(&output)
        .arg("--quiet")
        .output()
        .expect("run md2pdf");

    assert!(result.status.success(), "{:?}", result);
    assert!(fs::metadata(output).expect("PDF metadata").len() > 20_000);
}

#[test]
fn renders_the_multilanguage_syntax_catalog() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("syntax-catalog.pdf");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("syntax-catalog.md");

    let result = binary()
        .arg(fixture)
        .arg("--output")
        .arg(&output)
        .arg("--quiet")
        .output()
        .expect("run md2pdf");

    assert!(result.status.success(), "{:?}", result);
    assert!(fs::metadata(output).expect("PDF metadata").len() > 20_000);
}

#[test]
fn rejects_an_invalid_accent_color() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("accent.md");
    fs::write(&source, "# Test").expect("write markdown");

    let result = binary()
        .arg(source)
        .args(["--accent", "tomato"])
        .output()
        .expect("run md2pdf");

    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("--accent"));
}

#[test]
fn renders_mermaid_diagrams_to_pdf() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("mermaid.pdf");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mermaid.md");

    let result = binary()
        .arg(fixture)
        .arg("--output")
        .arg(&output)
        .arg("--quiet")
        .output()
        .expect("run md2pdf");

    assert!(result.status.success(), "{:?}", result);
    let pdf = fs::read(output).expect("read PDF");
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(pdf.len() > 15_000);
}

#[test]
fn rejects_invalid_mermaid_without_creating_a_pdf() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("broken-mermaid.md");
    let output = directory.path().join("broken-mermaid.pdf");
    fs::write(&source, "# Broken\n\n```mermaid\nnot a diagram\n```\n").expect("write markdown");

    let result = binary()
        .arg(&source)
        .arg("--output")
        .arg(&output)
        .output()
        .expect("run md2pdf");

    assert_eq!(result.status.code(), Some(2));
    assert!(!output.exists());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("Mermaid"));
    assert!(!stderr.contains("TypstSource"));
    assert!(!stderr.contains("SourceDiagnostic"));
}

#[test]
fn converts_readme_with_remote_badge_images() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("readme.pdf");
    let readme = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");

    let result = binary()
        .arg(&readme)
        .arg("--output")
        .arg(&output)
        .arg("--quiet")
        .output()
        .expect("run md2pdf");

    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(!stderr.contains("TypstSource"));
    assert!(!stderr.contains("SourceDiagnostic"));
    assert!(fs::read(output).expect("read PDF").starts_with(b"%PDF-"));
}

#[test]
fn converts_readme_without_external_downloads() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("readme-offline.pdf");
    let readme = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");

    let result = binary()
        .arg(&readme)
        .arg("--output")
        .arg(&output)
        .arg("--no-external")
        .output()
        .expect("run md2pdf");

    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("md2pdf: warning:"));
    assert!(stderr.contains("--no-external"));
    assert!(!stderr.contains("TypstSource"));
    assert!(!stderr.contains("SourceDiagnostic"));
    assert!(fs::read(output).expect("read PDF").starts_with(b"%PDF-"));
}

#[test]
fn warns_and_skips_cleartext_http_images_by_default() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("http-image.md");
    let output = directory.path().join("http-image.pdf");
    fs::write(
        &source,
        "# Insecure\n\n![Remote](http://example.com/diagram.png)\n\nText.\n",
    )
    .expect("write markdown");

    let result = binary()
        .arg(&source)
        .arg("--output")
        .arg(&output)
        .output()
        .expect("run md2pdf");

    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("md2pdf: warning:"));
    assert!(stderr.contains("--allow-http"));
    assert!(fs::read(output).expect("read PDF").starts_with(b"%PDF-"));
}

#[test]
fn writes_one_pdf_per_file_with_name_format() {
    let directory = tempdir().expect("temporary directory");
    let a = directory.path().join("alpha.md");
    let b = directory.path().join("beta.md");
    let out = directory.path().join("out");
    fs::write(&a, "# Alpha\n\nOne.\n").expect("write a");
    fs::write(&b, "# Beta\n\nTwo.\n").expect("write b");
    fs::create_dir_all(&out).expect("outdir");

    let result = binary()
        .arg(&a)
        .arg(&b)
        .args([
            "--output-mode",
            "files",
            "--name-format",
            "{stem}-{index}.pdf",
            "--jobs",
            "1",
            "--output",
        ])
        .arg(&out)
        .arg("--quiet")
        .output()
        .expect("run md2pdf");

    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(out.join("alpha-1.pdf").is_file());
    assert!(out.join("beta-2.pdf").is_file());
}

#[test]
fn greps_directory_and_merges_pdfs() {
    let directory = tempdir().expect("temporary directory");
    let nested = directory.path().join("docs");
    fs::create_dir_all(&nested).expect("docs");
    fs::write(nested.join("keep.md"), "# Keep\n\nPage one.\n").expect("keep");
    fs::write(nested.join("skip.txt"), "ignored").expect("skip");
    fs::write(nested.join("also.md"), "# Also\n\nPage two.\n").expect("also");
    let output = directory.path().join("merged.pdf");

    let result = binary()
        .arg(&nested)
        .args(["--grep", "**/*.md", "--output-mode", "merge", "--output"])
        .arg(&output)
        .arg("--quiet")
        .output()
        .expect("run md2pdf");

    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let pdf = fs::read(&output).expect("read merged");
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(pdf.len() > 5_000);
}

#[test]
fn packs_pdfs_into_a_zip_archive() {
    let directory = tempdir().expect("temporary directory");
    let a = directory.path().join("one.md");
    let b = directory.path().join("two.md");
    fs::write(&a, "# One\n\nText.\n").expect("write one");
    fs::write(&b, "# Two\n\nText.\n").expect("write two");
    let output = directory.path().join("docs.zip");

    let result = binary()
        .arg(&a)
        .arg(&b)
        .args([
            "--output-mode",
            "zip",
            "--name-format",
            "{stem}.pdf",
            "--output",
        ])
        .arg(&output)
        .arg("--quiet")
        .output()
        .expect("run md2pdf");

    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = fs::read(&output).expect("read zip");
    assert_eq!(&bytes[0..2], b"PK");
}

#[test]
fn rejects_merge_without_output() {
    let directory = tempdir().expect("temporary directory");
    let source = directory.path().join("doc.md");
    fs::write(&source, "# Doc\n").expect("write");

    let result = binary()
        .arg(&source)
        .args(["--output-mode", "merge"])
        .output()
        .expect("run md2pdf");

    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("--output"));
}

#[test]
fn expands_positional_glob_and_merges() {
    let directory = tempdir().expect("temporary directory");
    let nested = directory.path().join("docs");
    fs::create_dir_all(&nested).expect("docs");
    fs::write(nested.join("a.md"), "# A\n\nOne.\n").expect("a");
    fs::write(nested.join("b.md"), "# B\n\nTwo.\n").expect("b");
    fs::write(nested.join("skip.txt"), "ignored").expect("skip");
    let output = directory.path().join("merged.pdf");
    let pattern = directory.path().join("**").join("*.md");

    let result = binary()
        .arg(&pattern)
        .args(["--output-mode", "merge", "--output"])
        .arg(&output)
        .arg("--quiet")
        .output()
        .expect("run md2pdf");

    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let pdf = fs::read(&output).expect("read merged");
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(pdf.len() > 5_000);
}

#[test]
fn rejects_empty_grep_matches() {
    let directory = tempdir().expect("temporary directory");
    let nested = directory.path().join("empty");
    fs::create_dir_all(&nested).expect("dir");
    fs::write(nested.join("note.txt"), "no markdown").expect("txt");

    let result = binary()
        .arg(&nested)
        .args(["--grep", "**/*.md", "--quiet"])
        .output()
        .expect("run md2pdf");

    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("no Markdown files"));
}
