# Slide decks

Pass `--slides` to turn one Markdown file into a 16:9 PDF presentation:

```console
md2pdf presentation.md --slides --output presentation.pdf
```

Separate slides with a Markdown horizontal rule surrounded by blank lines:

````markdown
# Product update

August 2026

---

## Architecture

- Markdown parser
- Embedded Typst engine
- PDF serializer

---

## Demo

```rust
fn main() {
    println!("slides");
}
```
````

The output is a PDF deck with PowerPoint-compatible widescreen dimensions
(13.333333 × 7.5 inches). It can be presented full screen, but it is not an
editable `.pptx` file.

## Templates

The first Markdown section becomes a vertically centered cover card. Every
section after a `---` separator uses a structured content card with a heading
rule and discreet page number.

Choose one of three built-in designs:

| Template | Style | Command |
| --- | --- | --- |
| `modern` | Warm canvas, white cards, strong hierarchy (default) | `--slide-template modern` |
| `minimal` | White canvas, restrained borders, generous whitespace | `--slide-template minimal` |
| `dark` | Dark navy canvas with high-contrast text | `--slide-template dark` |

For example:

```console
md2pdf presentation.md --slides --slide-template dark -o presentation.pdf
```

`--accent` customizes the highlight color in every template. The template
option requires `--slides` and never changes the standard document renderer.

## Slide behavior

- One Markdown file produces one deck.
- Each `---` horizontal rule starts the next slide.
- Headings, lists, tables, code, images, links, and Mermaid diagrams use the
  existing Markdown renderer with presentation-sized layout.
- Standalone images are constrained to the slide area without stretching.
- Document headers are omitted; slide decks use a discreet page number.
- `--accent`, `--margin`, `--code-theme`, and `--line-numbers` still apply.
- `--page-size`, `--landscape`, and `--page-break-before` cannot be combined
  with `--slides`, because the slide size and boundaries are fixed.

Keep each section short enough to fit on one slide. If a section flows onto an
extra page, `md2pdf` still writes the PDF and prints a warning with the expected
Markdown slide count and actual PDF page count. Pass `--quiet` to suppress
warnings as usual.

Without `--slides`, horizontal rules remain visible separators and all existing
A4 / Letter behavior is unchanged.
