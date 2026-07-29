use std::{fs, path::Path};

use typst_as_lib::TypstEngine;
use typst_layout::PagedDocument;

use crate::{Error, Result};

const FONTS: [&[u8]; 6] = [
    include_bytes!("../assets/fonts/DejaVuSans.ttf"),
    include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf"),
    include_bytes!("../assets/fonts/DejaVuSans-Oblique.ttf"),
    include_bytes!("../assets/fonts/DejaVuSans-BoldOblique.ttf"),
    include_bytes!("../assets/fonts/DejaVuSansMono.ttf"),
    include_bytes!("../assets/fonts/DejaVuSansMono-Bold.ttf"),
];
const DARK_THEME: &[u8] = include_bytes!("../assets/themes/md2pdf-dark.tmTheme");

/// Compile Typst source and write the resulting PDF.
///
/// Relative images and other local assets are resolved from `source_dir`.
/// Virtual `assets` (for example rendered Mermaid SVGs) are resolved in memory.
/// The returned value is the generated page count.
pub fn render(
    source: &str,
    output: &Path,
    source_dir: &Path,
    assets: &[(String, Vec<u8>)],
) -> Result<usize> {
    let mut binaries: Vec<(&str, &[u8])> = Vec::with_capacity(assets.len() + 1);
    binaries.push(("md2pdf-dark.tmTheme", DARK_THEME));
    for (name, bytes) in assets {
        binaries.push((name.as_str(), bytes.as_slice()));
    }

    let engine = TypstEngine::builder()
        .main_file(source)
        .fonts(FONTS)
        .with_static_file_resolver(binaries)
        .with_file_system_resolver(source_dir)
        .build();
    let result = engine.compile::<PagedDocument>();
    let document = result
        .output
        .map_err(|error| Error::Pdf(format!("{error:?}")))?;
    drop(engine);
    let page_count = document.pages().len();
    let bytes = typst_pdf::pdf(&document, &Default::default())
        .map_err(|error| Error::Pdf(format!("{error:?}")))?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|source| Error::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(output, bytes).map_err(|source| Error::Write {
        path: output.to_path_buf(),
        source,
    })?;
    Ok(page_count)
}
