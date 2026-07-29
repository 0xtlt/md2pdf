# AGENTS.md

## Cursor Cloud specific instructions

`md2pdf` is a single standalone Rust CLI (no services, database, or web server). It converts Markdown to PDF using an embedded Typst engine; fonts, themes, and grammars are compiled into the binary, so there is nothing to start or keep running.

### Toolchain caveat (important)

The crate uses `edition = "2024"` (see `Cargo.toml`), which requires **Rust >= 1.85**. The base image may ship an older stable (e.g. 1.83) that cannot compile the crate. The startup update script runs `rustup update stable` / `rustup default stable` to ensure a compatible toolchain. If a fresh session fails to build with an edition-2024 error, run `rustup update stable && rustup default stable`.

### Standard commands

These are already documented in `README.md` / `CONTRIBUTING.md`; use them directly:

- Build (release): `cargo build --release` — the binary lands at `target/release/md2pdf`. The first release build is slow (~3 min) due to the Typst dependency tree.
- Run: `./target/release/md2pdf example.md` (writes `example.pdf` next to the source). See `README.md` for CLI options.
- Quality gate: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps`.

### Visual PDF verification (optional)

Poppler is not installed by the update script. To rasterize a generated PDF for manual inspection, install it once per session (`sudo apt-get update && sudo apt-get install -y poppler-utils`) then `pdftoppm -png -r 120 example.pdf out`.
