use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::cli::{CodeTheme, PageSize};

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
#[must_use]
pub fn to_typst(markdown: &str, options: &TypstOptions) -> String {
    let parser = Parser::new_ext(markdown, parser_options());
    let mut body = String::new();
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
                    body.push_str(&code_block(
                        &source,
                        &language,
                        max_code_columns(options),
                        max_code_lines(options),
                        options.line_numbers,
                    ));
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

    format!("{}\n{}", template(options), body)
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
    let code_fill = match options.code_theme {
        CodeTheme::Dark => "#101828",
        CodeTheme::Light => "#F4F6F8",
    };
    let code_text = match options.code_theme {
        CodeTheme::Dark => "#E4E7EC",
        CodeTheme::Light => "#17202A",
    };
    let raw_theme = match options.code_theme {
        CodeTheme::Dark => {
            "#set raw(theme: \"md2pdf-dark.tmTheme\", \
             syntaxes: \"md2pdf-rust.sublime-syntax\")"
        }
        CodeTheme::Light => "#set raw(syntaxes: \"md2pdf-rust.sublime-syntax\")",
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
) -> String {
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
        output.push_str(&format!(
            "#raw(block: true, lang: {}, {})\n",
            typst_string(language),
            typst_string(chunk)
        ));
    }
    output.push_str("#v(18pt, weak: true)\n\n");
    output
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
        let source = to_typst(
            "# Test\n\nAn **example** with a [link](https://example.com).\n\n```rust\nfn main() {}\n```",
            &options(),
        );
        assert!(source.contains("= #text(\"Test\")"));
        assert!(source.contains("#link(\"https://example.com\")"));
        assert!(source.contains("lang: \"rust\""));
        assert!(source.contains("1 │ fn main() {}"));
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
        let typst = code_block(&code, "rust", 100, 50, true);
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
        let typst = to_typst("# Image\n\n![Example](diagram.svg)", &options());
        assert!(typst.contains("#image(\"diagram.svg\", width: 90%)"));
        assert!(typst.contains("above: 7pt, below: 18pt"));
    }

    #[test]
    fn uses_each_code_background_declaration_once() {
        let typst = to_typst("# Code\n\n```rust\nfn main() {}\n```", &options());
        assert_eq!(typst.matches("fill: rgb(\"#101828\")").count(), 1);
    }
}
