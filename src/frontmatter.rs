use std::path::Path;

use anyhow::Context;
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

/// Update the provenance for a specific anchor in a markdown file.
/// For single-anchor updates. Prefer `update_provenance_batch` when
/// updating multiple anchors in the same file to avoid N reads/writes.
pub fn update_provenance(
    path: &Path,
    anchor_path: &str,
    anchor_symbol: Option<&str>,
    new_provenance: &str,
) -> Result<(), anyhow::Error> {
    update_provenance_batch(path, &[(anchor_path, anchor_symbol, new_provenance)])
}

/// Update provenance for multiple anchors in a single file with one read/write.
/// Each tuple is (anchor_path, anchor_symbol, new_provenance).
pub fn update_provenance_batch(
    path: &Path,
    updates: &[(&str, Option<&str>, &str)],
) -> Result<(), anyhow::Error> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let (yaml, body) = extract_frontmatter(&text)
        .ok_or_else(|| anyhow::anyhow!("no frontmatter in {}", path.display()))?;

    let mut root: serde_yaml::Value = serde_yaml::from_str(yaml)
        .with_context(|| format!("failed to parse frontmatter YAML in {}", path.display()))?;

    let anchors = root
        .get_mut("steer")
        .and_then(|s| s.get_mut("anchors"))
        .and_then(|a| a.as_sequence_mut())
        .ok_or_else(|| anyhow::anyhow!("no steer.anchors in {}", path.display()))?;

    for (upd_path, upd_symbol, upd_provenance) in updates {
        for anchor in anchors.iter_mut() {
            let path_match = anchor
                .get("path")
                .and_then(|v| v.as_str())
                .map(|p| p == *upd_path)
                .unwrap_or(false);

            let symbol_match = match upd_symbol {
                Some(sym) => anchor
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .map(|s| s == *sym)
                    .unwrap_or(false),
                None => anchor.get("symbol").is_none() || anchor["symbol"].is_null(),
            };

            if path_match && symbol_match {
                if let Some(map) = anchor.as_mapping_mut() {
                    map.insert(
                        serde_yaml::Value::String("provenance".to_string()),
                        serde_yaml::Value::String(upd_provenance.to_string()),
                    );
                }
                break;
            }
        }
    }

    let new_yaml = serde_yaml::to_string(&root)
        .context("failed to serialize updated frontmatter")?;
    let new_yaml = new_yaml.trim_end_matches('\n');

    let new_text = format!("---\n{}\n---\n{}", new_yaml, body);
    std::fs::write(path, new_text)
        .with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}
