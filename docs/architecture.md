# Architecture

## Overview

`md2pdf` uses an in-process pipeline:

```text
Markdown
  -> pulldown-cmark events
    -> collect Mermaid fences as structured body jobs
    -> async spawn_blocking Mermaid render (capped Tokio pool)
    -> Liquid TextMate highlighting (sync, reused highlighter)
    -> in-memory Typst document
    -> Typst page layout
    -> PDF serialization
```

The binary embeds the DejaVu fonts, syntax themes, and a precompiled Liquid
TextMate grammar. General-purpose fenced code uses Typst's complete
Syntect/two-face catalog with the native Oniguruma backend. Mermaid diagrams
are rendered to SVG in process with a pure-Rust Mermaid engine. Independent
Mermaid fences are scheduled with Tokio `spawn_blocking` on a shared runtime
whose blocking pool is capped to the available CPU cores (2–8). Liquid stays
synchronous so one highlighter instance can be reused. The target
machine does not need Python, a browser, LaTeX, or the Typst executable.

## Modules

| Module | Responsibility |
| --- | --- |
| `cli` | Command-line arguments and accepted values |
| `inputs` | Multi-file discovery, `--grep` filtering, and name-format expansion |
| `highlight` | Liquid TextMate grammar, themes, and Typst token conversion |
| `markdown` | Markdown event conversion into Typst source and Mermaid assets |
| `pdf` | Typst compilation, PDF write/merge, and file resolution |
| `error` | Structured errors and user-facing CLI messages |
| `main` | Validation, batch orchestration, and parallel conversion |

## Markdown conversion

`pulldown-cmark` produces a stream of events. The converter only keeps state for
the current paragraph, heading, code block, image, list, or table. Fenced
`mermaid` / `mmd` blocks are rendered to SVG, attached as in-memory virtual
assets, and resolved by Typst before the filesystem resolver.

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

## Syntax architecture

The primary path sends fenced code to Typst's full Syntect/two-face catalog.
Oniguruma executes the production grammar regexes natively. Liquid takes a
small extension path through Shiki because the upstream two-face catalog does
not include it. Its dependency closure includes HTML, CSS, JSON, and JavaScript
grammars, so nested Shopify templates are highlighted structurally.

No language uses hand-written keyword matching. Unknown identifiers produce
plain text instead of aborting PDF generation.

## Multi-file batch conversion

When multiple sources or directories are provided, `main` collects matching files
via `inputs` (`walkdir` + `globset` / `glob`), converts each Markdown document on a
capped **rayon** pool (`--jobs`), then packages results according to
`--output-mode`:

- positional globs such as `./**/*.md` are expanded by the CLI (quote them so
  the shell leaves them intact);
- directories are walked recursively (`--grep`, default `**/*.md`);
- `files` — one PDF per input (optional output directory + `--name-format`);
- `merge` — concatenate PDF page trees with `lopdf`;
- `zip` — deflated archive of per-file PDFs.

Mermaid rendering still uses the shared Tokio runtime; workers drive it with
`Handle::block_on` so nested file-level parallelism does not call
`Runtime::block_on` concurrently.

## Resources

Image paths are resolved relative to the Markdown file. With standard input,
they are resolved from the current directory. Network resources are not
fetched.

## Memory and allocation strategy

The release binary uses jemalloc for Rust and supported native allocations.
This reduces allocator fragmentation during Typst's allocation-heavy page
layout. Syntax grammars and regexes initialize lazily. The Typst engine is
dropped as soon as compilation produces an owned paged document.

Typst still materializes the complete paged document before serialization.
Peak memory therefore scales with page count and content complexity, especially
for syntax-highlighted code and auto-sized tables.

## Invariants

- line numbers are disabled by default;
- code cannot exceed the printable width;
- each code chunk remains indivisible within a page;
- missing output directories are created automatically;
- failures return exit code `2` and never report false success.
