---
name: render-markdown-pdf
description: Render Markdown files as polished PDF documents or 16:9 slide decks with the md2pdf CLI, including Mermaid diagrams, syntax-highlighted code, images, batch conversion, merged PDFs, and ZIP output. Use when the user asks to convert or export Markdown to PDF, create a PDF presentation from Markdown, or process multiple Markdown files into PDF artifacts.
---

# Render Markdown to PDF

Use `md2pdf` for Markdown-to-PDF work. It runs without a browser, LaTeX, or an external runtime. Slide mode produces a widescreen PDF deck, not an editable `.pptx` file.

## Workflow

1. Identify the Markdown source or sources, output path, and whether the result is a document, slide deck, merged PDF, ZIP, or directory of PDFs.
2. Run `command -v md2pdf` and `md2pdf --help`. Treat the installed CLI help as authoritative for available options.
3. If the command is missing, show the relevant install command from this repository's README and ask before installing it. For Homebrew, use `brew install 0xtlt/tap/md2pdf`; the unqualified formula is a different tool.
4. Render with the smallest suitable command from the examples below.
5. Trust `md2pdf` as the rendering authority. Check that every expected output exists and is non-empty, and preserve its warnings: a slide-overflow warning means the Markdown section must be shortened.
6. Report each output with an absolute path and mention any warning or limitation.

## Documents

Render one document, allowing the default output name when the user did not request one:

```bash
md2pdf document.md
md2pdf document.md --output build/document.pdf
```

Apply only requested presentation options. Useful choices include `--page-size a4|letter`, `--landscape`, `--margin`, `--title`, `--author`, `--accent`, `--code-theme dark|light`, `--line-numbers`, `--no-header`, and `--page-break-before`.

Use `--no-external` when network access is unavailable or remote images must not be fetched. HTTPS is allowed by default; use `--allow-http` only when the user accepts cleartext image downloads.

## Slide decks

Use `--slides`; a horizontal rule surrounded by blank lines starts the next slide:

```bash
md2pdf presentation.md --slides --output presentation.pdf
md2pdf presentation.md --slides --slide-template dark --accent '#60A5FA' -o presentation.pdf
```

Choose `modern`, `minimal`, or `dark` for `--slide-template`. Keep each Markdown section short enough for one slide. Do not combine slide mode with `--page-size`, `--landscape`, or `--page-break-before`.

## Multiple files

Quote globs so `md2pdf` expands them consistently:

```bash
md2pdf './**/*.md' --output-mode merge -o all.pdf
md2pdf 'docs/**/ADR-*.md' --output-mode zip -o docs.zip --jobs 4
md2pdf a.md b.md --output-mode files -o out/ --name-format '{stem}-{index}.pdf'
```

Use `--grep` to filter directory inputs. Use `--name-format` placeholders `{stem}`, `{name}`, `{dir}`, `{index}`, and `{ext}` only with per-file or ZIP output.

## Boundaries

- Do not silently replace `md2pdf` with Pandoc, LaTeX, a browser, or another converter.
- Do not overwrite an unrelated existing output without confirming the target.
- Do not claim a slide deck is valid when `md2pdf` reports more PDF pages than Markdown slides.
