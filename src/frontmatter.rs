use std::path::Path;

use serde::Deserialize;

use crate::models::{DocFile, SteerFrontmatter};

/// Raw YAML representation of the full frontmatter, used to extract the `steer` sub-key.
#[derive(Debug, Deserialize)]
struct RawFrontmatter {
    steer: Option<SteerFrontmatter>,
}

/// Split a markdown document into `(yaml_text, body_text)` at the `---` delimiters.
///
/// Returns `None` if the document does not start with `---` or has no closing delimiter.
pub fn extract_frontmatter(text: &str) -> Option<(&str, &str)> {
    let text = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n"))?;
    // Find the closing `---`
    let close = text.find("\n---\n")
        .map(|i| (i, i + 5))
        .or_else(|| text.find("\n---\r\n").map(|i| (i, i + 6)))
        .or_else(|| {
            // Handle trailing `---` at end of file (no newline after)
            if let Some(i) = text.rfind("\n---") {
                let rest = &text[i + 4..];
                if rest.trim_end_matches(['\r', '\n']).is_empty() {
                    return Some((i, text.len()));
                }
            }
            None
        })?;

    let yaml = &text[..close.0];
    let body = &text[close.1..];
    Some((yaml, body))
}

/// Parse a markdown string and return a `DocFile` if it contains a `steer:` block with anchors.
///
/// `doc_path` is stored as the file path in the returned `DocFile`.
/// `doc_repo` is the git remote URL of the repository containing this doc.
pub fn parse_doc_string(text: &str, doc_path: &str, doc_repo: &str) -> Option<DocFile> {
    let (yaml, body) = extract_frontmatter(text)?;

    let raw: RawFrontmatter = serde_yaml::from_str(yaml).ok()?;
    let frontmatter = raw.steer?;

    if frontmatter.anchors.is_empty() {
        return None;
    }

    Some(DocFile {
        path: doc_path.to_string(),
        doc_repo: doc_repo.to_string(),
        frontmatter,
        content: body.to_string(),
        raw_frontmatter: yaml.to_string(),
    })
}

/// Read a markdown file and return a `DocFile` if it contains a `steer:` block with anchors.
pub fn parse_doc_file(path: &Path, doc_repo: &str) -> Option<DocFile> {
    let text = std::fs::read_to_string(path).ok()?;
    let doc_path = path.to_string_lossy().into_owned();
    parse_doc_string(&text, &doc_path, doc_repo)
}

/// Scan a directory recursively for `.md` files with steer frontmatter.
///
/// If `group_filter` is `Some`, only docs whose `steer.group` matches the filter are returned.
pub fn scan_docs(dir: &Path, doc_repo: &str, group_filter: Option<&str>) -> Vec<DocFile> {
    let pattern = format!("{}/**/*.md", dir.to_string_lossy());
    let paths = glob::glob(&pattern).into_iter().flatten().flatten();

    paths
        .filter_map(|path| parse_doc_file(&path, doc_repo))
        .filter(|doc| {
            if let Some(filter) = group_filter {
                doc.frontmatter.group.as_deref() == Some(filter)
            } else {
                true
            }
        })
        .collect()
}
