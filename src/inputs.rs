//! Discover Markdown inputs from files/directories/globs and expand name templates.

use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use walkdir::WalkDir;

use crate::{Error, Result};

const DEFAULT_DIR_GREP: &str = "**/*.md";

/// A Markdown source selected for conversion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputSource {
    /// Absolute or user-supplied path to the Markdown file, or `-` for stdin.
    pub path: PathBuf,
    /// 1-based index in the collected input list.
    pub index: usize,
}

/// Collect Markdown inputs from files, directories, and glob patterns.
///
/// Positional arguments that do not exist on disk but contain glob metacharacters
/// (for example `./**/*.md`) are expanded in place. When any source is a
/// directory, files are filtered with `grep` (default `**/*.md`). Explicit files
/// and glob matches are only filtered when `grep` is set.
pub fn collect_sources(sources: &[PathBuf], grep: Option<&str>) -> Result<Vec<InputSource>> {
    if sources.is_empty() {
        return Err(Error::MissingInput);
    }

    let has_directory = sources.iter().any(|path| path.is_dir());
    let grep_pattern = match (grep, has_directory) {
        (Some(pattern), _) => Some(pattern),
        (None, true) => Some(DEFAULT_DIR_GREP),
        (None, false) => None,
    };
    let globset = grep_pattern.map(build_globset).transpose()?;
    let filter_matches = grep.is_some();

    let mut unique = BTreeSet::new();
    for source in sources {
        if source == Path::new("-") {
            unique.insert(source.clone());
            continue;
        }
        if source.is_dir() {
            collect_from_directory(source, globset.as_ref(), &mut unique)?;
        } else if source.is_file() {
            if filter_matches
                && let Some(set) = &globset
                && !matches_grep(source, source, set)
            {
                continue;
            }
            unique.insert(source.clone());
        } else if looks_like_glob(source) {
            let extra_filter = if filter_matches {
                globset.as_ref()
            } else {
                None
            };
            expand_glob_source(source, extra_filter, &mut unique)?;
        } else {
            return Err(Error::InputNotFound(source.clone()));
        }
    }

    if unique.is_empty() {
        return Err(Error::NoMatchingInputs);
    }

    Ok(unique
        .into_iter()
        .enumerate()
        .map(|(offset, path)| InputSource {
            path,
            index: offset + 1,
        })
        .collect())
}

fn looks_like_glob(path: &Path) -> bool {
    path.to_string_lossy()
        .chars()
        .any(|ch| matches!(ch, '*' | '?' | '['))
}

fn expand_glob_source(
    pattern: &Path,
    grep: Option<&GlobSet>,
    unique: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let pattern = pattern.to_string_lossy();
    let entries =
        glob::glob(pattern.as_ref()).map_err(|error| Error::InvalidGrep(error.to_string()))?;
    let mut matched = false;
    for entry in entries {
        let path = entry.map_err(|error| Error::Read {
            path: PathBuf::from(pattern.as_ref()),
            source: std::io::Error::other(error.to_string()),
        })?;
        if !path.is_file() {
            continue;
        }
        if let Some(set) = grep {
            let root = path.parent().unwrap_or_else(|| Path::new("."));
            if !matches_grep(&path, root, set) {
                continue;
            }
        }
        unique.insert(path);
        matched = true;
    }
    if !matched {
        // Pattern parsed but matched nothing — surface as empty selection later
        // unless other sources contribute files.
        return Ok(());
    }
    Ok(())
}

fn collect_from_directory(
    root: &Path,
    globset: Option<&GlobSet>,
    unique: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let walker = WalkDir::new(root).follow_links(false).into_iter();
    for entry in walker {
        let entry = entry.map_err(|error| Error::Read {
            path: root.to_path_buf(),
            source: std::io::Error::other(error.to_string()),
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if let Some(set) = globset
            && !matches_grep(path, root, set)
        {
            continue;
        }
        unique.insert(path.to_path_buf());
    }
    Ok(())
}

fn build_globset(pattern: &str) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    let glob = Glob::new(pattern).map_err(|error| Error::InvalidGrep(error.to_string()))?;
    builder.add(glob);
    builder
        .build()
        .map_err(|error| Error::InvalidGrep(error.to_string()))
}

fn matches_grep(path: &Path, root: &Path, globset: &GlobSet) -> bool {
    let candidates = [
        path.to_string_lossy().into_owned(),
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path.strip_prefix(root)
            .ok()
            .map(|relative| relative.to_string_lossy().into_owned())
            .unwrap_or_default(),
    ];
    candidates
        .iter()
        .any(|candidate| !candidate.is_empty() && globset.is_match(candidate))
}

/// Expand a `--name-format` template for one input.
pub fn expand_name_format(template: &str, source: &InputSource) -> Result<String> {
    validate_name_format(template)?;
    let path = &source.path;
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document.md".to_owned());
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".to_owned());
    let ext = path
        .extension()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = path
        .parent()
        .and_then(Path::file_name)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_owned());

    Ok(template
        .replace("{stem}", &stem)
        .replace("{name}", &name)
        .replace("{dir}", &dir)
        .replace("{index}", &source.index.to_string())
        .replace("{ext}", &ext))
}

/// Ensure two inputs never resolve to the same output name.
pub fn ensure_unique_names(names: &[String]) -> Result<()> {
    let mut seen = HashMap::new();
    for name in names {
        if seen.insert(name.as_str(), ()).is_some() {
            return Err(Error::NameCollision(name.clone()));
        }
    }
    Ok(())
}

fn validate_name_format(template: &str) -> Result<()> {
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            return Err(Error::InvalidNameFormat(
                "unclosed `{` in name format".to_owned(),
            ));
        };
        let token = &after[..end];
        match token {
            "stem" | "name" | "dir" | "index" | "ext" => {}
            other => {
                return Err(Error::InvalidNameFormat(format!(
                    "unknown placeholder `{{{other}}}`"
                )));
            }
        }
        rest = &after[end + 1..];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::tempdir;

    #[test]
    fn collects_explicit_files_without_grep() {
        let directory = tempdir().expect("tempdir");
        let a = directory.path().join("a.md");
        let b = directory.path().join("b.md");
        fs::write(&a, "# A").unwrap();
        fs::write(&b, "# B").unwrap();
        let sources = collect_sources(&[a.clone(), b.clone()], None).expect("collect");
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].index, 1);
        assert_eq!(sources[1].index, 2);
    }

    #[test]
    fn greps_directory_contents() {
        let directory = tempdir().expect("tempdir");
        let keep = directory.path().join("keep.md");
        let skip = directory.path().join("skip.txt");
        let nested = directory.path().join("nested");
        fs::create_dir_all(&nested).unwrap();
        let adr = nested.join("ADR-1.md");
        fs::write(&keep, "# Keep").unwrap();
        fs::write(&skip, "nope").unwrap();
        fs::write(&adr, "# ADR").unwrap();

        let sources = collect_sources(&[directory.path().to_path_buf()], Some("**/ADR-*.md"))
            .expect("collect");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].path, adr);

        let all_md = collect_sources(&[directory.path().to_path_buf()], None).expect("default");
        assert_eq!(all_md.len(), 2);
    }

    #[test]
    fn expands_positional_glob_patterns() {
        let directory = tempdir().expect("tempdir");
        let nested = directory.path().join("nested");
        fs::create_dir_all(&nested).unwrap();
        let keep = nested.join("keep.md");
        let skip = nested.join("skip.txt");
        fs::write(&keep, "# Keep").unwrap();
        fs::write(&skip, "nope").unwrap();

        let pattern = directory.path().join("**").join("*.md");
        let sources = collect_sources(&[pattern], None).expect("glob");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].path, keep);
    }

    #[test]
    fn expands_name_format_placeholders() {
        let source = InputSource {
            path: PathBuf::from("docs/guide.md"),
            index: 3,
        };
        assert_eq!(
            expand_name_format("{dir}/{stem}-{index}.{ext}.pdf", &source).unwrap(),
            "docs/guide-3.md.pdf"
        );
    }

    #[test]
    fn rejects_unknown_placeholders_and_collisions() {
        let source = InputSource {
            path: PathBuf::from("a.md"),
            index: 1,
        };
        assert!(expand_name_format("{title}.pdf", &source).is_err());
        assert!(ensure_unique_names(&["a.pdf".into(), "a.pdf".into()]).is_err());
        assert!(ensure_unique_names(&["a.pdf".into(), "b.pdf".into()]).is_ok());
    }
}
