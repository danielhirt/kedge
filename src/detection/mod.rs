pub mod fingerprint;
pub mod git;

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::frontmatter::scan_docs;
use crate::models::{CleanDoc, DriftReport, DriftedAnchor, DriftedDoc};

use self::fingerprint::{Language, ast_fingerprint, content_hash};
use self::git::{diff_since, diff_summary, head_sha, read_file_at_rev};

/// Normalise a git remote URL to a canonical path string for comparison.
///
/// For `file:///some/path` this returns `/some/path`.
/// For other URLs (https://, git@…) we return the URL unchanged for an
/// exact-string comparison.
fn repo_path_from_url(url: &str) -> &str {
    url.strip_prefix("file://").unwrap_or(url)
}

/// Return true when `anchor_repo` refers to the same repository as
/// `code_repo_url`.  We compare the canonical path/URL strings so that
/// `file:///tmp/x` matches `file:///tmp/x` but also handles the case where
/// one side carries the `file://` prefix and the other doesn't.
fn anchor_matches_repo(anchor_repo: &str, code_repo_url: &str) -> bool {
    repo_path_from_url(anchor_repo) == repo_path_from_url(code_repo_url)
}

/// Compute a stable fingerprint for `content` using AST when the language is
/// supported, falling back to a content-hash otherwise.
fn fingerprint(content: &str, path: &str, symbol: Option<&str>) -> String {
    let ext = PathBuf::from(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();

    match Language::from_extension(&ext) {
        Some(lang) => ast_fingerprint(content, lang, symbol).unwrap_or_else(|_| content_hash(content)),
        None => content_hash(content),
    }
}

/// Run the detection pipeline over a docs directory against a local code repository.
///
/// * `code_repo_path`  – filesystem path to the checked-out code repository.
/// * `docs_dir`        – directory tree that is scanned for `*.md` files with
///                       steer frontmatter.
/// * `code_repo_url`   – the canonical URL (or `file://` path) used to match
///                       anchor `repo:` fields against this repository.
/// * `repo_name`       – short name used to populate `DriftReport::repo`.
pub fn detect_drift(
    code_repo_path: &Path,
    docs_dir: impl AsRef<Path>,
    code_repo_url: &str,
    repo_name: &str,
) -> Result<DriftReport> {
    let docs_dir = docs_dir.as_ref();

    // 1. Current HEAD of the code repository.
    let current_sha = head_sha(code_repo_path)?;

    // 2. Scan docs directory — no group filter here.
    let docs = scan_docs(docs_dir, "docs", None);

    let mut drifted: Vec<DriftedDoc> = Vec::new();
    let mut clean: Vec<CleanDoc> = Vec::new();

    // 3. For each doc and each anchor decide whether it has drifted.
    for doc in &docs {
        let relevant_anchors: Vec<_> = doc
            .frontmatter
            .anchors
            .iter()
            .filter(|a| anchor_matches_repo(&a.repo, code_repo_url))
            .collect();

        if relevant_anchors.is_empty() {
            // This doc has no anchors pointing at our repo – skip it.
            continue;
        }

        let mut drifted_anchors: Vec<DriftedAnchor> = Vec::new();

        for anchor in &relevant_anchors {
            // Read file at provenance commit and at HEAD.
            let content_at_provenance =
                read_file_at_rev(code_repo_path, &anchor.provenance, &anchor.path)?;
            let content_at_head =
                read_file_at_rev(code_repo_path, &current_sha, &anchor.path)?;

            let symbol = anchor.symbol.as_deref();
            let fp_provenance = fingerprint(&content_at_provenance, &anchor.path, symbol);
            let fp_head = fingerprint(&content_at_head, &anchor.path, symbol);

            if fp_provenance != fp_head {
                let diff = diff_since(code_repo_path, &anchor.provenance, &anchor.path)
                    .unwrap_or_default();
                let diff_sum =
                    diff_summary(code_repo_path, &anchor.provenance, &anchor.path)
                        .unwrap_or_default();

                drifted_anchors.push(DriftedAnchor {
                    path: anchor.path.clone(),
                    symbol: anchor.symbol.clone(),
                    provenance: anchor.provenance.clone(),
                    current_commit: current_sha.clone(),
                    diff_summary: diff_sum,
                    diff,
                });
            }
        }

        if drifted_anchors.is_empty() {
            clean.push(CleanDoc {
                doc: doc.path.clone(),
                anchor_count: relevant_anchors.len(),
            });
        } else {
            drifted.push(DriftedDoc {
                doc: doc.path.clone(),
                doc_repo: doc.doc_repo.clone(),
                anchors: drifted_anchors,
            });
        }
    }

    Ok(DriftReport {
        repo: repo_name.to_string(),
        git_ref: "HEAD".to_string(),
        commit: current_sha,
        drifted,
        clean,
    })
}
