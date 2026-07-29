use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::Parser;
use md2pdf::{
    Result,
    cli::Cli,
    error::Error,
    markdown::{TypstOptions, first_title, to_typst},
    pdf,
};

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("md2pdf: error: {error}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    validate(&cli)?;
    let source = cli
        .source
        .as_ref()
        .or(cli.legacy_source.as_ref())
        .ok_or(Error::MissingInput)?;
    let (markdown, source_dir) = read_source(source)?;
    if markdown.trim().is_empty() {
        return Err(Error::EmptyInput);
    }
    let output = output_path(source, cli.output.as_ref())?;
    let title = cli
        .title
        .clone()
        .or_else(|| first_title(&markdown))
        .unwrap_or_else(|| "Markdown Document".to_owned());
    let typst = to_typst(
        &markdown,
        &TypstOptions {
            accent: cli.accent.clone(),
            code_theme: cli.code_theme,
            line_numbers: cli.line_numbers,
            title: title.clone(),
            author: cli.author.clone(),
            label: cli.label.clone(),
            footer: cli.footer.clone().unwrap_or_else(|| title.clone()),
            page_size: cli.page_size,
            landscape: cli.landscape,
            margin_mm: cli.margin,
            show_header: !cli.no_header,
            page_break_prefixes: cli.page_break_before.clone(),
        },
    )?;
    let pages = pdf::render(&typst.source, &output, &source_dir, &typst.assets)?;
    if !cli.quiet {
        println!(
            "PDF generated ({pages} page(s)): {}",
            absolute(&output).display()
        );
    }
    Ok(())
}

fn validate(cli: &Cli) -> Result<()> {
    if cli.source.is_some() && cli.legacy_source.is_some() {
        return Err(Error::ConflictingInputs);
    }
    if !(8.0..=45.0).contains(&cli.margin) {
        return Err(Error::InvalidMargin);
    }
    let valid_accent = cli.accent.len() == 7
        && cli.accent.starts_with('#')
        && cli.accent[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid_accent {
        return Err(Error::InvalidAccent);
    }
    Ok(())
}

fn read_source(path: &Path) -> Result<(String, PathBuf)> {
    if path == Path::new("-") {
        let mut markdown = String::new();
        io::stdin()
            .read_to_string(&mut markdown)
            .map_err(|source| Error::Read {
                path: PathBuf::from("<stdin>"),
                source,
            })?;
        return Ok((
            markdown,
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        ));
    }
    if !path.is_file() {
        return Err(Error::InputNotFound(path.to_path_buf()));
    }
    let markdown = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    Ok((
        markdown,
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
    ))
}

fn output_path(source: &Path, requested: Option<&PathBuf>) -> Result<PathBuf> {
    if let Some(path) = requested {
        return Ok(path.clone());
    }
    if source == Path::new("-") {
        return Err(Error::OutputRequiredForStdin);
    }
    Ok(source.with_extension("pdf"))
}

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}
