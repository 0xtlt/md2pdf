# Markdown support

| Element | Support | Notes |
| --- | :---: | --- |
| Headings `#` through `######` | Yes | The first `#` becomes the PDF title |
| Paragraphs | Yes | Consistent typographic spacing |
| Bold and italic | Yes | Combined styles are supported |
| Strikethrough | Yes | CommonMark extension enabled |
| Links | Yes | Clickable PDF annotations |
| Local images | Yes | Paths relative to the source |
| Remote images | No | Omitted; alt text is kept |
| Ordered lists | Yes | Nested lists supported |
| Unordered lists | Yes | Nested lists supported |
| Task lists | Yes | Rendered with Unicode symbols |
| Tables | Yes | Dark header and alternating rows |
| Block quotes | Yes | Callout with an accent rule |
| Horizontal rules | Yes | Full-width separator |
| Inline code | Yes | Monospace font |
| Fenced code blocks | Yes | Language follows the opening fence |
| Mermaid diagrams | Yes | `mermaid` / `mmd` fences rendered to SVG |
| Raw HTML | Partial | Displayed as text |
| Footnotes | Partial | Superscript reference only |
| Mathematics | Partial | Displayed as raw content |

## Code blocks

Fenced blocks use complete TextMate grammars. The main Syntect/two-face catalog
supports common systems, web, data, shell, and application languages. Liquid
uses its full grammar with embedded HTML, CSS, JSON, and JavaScript contexts.
Unknown identifiers fall back to plain text. Long lines are wrapped visually.
`--line-numbers` numbers visual lines and is disabled by default.

See [Syntax highlighting](syntax-highlighting.md) for tested identifiers and
examples.

## Mermaid diagrams

Fenced blocks tagged `mermaid` or `mmd` are rendered to SVG with a pure-Rust
Mermaid engine and embedded as centered images. Diagrams use Mermaid's classic
light palette so they match the light PDF page. Labels use the embedded DejaVu
Sans font. Simple diagrams stay compact; large diagrams may use more of the page
so text remains readable. Invalid Mermaid source fails PDF generation with a
clear error. No browser or Node.js runtime is required.

Supported diagram families include flowcharts, sequence, class, state, ER, pie,
XY, quadrant, gantt, timeline, journey, mindmap, and git graphs, subject to the
embedded renderer.

## Images

Supported formats follow Typst's image engine and include PNG, JPEG, and SVG. An
image alone in a paragraph becomes a centered block. An image embedded in text
is sized to the line height. Remote `http(s)` images are not downloaded; their
alt text is kept so linked badges still produce clickable labels.

## Result callout

A paragraph beginning with `Expected result:` automatically receives a green
validation style.
