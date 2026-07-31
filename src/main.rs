use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::Parser;
use md2pdf::{
    Result,
    cli::{Cli, OutputMode, default_jobs},
    error::Error,
    inputs::{self, InputSource},
    markdown::{TypstOptions, first_title, to_typst},
    pdf,
};
use rayon::prelude::*;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

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
    let sources = resolve_sources(&cli)?;
    let jobs = cli.jobs.unwrap_or_else(default_jobs).max(1);

    if sources.len() == 1 && sources[0].path == Path::new("-") {
        return run_stdin(&cli);
    }

    let converted = convert_sources(&cli, &sources, jobs)?;
    match cli.output_mode {
        OutputMode::Files => write_files(&cli, &sources, &converted)?,
        OutputMode::Merge => write_merge(&cli, &converted)?,
        OutputMode::Zip => write_zip(&cli, &sources, &converted)?,
    }
    Ok(())
}

fn resolve_sources(cli: &Cli) -> Result<Vec<InputSource>> {
    if !cli.sources.is_empty() && cli.legacy_source.is_some() {
        return Err(Error::ConflictingInputs);
    }
    let mut paths = cli.sources.clone();
    if let Some(legacy) = &cli.legacy_source {
        paths.push(legacy.clone());
    }
    if paths.is_empty() {
        return Err(Error::MissingInput);
    }

    let has_stdin = paths.iter().any(|path| path == Path::new("-"));
    if has_stdin {
        if paths.len() != 1 || cli.grep.is_some() {
            return Err(Error::StdinBatchUnsupported);
        }
        return Ok(vec![InputSource {
            path: PathBuf::from("-"),
            index: 1,
        }]);
    }

    inputs::collect_sources(&paths, cli.grep.as_deref())
}

fn validate(cli: &Cli) -> Result<()> {
    if cli.jobs == Some(0) {
        return Err(Error::InvalidJobs);
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
    match cli.output_mode {
        OutputMode::Merge if cli.output.is_none() => {
            return Err(Error::OutputRequiredForMode("merge"));
        }
        OutputMode::Zip if cli.output.is_none() => {
            return Err(Error::OutputRequiredForMode("zip"));
        }
        _ => {}
    }
    Ok(())
}

struct ConvertedDocument {
    bytes: Vec<u8>,
    pages: usize,
    warnings: Vec<String>,
    entry_name: String,
}

fn convert_sources(
    cli: &Cli,
    sources: &[InputSource],
    jobs: usize,
) -> Result<Vec<ConvertedDocument>> {
    let names = sources
        .iter()
        .map(|source| inputs::expand_name_format(&cli.name_format, source))
        .collect::<Result<Vec<_>>>()?;
    if matches!(cli.output_mode, OutputMode::Files | OutputMode::Zip) {
        inputs::ensure_unique_names(&names)?;
    }

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|error| Error::Pdf(format!("failed to start worker pool: {error}")))?;

    let results: Vec<Result<ConvertedDocument>> = pool.install(|| {
        sources
            .par_iter()
            .zip(names.par_iter())
            .map(|(source, entry_name)| convert_one(cli, source, entry_name.clone()))
            .collect()
    });

    results.into_iter().collect()
}

fn convert_one(cli: &Cli, source: &InputSource, entry_name: String) -> Result<ConvertedDocument> {
    let (markdown, source_dir) = read_source(&source.path)?;
    if markdown.trim().is_empty() {
        return Err(Error::EmptyInput);
    }
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
            source_dir: source_dir.clone(),
            allow_external: !cli.no_external,
            allow_http: cli.allow_http,
        },
    )?;
    let (bytes, pages) = pdf::render_to_bytes(&typst.source, &source_dir, &typst.assets)?;
    Ok(ConvertedDocument {
        bytes,
        pages,
        warnings: typst.warnings,
        entry_name,
    })
}

fn run_stdin(cli: &Cli) -> Result<()> {
    let output = cli.output.clone().ok_or(Error::OutputRequiredForStdin)?;
    if !matches!(cli.output_mode, OutputMode::Files) {
        return Err(Error::StdinBatchUnsupported);
    }
    let (markdown, source_dir) = read_source(Path::new("-"))?;
    if markdown.trim().is_empty() {
        return Err(Error::EmptyInput);
    }
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
            source_dir: source_dir.clone(),
            allow_external: !cli.no_external,
            allow_http: cli.allow_http,
        },
    )?;
    emit_warnings(cli, &typst.warnings);
    let pages = pdf::render(&typst.source, &output, &source_dir, &typst.assets)?;
    if !cli.quiet {
        println!(
            "PDF generated ({pages} page(s)): {}",
            absolute(&output).display()
        );
    }
    Ok(())
}

fn write_files(cli: &Cli, sources: &[InputSource], converted: &[ConvertedDocument]) -> Result<()> {
    if sources.len() == 1 {
        if let Some(output) = &cli.output
            && !(output.exists() && output.is_dir())
        {
            emit_warnings(cli, &converted[0].warnings);
            pdf::write_bytes(output, &converted[0].bytes)?;
            if !cli.quiet {
                println!(
                    "PDF generated ({} page(s)): {}",
                    converted[0].pages,
                    absolute(output).display()
                );
            }
            return Ok(());
        }
    } else if let Some(output) = &cli.output
        && output.exists()
        && !output.is_dir()
    {
        return Err(Error::OutputMustBeDirectory(output.clone()));
    }

    if let Some(dir) = &cli.output {
        fs::create_dir_all(dir).map_err(|source| Error::Write {
            path: dir.clone(),
            source,
        })?;
    }

    for (source, document) in sources.iter().zip(converted.iter()) {
        emit_warnings(cli, &document.warnings);
        let path = match &cli.output {
            Some(dir) => dir.join(&document.entry_name),
            None => source
                .path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&document.entry_name),
        };
        pdf::write_bytes(&path, &document.bytes)?;
        if !cli.quiet {
            println!(
                "PDF generated ({} page(s)): {}",
                document.pages,
                absolute(&path).display()
            );
        }
    }
    Ok(())
}

fn write_merge(cli: &Cli, converted: &[ConvertedDocument]) -> Result<()> {
    let output = cli
        .output
        .as_ref()
        .ok_or(Error::OutputRequiredForMode("merge"))?;
    for document in converted {
        emit_warnings(cli, &document.warnings);
    }
    let parts: Vec<Vec<u8>> = converted.iter().map(|doc| doc.bytes.clone()).collect();
    let merged = pdf::merge_pdfs(&parts)?;
    let pages: usize = converted.iter().map(|doc| doc.pages).sum();
    pdf::write_bytes(output, &merged)?;
    if !cli.quiet {
        println!(
            "PDF generated ({pages} page(s)): {}",
            absolute(output).display()
        );
    }
    Ok(())
}

fn write_zip(cli: &Cli, _sources: &[InputSource], converted: &[ConvertedDocument]) -> Result<()> {
    let output = cli
        .output
        .as_ref()
        .ok_or(Error::OutputRequiredForMode("zip"))?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let file = File::create(output).map_err(|source| Error::Write {
        path: output.clone(),
        source,
    })?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut total_pages = 0usize;
    for document in converted {
        emit_warnings(cli, &document.warnings);
        zip.start_file(&document.entry_name, options)
            .map_err(|error| Error::Zip {
                path: output.clone(),
                message: error.to_string(),
            })?;
        zip.write_all(&document.bytes).map_err(|error| Error::Zip {
            path: output.clone(),
            message: error.to_string(),
        })?;
        total_pages += document.pages;
    }
    zip.finish().map_err(|error| Error::Zip {
        path: output.clone(),
        message: error.to_string(),
    })?;
    if !cli.quiet {
        println!(
            "ZIP generated ({} file(s), {total_pages} page(s)): {}",
            converted.len(),
            absolute(output).display()
        );
    }
    Ok(())
}

fn emit_warnings(cli: &Cli, warnings: &[String]) {
    if cli.quiet {
        return;
    }
    for warning in warnings {
        eprintln!("md2pdf: warning: {warning}");
    }
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

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}
