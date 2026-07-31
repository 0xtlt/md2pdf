use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

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
