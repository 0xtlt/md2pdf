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
    fs::write(&source, "# Compatible\n\nTexte.").expect("write markdown");

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
