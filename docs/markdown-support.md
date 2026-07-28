# Markdown support

| Element | Support | Notes |
| --- | :---: | --- |
| Headings `#` through `######` | Yes | The first `#` becomes the PDF title |
| Paragraphs | Yes | Consistent typographic spacing |
| Bold and italic | Yes | Combined styles are supported |
| Strikethrough | Yes | CommonMark extension enabled |
| Links | Yes | Clickable PDF annotations |
| Local images | Yes | Paths relative to the source |
| Remote images | No | No network downloads |
| Ordered lists | Yes | Nested lists supported |
| Unordered lists | Yes | Nested lists supported |
| Task lists | Yes | Rendered with Unicode symbols |
| Tables | Yes | Dark header and alternating rows |
| Block quotes | Yes | Callout with an accent rule |
| Horizontal rules | Yes | Full-width separator |
| Inline code | Yes | Monospace font |
| Fenced code blocks | Yes | Language follows the opening fence |
| Raw HTML | Partial | Displayed as text |
| Footnotes | Partial | Superscript reference only |
| Mathematics | Partial | Displayed as raw content |

## Code blocks

Rust uses a compact embedded syntax definition optimized for startup and memory.
Other language identifiers recognized by Typst use its built-in syntax
highlighters. Long lines are wrapped visually. `--line-numbers` numbers visual lines and is
disabled by default.

## Images

Supported formats follow Typst's image engine and include PNG, JPEG, and SVG. An
image alone in a paragraph becomes a centered block. An image embedded in text
is sized to the line height.

## Result callout

A paragraph beginning with `Expected result:` automatically receives a green
validation style.
