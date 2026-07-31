# AGENTS.md

## Cursor Cloud specific instructions

`md2pdf` is a single, self-contained Rust CLI crate (Markdown → PDF via an embedded
Typst engine). There is no database, server, or other service to run — the only
"application" is the `md2pdf` binary.

- Toolchain: `Cargo.toml` sets `edition = "2024"`, which requires Rust **>= 1.85**.
  The base VM image ships an older default (1.83); the startup update script pins
  the `stable` toolchain (with `rustfmt` + `clippy`), so just use the default
  toolchain — do not run against the old 1.83 default.
- Quality gate / lint / test / doc commands are already documented in
  `CONTRIBUTING.md` (`cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo test --locked`, `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked`).
  Integration tests in `tests/cli.rs` invoke the real built binary.
- Run the app: `cargo run --locked -- <input.md> --output <out.pdf>` (see `README.md`
  for all flags). `example.md` is a good end-to-end sample exercising tables, Mermaid,
  and syntax highlighting.
- Some tests download remote images over HTTPS (e.g. README badge tests), so they need
  network egress; they pass when egress is allowed.
- Visual verification: to render a generated PDF to images, install `poppler-utils`
  ad hoc (`pdftoppm`); it is intentionally **not** in the update script since it is only
  needed for manual inspection, not for building/testing.
