pub mod fingerprint;
pub mod git;

use std::path::Path;

use anyhow::Result;

use crate::frontmatter::scan_docs;
use crate::models::{CleanDoc, DriftReport, DriftedAnchor, DriftedDoc};

use self::fingerprint::{compute_sig, SIG_PREFIX};
use self::git::{diff_with_summary, head_sha, read_file_at_rev};

/// Normalise a git remote URL to a canonical path string for comparison.
fn repo_path_from_url(url: &str) -> &str {
    url.strip_prefix("file://").unwrap_or(url)
}

/// Return true when `anchor_repo` refers to the same repository as `code_repo_url`.
fn anchor_matches_repo(anchor_repo: &str, code_repo_url: &str) -> bool {
    repo_path_from_url(anchor_repo) == repo_path_from_url(code_repo_url)
}

/// Run the detection pipeline over a docs directory against a local code repository.
///
/// * `code_repo_path` – filesystem path to the checked-out code repository.
/// * `docs_dir` – directory tree scanned for `*.md` files with steer frontmatter.
/// * `code_repo_url` – canonical URL (or `file://` path) to match anchor `repo:` fields.
/// * `repo_name` – short name used to populate `DriftReport::repo`.
pub fn detect_drift(
    code_repo_path: &Path,
    docs_dir: impl AsRef<Path>,
    code_repo_url: &str,
    repo_name: &str,
) -> Result<DriftReport> {
    let docs_dir = docs_dir.as_ref();
    let current_sha = head_sha(code_repo_path)?;
    let docs = scan_docs(docs_dir, code_repo_url, None);

    let mut drifted: Vec<DriftedDoc> = Vec::new();
    let mut clean: Vec<CleanDoc> = Vec::new();

    for doc in &docs {
        let relevant_anchors: Vec<_> = doc
            .frontmatter
            .anchors
            .iter()
            .filter(|a| anchor_matches_repo(&a.repo, code_repo_url))
            .collect();

        if relevant_anchors.is_empty() {
            continue;
        }

        let mut drifted_anchors: Vec<DriftedAnchor> = Vec::new();

        for anchor in &relevant_anchors {
            let symbol = anchor.symbol.as_deref();

            let is_drifted = if anchor.provenance.starts_with(SIG_PREFIX) {
                // Content-addressed provenance: compare fingerprint directly
                // No git history needed — just read the current file
                let current_content = std::fs::read_to_string(
                    code_repo_path.join(&anchor.path),
                ).or_else(|_| {
                    // Fall back to reading from HEAD commit
                    read_file_at_rev(code_repo_path, &current_sha, &anchor.path)
                })?;

                let current_sig = compute_sig(&current_content, &anchor.path, symbol);
                current_sig != anchor.provenance
            } else {
                // Legacy git SHA provenance: read at both revisions and compare fingerprints
                let content_at_provenance =
                    read_file_at_rev(code_repo_path, &anchor.provenance, &anchor.path)?;
                let content_at_head =
                    read_file_at_rev(code_repo_path, &current_sha, &anchor.path)?;

                let sig_provenance = compute_sig(&content_at_provenance, &anchor.path, symbol);
                let sig_head = compute_sig(&content_at_head, &anchor.path, symbol);
                sig_provenance != sig_head
            };

            if is_drifted {
                let (diff, diff_sum) = if anchor.provenance.starts_with(SIG_PREFIX) {
                    (String::new(), "content fingerprint changed".to_string())
                } else {
                    diff_with_summary(code_repo_path, &anchor.provenance, &anchor.path)
                        .unwrap_or_default()
                };

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
