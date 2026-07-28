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

The binary embeds the DejaVu fonts, dark syntax theme, and a compact Rust syntax
definition. Other languages use Typst's built-in syntax collection as a
fallback. The target machine does not need Python, a browser, LaTeX, or the
Typst executable.

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

## Memory and allocation strategy

The release binary uses jemalloc for Rust and supported native allocations.
This reduces allocator fragmentation during Typst's allocation-heavy page
layout. Rust code blocks use the embedded targeted syntax definition, avoiding
initialization of the full syntax collection when it is unnecessary. The Typst
engine is dropped as soon as compilation produces an owned paged document.

Typst still materializes the complete paged document before serialization.
Peak memory therefore scales with page count and content complexity, especially
for syntax-highlighted code and auto-sized tables.

## Invariants

- line numbers are disabled by default;
- code cannot exceed the printable width;
- each code chunk remains indivisible within a page;
- missing output directories are created automatically;
- failures return exit code `2` and never report false success.
