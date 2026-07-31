# md2pdf

[![CI](https://github.com/0xtlt/md2pdf/actions/workflows/ci.yml/badge.svg)](https://github.com/0xtlt/md2pdf/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A fast Markdown-to-PDF converter written in Rust. Pagination, images, tables,
links, Mermaid diagrams, and syntax highlighting — no Python, browser, or
LaTeX required.

![Generated PDF preview](docs/assets/preview-v2.png)

## Install

### Homebrew

Homebrew core ships a **different** Go-based `md2pdf`. Install this one with:

```console
brew install 0xtlt/tap/md2pdf
```

If `md2pdf --help` mentions `md2pdf.go`, you have the wrong package:

```console
brew uninstall md2pdf
brew install 0xtlt/tap/md2pdf
```

### Binary

Download the archive for your platform from the
[latest release](https://github.com/0xtlt/md2pdf/releases/latest).

### Cargo

```console
cargo install --git https://github.com/0xtlt/md2pdf
```

## Usage

```console
md2pdf document.md
md2pdf document.md --output build/document.pdf
md2pdf document.md --code-theme light --accent '#2563EB'
md2pdf document.md --page-size letter --landscape
cat document.md | md2pdf - --output document.pdf
md2pdf docs/ --grep '**/ADR-*.md' --output-mode merge -o adr.pdf
md2pdf docs/ --output-mode zip -o docs.zip -j 4
md2pdf a.md b.md --output-mode files -o out/ --name-format '{stem}-{index}.pdf'
```

| Option | Default | Description |
| --- | --- | --- |
| `SOURCE...` | required | Markdown files and/or directories (`-` for stdin) |
| `-o, --output PATH` | source with `.pdf` | Output PDF, zip path, or directory for per-file mode |
| `--grep GLOB` | `**/*.md` for directories | Path glob used when selecting files under directories |
| `--output-mode files\|merge\|zip` | `files` | Per-file PDFs, one merged PDF, or a zip of PDFs |
| `--name-format TEMPLATE` | `{stem}.pdf` | Per-file / zip entry names (`{stem}`, `{name}`, `{dir}`, `{index}`, `{ext}`) |
| `-j, --jobs N` | CPU count (1–8) | Parallel file conversions |
| `--title TEXT` | first `#` heading | PDF metadata title |
| `--author TEXT` | empty | PDF metadata author |
| `--page-size a4\|letter` | `a4` | Page format |
| `--landscape` | off | Landscape orientation |
| `--margin MM` | `17` | Margins (8–45 mm) |
| `--accent '#RRGGBB'` | `#C94C35` | Heading color |
| `--code-theme dark\|light` | `dark` | Code block theme |
| `--line-numbers` | off | Show code line numbers |
| `--no-header` | off | Hide page header |
| `--page-break-before PREFIX` | none | Page break before matching `##` |
| `--no-external` | off | Do not download remote images |
| `--allow-http` | off | Allow cleartext `http://` image downloads |
| `-q, --quiet` | off | Suppress success output and warnings |

Run `md2pdf --help` for the full list.

## Features

- Typst PDF engine with embedded fonts
- TextMate syntax highlighting (dark / light)
- Mermaid diagrams from `mermaid` / `mmd` fences
- Clickable links and local images
- Remote HTTPS images downloaded by default (`--no-external` to deny, `--allow-http` for cleartext HTTP)
- Multi-file conversion: merge, zip, or per-file output with parallel jobs
- A4 / Letter, portrait or landscape
- Custom title, author, header, footer, margins, and accent

## Docs

- [Markdown support](docs/markdown-support.md)
- [Syntax highlighting](docs/syntax-highlighting.md)
- [Architecture](docs/architecture.md)
- [Contributing](CONTRIBUTING.md)

## License

[MIT](LICENSE). Embedded DejaVu fonts:
[`assets/fonts/LICENSE_DEJAVU`](assets/fonts/LICENSE_DEJAVU).
