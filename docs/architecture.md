# Architecture

## Overview

`md2pdf` uses an in-process pipeline:

```text
Markdown
  -> pulldown-cmark events
  -> in-memory Typst document
  -> Typst page layout
  -> PDF serialization
```

The binary embeds the DejaVu fonts and dark syntax theme. The target machine
does not need Python, a browser, LaTeX, or the Typst executable.

## Modules

| Module | Responsibility |
| --- | --- |
| `cli` | Command-line arguments and accepted values |
| `markdown` | Markdown event conversion into Typst source |
| `pdf` | Typst compilation, file resolution, and PDF writing |
| `error` | Structured errors and user-facing CLI messages |
| `main` | Validation, input handling, and orchestration |

## Markdown conversion

`pulldown-cmark` produces a stream of events. The converter only keeps state for
the current paragraph, heading, code block, image, list, or table.

User text is never inserted directly into Typst syntax. Quotes, backslashes,
line breaks, carriage returns, and tabs are escaped before source generation.

## Code layout

Typst does not automatically wrap `raw` blocks. The converter:

1. calculates available width from the page size and margins;
2. wraps long lines at a Unicode-safe boundary;
3. splits very large blocks into page-safe chunks;
4. keeps chunks from the same source block visually connected;
5. applies wider spacing between separate Markdown code blocks.

This avoids clipped text and preserves page headers and footers in long
documents.

## Resources

Image paths are resolved relative to the Markdown file. With standard input,
they are resolved from the current directory. Network resources are not
fetched.

## Invariants

- line numbers are disabled by default;
- code cannot exceed the printable width;
- each code chunk remains indivisible within a page;
- missing output directories are created automatically;
- failures return exit code `2` and never report false success.
