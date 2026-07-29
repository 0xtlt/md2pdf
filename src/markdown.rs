use mermaid_rs_renderer::{RenderOptions, Theme, render_with_options};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::{
    Error, Result,
    cli::{CodeTheme, PageSize},
    highlight::{StyledToken, SyntaxHighlighter},
};

const POINTS_PER_MM: f32 = 2.834_646;
const CODE_GLYPH_WIDTH_PT: f32 = 4.45;
const CODE_LINE_HEIGHT_PT: f32 = 8.8;

/// Rendering options used to build the intermediate Typst document.
#[derive(Clone, Debug)]
pub struct TypstOptions {
    /// Accent color as a validated `#RRGGBB` string.
    pub accent: String,
    /// Syntax-highlighting theme for fenced code blocks.
    pub code_theme: CodeTheme,
    /// Whether fenced code blocks should display line numbers.
    pub line_numbers: bool,
    /// PDF metadata title.
    pub title: String,
    /// PDF metadata author.
    pub author: String,
    /// Text displayed in the page header.
    pub label: String,
    /// Text displayed in the page footer.
    pub footer: String,
    /// Output paper size.
    pub page_size: PageSize,
    /// Whether pages are rendered in landscape orientation.
    pub landscape: bool,
    /// Page margin in millimetres.
    pub margin_mm: f32,
    /// Whether to render the page header.
    pub show_header: bool,
    /// H2 title prefixes that should start on a new page.
    pub page_break_prefixes: Vec<String>,
}

/// Converted Typst document plus virtual binary assets such as Mermaid SVGs.
#[derive(Clone, Debug, Default)]
pub struct TypstDocument {
    /// Complete Typst source ready for compilation.
    pub source: String,
    /// In-memory files resolved during PDF compilation (`path` → bytes).
    pub assets: Vec<(String, Vec<u8>)>,
}

#[derive(Default)]
struct InlineBuffer {
    typst: String,
    plain: String,
}

/// Extract the plain-text content of the first level-one Markdown heading.
#[must_use]
pub fn first_title(markdown: &str) -> Option<String> {
    let parser = Parser::new_ext(markdown, parser_options());
    let mut in_h1 = false;
    let mut title = String::new();
    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => in_h1 = true,
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => break,
            Event::Text(text) | Event::Code(text) if in_h1 => title.push_str(&text),
            _ => {}
        }
    }
    (!title.trim().is_empty()).then(|| title.trim().to_owned())
}

/// Convert Markdown into a complete Typst source document.
pub fn to_typst(markdown: &str, options: &TypstOptions) -> Result<TypstDocument> {
    let parser = Parser::new_ext(markdown, parser_options());
    let mut liquid_highlighter = None;
    let mut body = String::new();
    let mut assets = Vec::new();
    let mut paragraph: Option<InlineBuffer> = None;
    let mut heading: Option<(HeadingLevel, InlineBuffer)> = None;
    let mut code: Option<(String, String)> = None;
    let mut image_depth = 0usize;
    let mut standalone_image = false;
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut in_table_head = false;
    let mut in_table_cell = false;

    for event in parser {
        if let Some((_, source)) = &mut code {
            match event {
                Event::Text(text) => source.push_str(&text),
                Event::End(TagEnd::CodeBlock) => {
                    let (language, source) = code.take().expect("code buffer exists");
                    if is_mermaid_language(&language) {
                        let (typst, asset) = mermaid_block(&source, options, assets.len())?;
                        assets.push(asset);
                        body.push_str(&typst);
                    } else {
                        body.push_str(&code_block(
                            &source,
                            &language,
                            max_code_columns(options),
                            max_code_lines(options),
                            options.line_numbers,
                            options.code_theme,
                            &mut liquid_highlighter,
                        )?);
                    }
                }
                _ => {}
            }
            continue;
        }

        match event {
            Event::Start(Tag::Paragraph) => {
                if paragraph.is_none() {
                    paragraph = Some(InlineBuffer::default());
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if let Some(buffer) = paragraph.take() {
                    if is_expected_result(&buffer.plain) {
                        body.push_str(&format!(
                            "#block(width: 100%, above: 6pt, below: 14pt, \
                             fill: rgb(\"#EAF7F1\"), inset: 9pt, \
                             stroke: (left: 3pt + rgb(\"#237A57\")))[{}]\n\n",
                            buffer.typst
                        ));
                    } else {
                        body.push_str(&buffer.typst);
                        if !in_table_cell {
                            body.push_str("\n\n");
                        }
                    }
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some((level, InlineBuffer::default()));
            }
            Event::End(TagEnd::Heading(level)) => {
                if let Some((_, buffer)) = heading.take() {
                    if level == HeadingLevel::H2
                        && options
                            .page_break_prefixes
                            .iter()
                            .any(|prefix| buffer.plain.starts_with(prefix))
                    {
                        body.push_str("#pagebreak(weak: true)\n");
                    }
                    let marks = "=".repeat(heading_number(level));
                    body.push_str(&format!("{marks} {}\n\n", buffer.typst));
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_owned()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((language, String::new()));
            }
            Event::Start(Tag::BlockQuote(_)) => {
                body.push_str(
                    "#block(width: 100%, above: 6pt, below: 14pt, \
                     fill: rgb(\"#FFF5F2\"), inset: 9pt, \
                     stroke: (left: 3pt + accent))[\n",
                );
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                body.push_str("]\n\n");
            }
            Event::Start(Tag::List(start)) => {
                if let Some(buffer) = paragraph.take() {
                    body.push_str(&buffer.typst);
                }
                list_stack.push(start);
                if list_stack.len() > 1 {
                    body.push('\n');
                }
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                body.push('\n');
            }
            Event::Start(Tag::Item) => {
                let indent = "  ".repeat(list_stack.len().saturating_sub(1));
                let marker = if list_stack.last().copied().flatten().is_some() {
                    "+"
                } else {
                    "-"
                };
                body.push_str(&format!("{indent}{marker} "));
                paragraph = Some(InlineBuffer::default());
            }
            Event::End(TagEnd::Item) => {
                if let Some(buffer) = paragraph.take() {
                    body.push_str(&buffer.typst);
                }
                body.push('\n');
            }
            Event::Start(Tag::Table(alignments)) => {
                let table_columns = alignments.len();
                body.push_str(&format!(
                    "#block(width: 100%, above: 8pt, below: 14pt)[\n\
                     #table(columns: {}, inset: 6pt, stroke: 0.4pt + rgb(\"#D0D5DD\"), \
                     fill: (x, y) => if y == 0 {{ ink }} else if calc.even(y) {{ rgb(\"#F8FAFC\") }},\n",
                    table_columns
                ));
            }
            Event::End(TagEnd::Table) => {
                body.push_str(")\n]\n\n");
            }
            Event::Start(Tag::TableHead) => in_table_head = true,
            Event::End(TagEnd::TableHead) => in_table_head = false,
            Event::Start(Tag::TableCell) => {
                in_table_cell = true;
                body.push('[');
                if in_table_head {
                    body.push_str("#set text(fill: white, weight: \"bold\"); ");
                }
                paragraph = Some(InlineBuffer::default());
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(buffer) = paragraph.take() {
                    body.push_str(&buffer.typst);
                }
                in_table_cell = false;
                body.push_str("],\n");
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                image_depth += 1;
                standalone_image = paragraph
                    .as_ref()
                    .is_some_and(|buffer| buffer.typst.is_empty());
                if standalone_image {
                    paragraph.take();
                    body.push_str(&format!(
                        "#block(width: 100%, above: 7pt, below: 18pt)\
                         [#align(center)[#image({}, width: 90%)]]\n\n",
                        typst_string(&dest_url)
                    ));
                } else {
                    push_inline(
                        &mut paragraph,
                        &mut heading,
                        &format!("#image({}, height: 1em)", typst_string(&dest_url)),
                        "",
                    );
                }
            }
            Event::End(TagEnd::Image) => {
                image_depth = image_depth.saturating_sub(1);
                if standalone_image {
                    paragraph = Some(InlineBuffer::default());
                    standalone_image = false;
                }
            }
            Event::Start(Tag::Strong) => push_inline(&mut paragraph, &mut heading, "*", ""),
            Event::End(TagEnd::Strong) => push_inline(&mut paragraph, &mut heading, "*", ""),
            Event::Start(Tag::Emphasis) => push_inline(&mut paragraph, &mut heading, "_", ""),
            Event::End(TagEnd::Emphasis) => push_inline(&mut paragraph, &mut heading, "_", ""),
            Event::Start(Tag::Strikethrough) => {
                push_inline(&mut paragraph, &mut heading, "#strike[", "")
            }
            Event::End(TagEnd::Strikethrough) => push_inline(&mut paragraph, &mut heading, "]", ""),
            Event::Start(Tag::Link { dest_url, .. }) => push_inline(
                &mut paragraph,
                &mut heading,
                &format!("#link({})[", typst_string(&dest_url)),
                "",
            ),
            Event::End(TagEnd::Link) => push_inline(&mut paragraph, &mut heading, "]", ""),
            Event::Text(text) if image_depth == 0 => {
                let expression = format!("#text({})", typst_string(&text));
                push_inline(&mut paragraph, &mut heading, &expression, &text);
            }
            Event::Code(text) => {
                let expression = format!("#raw({})", typst_string(&text));
                push_inline(&mut paragraph, &mut heading, &expression, &text);
            }
            Event::SoftBreak => push_inline(&mut paragraph, &mut heading, " ", " "),
            Event::HardBreak => push_inline(&mut paragraph, &mut heading, "\\\n", "\n"),
            Event::Rule => body.push_str(
                "#block(width: 100%, above: 14pt, below: 14pt)\
                 [#line(length: 100%, stroke: 0.5pt + border)]\n\n",
            ),
            Event::TaskListMarker(checked) => {
                let marker = if checked { "☑ " } else { "☐ " };
                push_inline(
                    &mut paragraph,
                    &mut heading,
                    &format!("#text({})", typst_string(marker)),
                    marker,
                );
            }
            Event::FootnoteReference(label) => {
                push_inline(
                    &mut paragraph,
                    &mut heading,
                    &format!("#super[{}]", typst_string(&label)),
                    &label,
                );
            }
            Event::InlineMath(math) | Event::DisplayMath(math) => {
                push_inline(
                    &mut paragraph,
                    &mut heading,
                    &format!("#raw({})", typst_string(&math)),
                    &math,
                );
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                let expression = format!("#text({})", typst_string(&html));
                push_inline(&mut paragraph, &mut heading, &expression, &html);
            }
            _ => {}
        }
    }

    Ok(TypstDocument {
        source: format!("{}\n{}", template(options), body),
        assets,
    })
}

fn template(options: &TypstOptions) -> String {
    let paper = match options.page_size {
        PageSize::A4 => "a4",
        PageSize::Letter => "us-letter",
    };
    let header = if options.show_header {
        format!(
            r#"context [
  #set text(size: 7.5pt, weight: "bold", fill: ink)
  #text({})
  #v(2pt)
  #line(length: 100%, stroke: 0.4pt + border)
]"#,
            typst_string(&options.label)
        )
    } else {
        "none".to_owned()
    };
    let footer = format!(
        r#"context [
  #set text(size: 7.5pt, fill: muted)
  #grid(columns: (1fr, auto), [{}], [#counter(page).display("1")])
]"#,
        escape_markup_text(&options.footer)
    );
    let (code_fill, code_text) = code_palette(options.code_theme);
    let raw_theme = match options.code_theme {
        CodeTheme::Dark => "#set raw(theme: \"md2pdf-dark.tmTheme\")",
        CodeTheme::Light => "",
    };
    format!(
        r##"#let accent = rgb("{accent}")
#let ink = rgb("#17202A")
#let muted = rgb("#667085")
#let border = rgb("#D0D5DD")

#set document(title: {title}, author: ({author},))
#set page(
  paper: "{paper}",
  flipped: {landscape},
  margin: (x: {margin}mm, top: {top_margin}mm, bottom: {margin}mm),
  header: {header},
  footer: {footer},
)
#set text(font: "DejaVu Sans", size: 9.5pt, fill: ink)
#set par(leading: 0.62em, spacing: 0.9em, justify: false)
#set heading(numbering: none)
#show heading.where(level: 1): it => align(center)[
  #v(10pt)
  #text(size: 24pt, weight: "bold", fill: ink)[#it.body]
  #v(8pt)
]
#show heading.where(level: 2): it => block(above: 16pt, below: 10pt)[
  #text(size: 16pt, weight: "bold", fill: accent)[#it.body]
]
#show heading.where(level: 3): it => block(above: 13pt, below: 7pt)[
  #text(size: 12pt, weight: "bold", fill: accent)[#it.body]
]
#show heading.where(level: 4): set text(size: 10pt, weight: "bold", fill: accent)
#show heading.where(level: 5): set text(size: 9.5pt, weight: "bold", fill: accent)
#show heading.where(level: 6): set text(size: 9pt, weight: "bold", fill: accent)
{raw_theme}
#show raw.where(block: true): it => block(
  width: 100%,
  fill: rgb("{code_fill}"),
  inset: 9pt,
  radius: 2pt,
  breakable: false,
)[
  #set text(font: "DejaVu Sans Mono", size: 7.3pt, fill: rgb("{code_text}"))
  #it
]
#show link: it => text(fill: rgb("#0969DA"), it)

"##,
        accent = options.accent,
        title = typst_string(&options.title),
        author = typst_string(&options.author),
        paper = paper,
        landscape = options.landscape,
        margin = options.margin_mm,
        top_margin = options.margin_mm + if options.show_header { 5.0 } else { 0.0 },
        header = header,
        footer = footer,
        code_fill = code_fill,
        code_text = code_text,
        raw_theme = raw_theme,
    )
}

fn code_block(
    source: &str,
    language: &str,
    max_columns: usize,
    max_lines: usize,
    line_numbers: bool,
    theme: CodeTheme,
    liquid_highlighter: &mut Option<SyntaxHighlighter>,
) -> Result<String> {
    let language = normalize_fence_language(language);
    let mut source = wrap_code(source.trim_end_matches('\n'), max_columns);
    if line_numbers {
        source = source
            .lines()
            .enumerate()
            .map(|(index, line)| format!("{:>3} │ {line}", index + 1))
            .collect::<Vec<_>>()
            .join("\n");
    }
    let lines = source.lines().collect::<Vec<_>>();
    let chunks = if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
            .chunks(max_lines)
            .map(|chunk| chunk.join("\n"))
            .collect()
    };
    let mut output = String::from("#v(7pt, weak: true)\n");
    for (index, chunk) in chunks.iter().enumerate() {
        if index > 0 {
            output.push_str("#v(5pt, weak: true)\n");
        }
        if is_liquid_language(&language) {
            let highlighter = match liquid_highlighter {
                Some(highlighter) => highlighter,
                None => liquid_highlighter.insert(SyntaxHighlighter::new(theme)?),
            };
            let lines = highlighter.highlight(chunk, "liquid")?;
            output.push_str(&highlighted_code_frame(&lines, theme));
        } else {
            output.push_str(&format!(
                "#raw(block: true, lang: {}, {})\n",
                typst_string(&language),
                typst_string(chunk)
            ));
        }
    }
    output.push_str("#v(18pt, weak: true)\n\n");
    Ok(output)
}

fn highlighted_code_frame(lines: &[Vec<StyledToken>], theme: CodeTheme) -> String {
    let (fill, foreground) = code_palette(theme);
    let mut output = format!(
        "#block(width: 100%, fill: rgb(\"{fill}\"), inset: 9pt, \
         radius: 2pt, breakable: false)[\n\
         #set text(font: \"DejaVu Sans Mono\", size: 7.3pt, \
         fill: rgb(\"{foreground}\"))\n\
         #set par(leading: 0.2em, spacing: 0pt)\n"
    );
    for (line_index, line) in lines.iter().enumerate() {
        if line_index > 0 {
            output.push_str("#linebreak()");
        }
        for token in line {
            output.push_str(&styled_token(token));
        }
    }
    output.push_str("\n]\n");
    output
}

fn code_palette(theme: CodeTheme) -> (&'static str, &'static str) {
    match theme {
        CodeTheme::Dark => ("#0D1117", "#E6EDF3"),
        CodeTheme::Light => ("#F6F8FA", "#1F2328"),
    }
}

fn is_liquid_language(language: &str) -> bool {
    matches!(
        language.trim().to_ascii_lowercase().as_str(),
        "liquid" | "shopify-liquid"
    )
}

fn is_mermaid_language(language: &str) -> bool {
    matches!(
        normalize_fence_language(language).as_str(),
        "mermaid" | "mmd"
    )
}

fn mermaid_block(
    source: &str,
    options: &TypstOptions,
    index: usize,
) -> Result<(String, (String, Vec<u8>))> {
    let mut render_options = RenderOptions {
        // PDF pages are light; keep diagrams on the classic Mermaid palette
        // instead of following the dark code-block theme.
        theme: Theme::mermaid_default(),
        ..RenderOptions::default()
    };
    // Typst resolves a single SVG font family; use the embedded DejaVu face.
    render_options.theme.font_family = "DejaVu Sans".to_owned();
    // Slightly denser than Mermaid's screen defaults so diagrams match print text.
    render_options.theme.font_size = 12.0;
    let svg = render_with_options(source.trim(), render_options)
        .map_err(|error| Error::Mermaid(error.to_string()))?;
    let svg = crop_mermaid_svg(&svg);
    let path = format!("md2pdf-mermaid-{index}.svg");
    let width_mm = mermaid_display_width_mm(&svg, options);
    let typst = format!(
        "#block(width: 100%, above: 7pt, below: 18pt)\
         [#align(center)[#image({}, width: {width_mm:.2}mm)]]\n\n",
        typst_string(&path)
    );
    Ok((typst, (path, svg.into_bytes())))
}

/// Remove excess Mermaid canvas padding so sizing uses the drawn content.
fn crop_mermaid_svg(svg: &str) -> String {
    let Some((canvas_x, canvas_y, canvas_w, canvas_h)) = svg_view_box(svg) else {
        return svg.to_owned();
    };
    let Some((min_x, min_y, max_x, max_y)) = mermaid_svg_content_bounds(svg) else {
        return svg.to_owned();
    };
    if max_x <= min_x || max_y <= min_y {
        return svg.to_owned();
    }
    let pad = 16.0;
    let x = min_x - pad;
    let y = min_y - pad;
    let width = ((max_x - min_x) + 2.0 * pad).min(canvas_w);
    let height = ((max_y - min_y) + 2.0 * pad).min(canvas_h);
    // Skip crop when there is no meaningful letterbox.
    let saved_w = canvas_w - width;
    let saved_h = canvas_h - height;
    if saved_w < 24.0 && saved_h < 24.0 {
        return svg.to_owned();
    }
    let _ = (canvas_x, canvas_y);
    let mut output = replace_svg_root_dimensions(svg, width, height, x, y, width, height);
    output = replace_svg_background_rect(&output, x, y, width, height);
    output
}

fn mermaid_svg_content_bounds(svg: &str) -> Option<(f32, f32, f32, f32)> {
    let without_defs = strip_svg_defs(svg);
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut found = false;

    for (index, _) in without_defs.match_indices("<rect") {
        let Some(end) = without_defs[index..].find('>') else {
            continue;
        };
        let tag = &without_defs[index..index + end];
        let Some(x) = svg_tag_number(tag, "x") else {
            continue;
        };
        let Some(y) = svg_tag_number(tag, "y") else {
            continue;
        };
        let Some(width) = svg_tag_number(tag, "width") else {
            continue;
        };
        let Some(height) = svg_tag_number(tag, "height") else {
            continue;
        };
        // Skip the full-canvas background rectangle.
        let is_background_fill = tag.contains("fill=\"#FFFFFF\"")
            || tag.contains("fill=\"#333333\"")
            || tag.contains("fill=\"#ffffff\"");
        if width > 200.0 && height > 200.0 && is_background_fill && !tag.contains("rx=") {
            continue;
        }
        include_bounds(&mut min_x, &mut min_y, &mut max_x, &mut max_y, x, y);
        include_bounds(
            &mut min_x,
            &mut min_y,
            &mut max_x,
            &mut max_y,
            x + width,
            y + height,
        );
        found = true;
    }

    for (index, _) in without_defs.match_indices("<line") {
        let Some(end) = without_defs[index..].find('>') else {
            continue;
        };
        let tag = &without_defs[index..index + end];
        let points = [
            (svg_tag_number(tag, "x1"), svg_tag_number(tag, "y1")),
            (svg_tag_number(tag, "x2"), svg_tag_number(tag, "y2")),
        ];
        for (x, y) in points {
            if let (Some(x), Some(y)) = (x, y) {
                include_bounds(&mut min_x, &mut min_y, &mut max_x, &mut max_y, x, y);
                found = true;
            }
        }
    }

    for (index, _) in without_defs.match_indices("<polygon") {
        let Some(end) = without_defs[index..].find('>') else {
            continue;
        };
        let tag = &without_defs[index..index + end];
        let Some(points) = svg_tag_attr(tag, "points") else {
            continue;
        };
        // Skip tiny marker arrowheads expressed in local transform space.
        if let Some((poly_min_x, poly_min_y, poly_max_x, poly_max_y)) = polygon_bounds(points) {
            let poly_w = poly_max_x - poly_min_x;
            let poly_h = poly_max_y - poly_min_y;
            if poly_w < 20.0 && poly_h < 20.0 {
                continue;
            }
            include_bounds(
                &mut min_x, &mut min_y, &mut max_x, &mut max_y, poly_min_x, poly_min_y,
            );
            include_bounds(
                &mut min_x, &mut min_y, &mut max_x, &mut max_y, poly_max_x, poly_max_y,
            );
            found = true;
        }
    }

    for (index, _) in without_defs.match_indices("<circle") {
        let Some(end) = without_defs[index..].find('>') else {
            continue;
        };
        let tag = &without_defs[index..index + end];
        if let (Some(cx), Some(cy), Some(r)) = (
            svg_tag_number(tag, "cx"),
            svg_tag_number(tag, "cy"),
            svg_tag_number(tag, "r"),
        ) {
            // Skip tiny marker dots expressed in local transform space.
            if r < 10.0 && cx.abs() < 20.0 && cy.abs() < 20.0 {
                continue;
            }
            include_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                cx - r,
                cy - r,
            );
            include_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                cx + r,
                cy + r,
            );
            found = true;
        }
    }

    for (index, _) in without_defs.match_indices("<ellipse") {
        let Some(end) = without_defs[index..].find('>') else {
            continue;
        };
        let tag = &without_defs[index..index + end];
        if let (Some(cx), Some(cy), Some(rx), Some(ry)) = (
            svg_tag_number(tag, "cx"),
            svg_tag_number(tag, "cy"),
            svg_tag_number(tag, "rx"),
            svg_tag_number(tag, "ry"),
        ) {
            include_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                cx - rx,
                cy - ry,
            );
            include_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                cx + rx,
                cy + ry,
            );
            found = true;
        }
    }

    for (index, _) in without_defs.match_indices("<text") {
        let Some(end) = without_defs[index..].find('>') else {
            continue;
        };
        let tag = &without_defs[index..index + end];
        if let (Some(x), Some(y)) = (svg_tag_number(tag, "x"), svg_tag_number(tag, "y")) {
            let font_size = svg_tag_number(tag, "font-size").unwrap_or(12.0);
            include_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                x - font_size * 4.0,
                y - font_size,
            );
            include_bounds(
                &mut min_x,
                &mut min_y,
                &mut max_x,
                &mut max_y,
                x + font_size * 4.0,
                y + font_size * 0.4,
            );
            found = true;
        }
    }

    for (index, _) in without_defs.match_indices(" d=\"") {
        let start = index + 4;
        let Some(end) = without_defs[start..].find('"') else {
            continue;
        };
        for number in path_coordinate_pairs(&without_defs[start..start + end]) {
            include_bounds(
                &mut min_x, &mut min_y, &mut max_x, &mut max_y, number.0, number.1,
            );
            found = true;
        }
    }

    found.then_some((min_x, min_y, max_x, max_y))
}

fn polygon_bounds(points: &str) -> Option<(f32, f32, f32, f32)> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut found = false;
    for pair in points.split_whitespace() {
        let mut parts = pair.split(',');
        let Some(x) = parts.next().and_then(|value| value.parse().ok()) else {
            continue;
        };
        let Some(y) = parts.next().and_then(|value| value.parse().ok()) else {
            continue;
        };
        include_bounds(&mut min_x, &mut min_y, &mut max_x, &mut max_y, x, y);
        found = true;
    }
    found.then_some((min_x, min_y, max_x, max_y))
}

fn svg_view_box(svg: &str) -> Option<(f32, f32, f32, f32)> {
    let view_box = svg_attr(svg, "viewBox")?;
    let parts = view_box
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() != 4 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
        parts[3].parse().ok()?,
    ))
}

fn include_bounds(
    min_x: &mut f32,
    min_y: &mut f32,
    max_x: &mut f32,
    max_y: &mut f32,
    x: f32,
    y: f32,
) {
    if !x.is_finite() || !y.is_finite() {
        return;
    }
    *min_x = min_x.min(x);
    *min_y = min_y.min(y);
    *max_x = max_x.max(x);
    *max_y = max_y.max(y);
}

fn strip_svg_defs(svg: &str) -> String {
    let Some(start) = svg.find("<defs") else {
        return svg.to_owned();
    };
    let Some(end_rel) = svg[start..].find("</defs>") else {
        return svg.to_owned();
    };
    let end = start + end_rel + "</defs>".len();
    format!("{}{}", &svg[..start], &svg[end..])
}

fn svg_tag_number(tag: &str, name: &str) -> Option<f32> {
    svg_tag_attr(tag, name)?.parse().ok()
}

fn svg_tag_attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let keyed = format!("{name}=\"");
    let mut search_from = 0usize;
    while let Some(relative) = tag[search_from..].find(&keyed) {
        let start = search_from + relative;
        let boundary_ok = start == 0
            || tag.as_bytes()[start - 1].is_ascii_whitespace()
            || tag.as_bytes()[start - 1] == b'<';
        if boundary_ok {
            let value_start = start + keyed.len();
            let end = value_start + tag[value_start..].find('"')?;
            return Some(&tag[value_start..end]);
        }
        search_from = start + keyed.len();
    }
    None
}

fn path_coordinate_pairs(path: &str) -> Vec<(f32, f32)> {
    let mut numbers = Vec::new();
    let mut current = String::new();
    for character in path.chars() {
        if character.is_ascii_digit() || character == '.' || character == '-' {
            current.push(character);
            continue;
        }
        if !current.is_empty() {
            if let Ok(value) = current.parse::<f32>() {
                numbers.push(value);
            }
            current.clear();
        }
    }
    if !current.is_empty()
        && let Ok(value) = current.parse::<f32>()
    {
        numbers.push(value);
    }
    numbers
        .chunks_exact(2)
        .map(|chunk| (chunk[0], chunk[1]))
        .collect()
}

fn replace_svg_root_dimensions(
    svg: &str,
    width: f32,
    height: f32,
    view_x: f32,
    view_y: f32,
    view_w: f32,
    view_h: f32,
) -> String {
    let Some(tag_end) = svg.find('>') else {
        return svg.to_owned();
    };
    let mut open = svg[..tag_end].to_owned();
    open = replace_attr(&open, "width", &format!("{width:.3}"));
    open = replace_attr(&open, "height", &format!("{height:.3}"));
    open = replace_attr(
        &open,
        "viewBox",
        &format!("{view_x:.3} {view_y:.3} {view_w:.3} {view_h:.3}"),
    );
    format!("{open}{}", &svg[tag_end..])
}

fn replace_svg_background_rect(svg: &str, x: f32, y: f32, width: f32, height: f32) -> String {
    let Some(start) = svg.find("<rect") else {
        return svg.to_owned();
    };
    let Some(end_rel) = svg[start..].find("/>") else {
        return svg.to_owned();
    };
    let end = start + end_rel + 2;
    let tag = &svg[start..end];
    if !(tag.contains("fill=\"#FFFFFF\"")
        || tag.contains("fill=\"#333333\"")
        || tag.contains("fill=\"#ffffff\""))
        || tag.contains("rx=")
    {
        return svg.to_owned();
    }
    let replacement = format!(
        "<rect x=\"{x:.3}\" y=\"{y:.3}\" width=\"{width:.3}\" height=\"{height:.3}\" fill=\"{}\"/>",
        if tag.contains("#333") {
            "#333333"
        } else {
            "#FFFFFF"
        }
    );
    format!("{}{}{}", &svg[..start], replacement, &svg[end..])
}

fn replace_attr(tag: &str, name: &str, value: &str) -> String {
    let key = format!("{name}=\"");
    if let Some(start) = tag.find(&key) {
        let value_start = start + key.len();
        if let Some(end_rel) = tag[value_start..].find('"') {
            let end = value_start + end_rel;
            return format!("{}{}{}", &tag[..value_start], value, &tag[end..]);
        }
    }
    format!("{tag} {name}=\"{value}\"")
}

/// Fit a Mermaid SVG into the printable page area without stretching.
///
/// Simple diagrams stay compact near body-text scale. Large / complex diagrams
/// may use more of the page so labels remain readable.
fn mermaid_display_width_mm(svg: &str, options: &TypstOptions) -> f32 {
    const CSS_PX_TO_PT: f32 = 0.58;
    let (raw_width, raw_height) = svg_dimensions_pt(svg).unwrap_or((400.0, 300.0));
    let natural_width_pt = raw_width * CSS_PX_TO_PT;
    let natural_height_pt = raw_height * CSS_PX_TO_PT;
    let (content_width_pt, content_height_pt) = mermaid_page_content_pt(options);
    let complex = is_complex_mermaid(raw_width, raw_height);
    let (max_width_pt, max_height_pt, max_upscale, max_width_mm) = if complex {
        (
            content_width_pt * 0.92,
            content_height_pt * 0.72,
            1.15,
            165.0,
        )
    } else {
        (content_width_pt * 0.52, content_height_pt * 0.38, 1.0, 95.0)
    };
    let scale = (max_width_pt / natural_width_pt)
        .min(max_height_pt / natural_height_pt)
        .clamp(0.35, max_upscale);
    ((natural_width_pt * scale) / POINTS_PER_MM).clamp(40.0, max_width_mm)
}

fn is_complex_mermaid(width: f32, height: f32) -> bool {
    let area = width * height;
    area > 300_000.0 || width > 700.0 || height > 700.0
}

fn mermaid_page_content_pt(options: &TypstOptions) -> (f32, f32) {
    let (portrait_width_mm, portrait_height_mm) = page_dimensions_mm(options.page_size);
    let page_width_mm = if options.landscape {
        portrait_height_mm
    } else {
        portrait_width_mm
    };
    let page_height_mm = if options.landscape {
        portrait_width_mm
    } else {
        portrait_height_mm
    };
    let header_mm = if options.show_header { 8.0 } else { 0.0 };
    let content_width_pt = (page_width_mm - 2.0 * options.margin_mm) * POINTS_PER_MM;
    let content_height_pt = (page_height_mm - 2.0 * options.margin_mm - header_mm) * POINTS_PER_MM;
    (content_width_pt, content_height_pt)
}

fn svg_dimensions_pt(svg: &str) -> Option<(f32, f32)> {
    let width = svg_length_attr(svg, "width");
    let height = svg_length_attr(svg, "height");
    if let (Some(width), Some(height)) = (width, height)
        && width > 0.0
        && height > 0.0
    {
        return Some((width, height));
    }
    let view_box = svg_attr(svg, "viewBox")?;
    let parts = view_box
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() != 4 {
        return None;
    }
    let width = parts[2].parse::<f32>().ok()?;
    let height = parts[3].parse::<f32>().ok()?;
    (width > 0.0 && height > 0.0).then_some((width, height))
}

fn svg_length_attr(svg: &str, name: &str) -> Option<f32> {
    let value = svg_attr(svg, name)?;
    let numeric = value
        .trim()
        .trim_end_matches("px")
        .trim_end_matches("pt")
        .trim();
    numeric.parse::<f32>().ok().filter(|value| *value > 0.0)
}

fn svg_attr<'a>(svg: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let start = svg.find(&key)? + key.len();
    let end = start + svg[start..].find('"')?;
    Some(&svg[start..end])
}

fn normalize_fence_language(language: &str) -> String {
    let trimmed = language.trim().trim_start_matches('.');
    trimmed
        .strip_prefix("language-")
        .unwrap_or(trimmed)
        .to_ascii_lowercase()
}

fn styled_token(token: &StyledToken) -> String {
    let mut body = format!("#raw({})", typst_string(&token.content));
    if token.underline {
        body = format!("#underline[{body}]");
    }
    if token.strikethrough {
        body = format!("#strike[{body}]");
    }
    let mut properties = vec![format!("fill: rgb({})", typst_string(&token.color))];
    if token.bold {
        properties.push("weight: \"bold\"".to_owned());
    }
    if token.italic {
        properties.push("style: \"italic\"".to_owned());
    }
    format!("#text({})[{body}]", properties.join(", "))
}

fn max_code_columns(options: &TypstOptions) -> usize {
    let (portrait_width_mm, portrait_height_mm) = page_dimensions_mm(options.page_size);
    let page_width_mm = if options.landscape {
        portrait_height_mm
    } else {
        portrait_width_mm
    };
    let usable_points = (page_width_mm - 2.0 * options.margin_mm) * POINTS_PER_MM
        - 18.0
        - if options.line_numbers { 27.0 } else { 0.0 };
    (usable_points / CODE_GLYPH_WIDTH_PT)
        .floor()
        .clamp(40.0, 180.0) as usize
}

fn max_code_lines(options: &TypstOptions) -> usize {
    let (portrait_width_mm, portrait_height_mm) = page_dimensions_mm(options.page_size);
    let page_height_mm = if options.landscape {
        portrait_width_mm
    } else {
        portrait_height_mm
    };
    let usable_points = (page_height_mm - 2.0 * options.margin_mm) * POINTS_PER_MM - 32.0;
    (usable_points / CODE_LINE_HEIGHT_PT)
        .floor()
        .clamp(18.0, 55.0) as usize
}

fn page_dimensions_mm(page_size: PageSize) -> (f32, f32) {
    match page_size {
        PageSize::A4 => (210.0, 297.0),
        PageSize::Letter => (215.9, 279.4),
    }
}

fn wrap_code(source: &str, max_columns: usize) -> String {
    let mut output = String::with_capacity(source.len());
    for (line_index, line) in source.lines().enumerate() {
        if line_index > 0 {
            output.push('\n');
        }
        let mut remaining = line;
        let mut continuation = false;
        while remaining.chars().count() > max_columns {
            let prefix_width = usize::from(continuation) * 2;
            let available = max_columns.saturating_sub(prefix_width).max(1);
            let split_limit = char_boundary_at(remaining, available);
            let preferred_start = char_boundary_at(remaining, available * 3 / 5);
            let split = remaining[preferred_start..split_limit]
                .rfind(char::is_whitespace)
                .map(|offset| preferred_start + offset + 1)
                .unwrap_or(split_limit);
            if continuation {
                output.push_str("  ");
            }
            output.push_str(remaining[..split].trim_end());
            output.push('\n');
            remaining = remaining[split..].trim_start();
            continuation = true;
        }
        if continuation {
            output.push_str("  ");
        }
        output.push_str(remaining);
    }
    output
}

fn char_boundary_at(value: &str, character_index: usize) -> usize {
    value
        .char_indices()
        .nth(character_index)
        .map_or(value.len(), |(index, _)| index)
}

fn push_inline(
    paragraph: &mut Option<InlineBuffer>,
    heading: &mut Option<(HeadingLevel, InlineBuffer)>,
    typst: &str,
    plain: &str,
) {
    if let Some(buffer) = paragraph {
        buffer.typst.push_str(typst);
        buffer.plain.push_str(plain);
    } else if let Some((_, buffer)) = heading {
        buffer.typst.push_str(typst);
        buffer.plain.push_str(plain);
    }
}

fn typst_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    )
}

fn escape_markup_text(value: &str) -> String {
    format!("#text({})", typst_string(value))
}

fn heading_number(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn is_expected_result(plain: &str) -> bool {
    let value = plain.trim_start().to_lowercase();
    value.starts_with("expected result:")
}

fn parser_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> TypstOptions {
        TypstOptions {
            accent: "#C94C35".into(),
            code_theme: CodeTheme::Dark,
            line_numbers: true,
            title: "Test".into(),
            author: String::new(),
            label: "DOC".into(),
            footer: "Test".into(),
            page_size: PageSize::A4,
            landscape: false,
            margin_mm: 17.0,
            show_header: true,
            page_break_prefixes: vec![],
        }
    }

    #[test]
    fn extracts_plain_title() {
        assert_eq!(
            first_title("# **Title** `Rust`\n\nText").as_deref(),
            Some("Title Rust")
        );
        assert_eq!(first_title("No title"), None);
    }

    #[test]
    fn converts_markdown_and_code() {
        let document = to_typst(
            "# Test\n\nAn **example** with a [link](https://example.com).\n\n```rust\nfn main() {}\n```",
            &options(),
        )
        .expect("valid bundled highlighter");
        assert!(document.source.contains("= #text(\"Test\")"));
        assert!(document.source.contains("#link(\"https://example.com\")"));
        assert!(document.source.contains("1 │ fn main() {}"));
        assert!(document.source.contains("lang: \"rust\""));
        assert!(document.assets.is_empty());
    }

    #[test]
    fn wraps_long_code_without_splitting_unicode() {
        let wrapped = wrap_code(
            "let cafe = \"a line that is far too long to fit inside the code column\";",
            24,
        );
        assert!(wrapped.lines().all(|line| line.chars().count() <= 24));
        assert!(wrapped.contains("cafe"));
        assert!(wrapped.lines().count() >= 3);
    }

    #[test]
    fn splits_large_code_blocks_into_page_safe_chunks() {
        let code = (1..=120)
            .map(|line| format!("let value_{line} = {line};"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut highlighter = None;
        let typst = code_block(
            &code,
            "rust",
            100,
            50,
            true,
            CodeTheme::Dark,
            &mut highlighter,
        )
        .expect("highlight code");
        assert_eq!(typst.matches("#raw(block: true").count(), 3);
        assert!(typst.contains("120 │ let value_120 = 120;"));
    }

    #[test]
    fn escapes_typst_control_characters() {
        assert_eq!(
            typst_string("quote \" slash \\ tab\tline\n"),
            "\"quote \\\" slash \\\\ tab\\tline\\n\""
        );
    }

    #[test]
    fn renders_standalone_images_as_spaced_blocks() {
        let document = to_typst("# Image\n\n![Example](diagram.svg)", &options())
            .expect("valid bundled highlighter");
        assert!(
            document
                .source
                .contains("#image(\"diagram.svg\", width: 90%)")
        );
        assert!(document.source.contains("above: 7pt, below: 18pt"));
    }

    #[test]
    fn uses_each_code_background_declaration_once() {
        let document = to_typst("# Code\n\n```rust\nfn main() {}\n```", &options())
            .expect("valid bundled highlighter");
        assert_eq!(document.source.matches("fill: rgb(\"#0D1117\")").count(), 1);
    }

    #[test]
    fn renders_mermaid_fences_as_virtual_svg_images() {
        let document = to_typst(
            "# Diagram\n\n```mermaid\nflowchart LR\n    A --> B\n```\n",
            &options(),
        )
        .expect("render mermaid");
        assert_eq!(document.assets.len(), 1);
        assert_eq!(document.assets[0].0, "md2pdf-mermaid-0.svg");
        assert!(document.assets[0].1.starts_with(b"<svg"));
        let svg = String::from_utf8_lossy(&document.assets[0].1);
        assert!(svg.contains("font-family=\"DejaVu Sans\""));
        assert!(
            document
                .source
                .contains("#image(\"md2pdf-mermaid-0.svg\", width:")
        );
        assert!(document.source.contains("mm)"));
        assert!(!document.source.contains("lang: \"mermaid\""));
    }

    #[test]
    fn crops_excess_mermaid_canvas_padding() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"450\" height=\"265\" \
             viewBox=\"-50 -10 450 265\">\
             <rect x=\"-50\" y=\"-10\" width=\"450\" height=\"265\" fill=\"#FFFFFF\"/>\
             <rect x=\"0.00\" y=\"0.00\" width=\"150.00\" height=\"65.00\" rx=\"3\" ry=\"3\" \
             fill=\"#EAEAEA\" stroke=\"#666666\"/>\
             <rect x=\"200.00\" y=\"179.00\" width=\"150.00\" height=\"65.00\" rx=\"3\" ry=\"3\" \
             fill=\"#EAEAEA\" stroke=\"#666666\"/></svg>";
        let cropped = crop_mermaid_svg(svg);
        let (width, height) = svg_dimensions_pt(&cropped).expect("dimensions");
        assert!(
            width < 400.0 && height < 280.0,
            "expected tighter crop, got {width}x{height}: {cropped}"
        );
        assert!(cropped.contains("viewBox=\""));
    }

    #[test]
    fn keeps_flowchart_diamonds_inside_cropped_viewbox() {
        let document = to_typst(
            "# Decision\n\n```mermaid\nflowchart TD\n  B{Decision} --> C[OK]\n  B --> D[No]\n```\n",
            &options(),
        )
        .expect("render mermaid");
        let svg = String::from_utf8_lossy(&document.assets[0].1);
        let (vx, vy, vw, vh) = svg_view_box(&svg).expect("viewBox");
        let diamond = svg
            .match_indices("<polygon")
            .map(|(index, _)| {
                let end = svg[index..].find('>').expect("polygon end");
                &svg[index..index + end]
            })
            .find_map(|tag| {
                let points = svg_tag_attr(tag, "points")?;
                let bounds = polygon_bounds(points)?;
                ((bounds.2 - bounds.0) > 20.0 && (bounds.3 - bounds.1) > 20.0).then_some(bounds)
            })
            .expect("diamond polygon");
        for (x, y) in [
            (diamond.0, (diamond.1 + diamond.3) / 2.0),
            (diamond.2, (diamond.1 + diamond.3) / 2.0),
            ((diamond.0 + diamond.2) / 2.0, diamond.1),
            ((diamond.0 + diamond.2) / 2.0, diamond.3),
        ] {
            assert!(
                x >= vx && x <= vx + vw && y >= vy && y <= vy + vh,
                "diamond point ({x},{y}) outside viewBox {vx} {vy} {vw} {vh}"
            );
        }
    }

    #[test]
    fn crops_padded_diamond_without_clipping_vertices() {
        let svg = concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"400\" height=\"400\" ",
            "viewBox=\"-50 -50 400 400\">",
            "<rect x=\"-50\" y=\"-50\" width=\"400\" height=\"400\" fill=\"#FFFFFF\"/>",
            "<polygon points=\"100,20 180,100 100,180 20,100\" fill=\"#ccc\"/>",
            "<rect x=\"60\" y=\"220\" width=\"80\" height=\"40\" rx=\"3\" ry=\"3\" fill=\"#EAEAEA\"/>",
            "</svg>"
        );
        let cropped = crop_mermaid_svg(svg);
        let (vx, vy, vw, vh) = svg_view_box(&cropped).expect("viewBox");
        assert!(vw < 400.0 && vh < 400.0, "expected crop, got {vw}x{vh}");
        for (x, y) in [(100.0, 20.0), (180.0, 100.0), (100.0, 180.0), (20.0, 100.0)] {
            assert!(
                x >= vx && x <= vx + vw && y >= vy && y <= vy + vh,
                "diamond vertex ({x},{y}) outside cropped viewBox {vx} {vy} {vw} {vh}"
            );
        }
    }

    #[test]
    fn svg_tag_attr_ignores_prefixed_attribute_names() {
        let tag = "<ellipse cx=\"10\" cy=\"20\" rx=\"5\" ry=\"6\" stroke-width=\"2\" width=\"9\" x=\"3\"/>";
        assert_eq!(svg_tag_attr(tag, "x"), Some("3"));
        assert_eq!(svg_tag_attr(tag, "width"), Some("9"));
        assert_eq!(svg_tag_attr(tag, "cx"), Some("10"));
        assert_eq!(svg_tag_attr(tag, "rx"), Some("5"));
    }

    #[test]
    fn sizes_tall_mermaid_diagrams_to_the_page_box() {
        let svg = r#"<svg width="431" height="531" viewBox="0 0 431 531"></svg>"#;
        let width_mm = mermaid_display_width_mm(svg, &options());
        assert!(
            (70.0..95.0).contains(&width_mm),
            "unexpected width_mm={width_mm}"
        );
    }

    #[test]
    fn sizes_wide_mermaid_diagrams_near_content_width() {
        let svg = r#"<svg width="450" height="265" viewBox="0 0 450 265"></svg>"#;
        let width_mm = mermaid_display_width_mm(svg, &options());
        assert!(
            (70.0..95.0).contains(&width_mm),
            "unexpected width_mm={width_mm}"
        );
    }

    #[test]
    fn sizes_complex_mermaid_diagrams_larger() {
        let svg = r#"<svg width="900" height="800" viewBox="0 0 900 800"></svg>"#;
        let width_mm = mermaid_display_width_mm(svg, &options());
        assert!(
            (120.0..165.0).contains(&width_mm),
            "unexpected width_mm={width_mm}"
        );
        assert!(is_complex_mermaid(900.0, 800.0));
        assert!(!is_complex_mermaid(400.0, 300.0));
    }

    #[test]
    fn rejects_invalid_mermaid_diagrams() {
        let error = to_typst("# Broken\n\n```mermaid\nnot a diagram\n```\n", &options())
            .expect_err("invalid mermaid");
        assert!(error.to_string().contains("Mermaid diagram failed"));
    }
}
