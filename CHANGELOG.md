# Changelog

All notable changes are documented in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Fixed

- document that Homebrew core's Go `md2pdf` collides with this formula, and
  require `brew install 0xtlt/tap/md2pdf`;
- harden the Homebrew tap updater so formula pushes authenticate reliably and
  warn about the core name collision in formula caveats;
- ship `packaging/homebrew/md2pdf.rb` so Homebrew can install from this repo
  even when the tap formula is missing.

## [3.2.1] - 2026-07-29

### Changed

- render Mermaid fences asynchronously with Tokio `spawn_blocking` on a shared,
  CPU-capped runtime so independent diagrams can overlap across cores;
- keep Liquid highlighting synchronous so a single TextMate highlighter is
  reused for the whole document;
- merge async Mermaid results through structured body segments instead of
  string placeholders;
- add a heavy Mermaid/Liquid benchmark fixture and sync-vs-async measurements.

### Added

- added in-process Mermaid diagram support for `mermaid` and `mmd` fenced
  blocks, rendered to SVG without a browser or Node.js runtime;
- sized Mermaid diagrams to the printable page area, cropping excess canvas
  padding and converting CSS-pixel dimensions to print-friendly widths;
- sized large Mermaid diagrams more generously so complex charts stay readable
  while simple ones remain compact;
- publish macOS Intel (`x86_64`) release archives alongside Apple Silicon;
- update [`0xtlt/homebrew-tap`](https://github.com/0xtlt/homebrew-tap) on each
  tagged release so `brew install 0xtlt/tap/md2pdf` stays current.

### Fixed

- fixed Mermaid SVG cropping so flowchart diamonds and circular nodes are not
  clipped;
- rendered Mermaid diagrams with the classic light palette instead of following
  the dark code-block theme;
- refreshed the README preview asset (`preview-v2.png`) so Mermaid diagrams
  show the light palette.

## [3.2.0] - 2026-07-28

### Added

- added automated release builds for Linux x86_64, Windows x86_64, and macOS
  arm64;
- added SHA-256 checksums for all prebuilt release archives.

### Changed

- replaced the compact Rust-only override with Typst's complete Syntect and
  two-face language catalog;
- retained jemalloc on supported platforms while using the system allocator on
  Windows;
- enabled the native Oniguruma regex backend for faster full-grammar
  highlighting;
- added precompiled Liquid TextMate highlighting with embedded HTML, CSS, JSON,
  and JavaScript dependencies;
- added multi-language unit, integration, and visual fixtures;
- retained a safe plain-text fallback for unknown language identifiers.

## [3.1.0] - 2026-07-28

### Changed

- reduced peak memory by up to 24% across the benchmark suite;
- reduced median conversion time by 44% to 56%;
- reduced CPU cycles by 43% to 59%;
- added a compact embedded Rust syntax definition while preserving the
  built-in fallback highlighters for other languages;
- switched to jemalloc to reduce allocation overhead and fragmentation;
- released the Typst engine before PDF serialization.

## [3.0.1] - 2026-07-28

### Changed

- translated all current repository content and CLI output to English;
- regenerated the example PDF and README preview with English content;
- documented English as the required language for user-facing messages.

## [3.0.0] - 2026-07-28

### Added

- architecture and Markdown support documentation;
- CI workflow for formatting, Clippy, tests, and Rust documentation;
- integration tests for standard input, relative images, and CLI errors;
- fully Rust-native Markdown-to-PDF conversion;
- embedded Typst engine, DejaVu fonts, and dark syntax theme;
- tables, lists, images, links, block quotes, and syntax highlighting;
- large-block pagination and long-line wrapping;
- A4 and Letter output, metadata, and visual customization;
- file and standard-input CLI sources;
- documented public Rust APIs;
- generous vertical spacing between block elements.

[Unreleased]: https://github.com/0xtlt/md2pdf/compare/v3.2.1...HEAD
[3.2.1]: https://github.com/0xtlt/md2pdf/compare/v3.2.0...v3.2.1
[3.2.0]: https://github.com/0xtlt/md2pdf/compare/v3.1.0...v3.2.0
[3.1.0]: https://github.com/0xtlt/md2pdf/compare/v3.0.1...v3.1.0
[3.0.1]: https://github.com/0xtlt/md2pdf/compare/v3.0.0...v3.0.1
[3.0.0]: https://github.com/0xtlt/md2pdf/tree/v3.0.0
