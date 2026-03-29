pub mod fingerprint;
pub mod git;

use std::path::Path;

use anyhow::{Context, Result};

use crate::frontmatter::scan_docs;
use crate::models::{CleanDoc, DriftReport, DriftedAnchor, DriftedDoc};
use crate::safety;

use self::fingerprint::{compute_sig, SIG_PREFIX};
use self::git::{diff_with_summary, head_sha, read_file_at_rev};

/// Convert an absolute doc path to a path relative to the docs repo root.
/// Falls back to canonicalized comparison, then returns the original on failure.
fn make_repo_relative(abs_path: &str, repo_root: &Path) -> String {
    let p = Path::new(abs_path);
    if let Ok(rel) = p.strip_prefix(repo_root) {
        return rel.to_string_lossy().into_owned();
    }
    // Try canonicalized paths (handles symlinks, e.g. /var → /private/var on macOS)
    if let (Ok(canon_p), Ok(canon_root)) = (p.canonicalize(), repo_root.canonicalize()) {
        if let Ok(rel) = canon_p.strip_prefix(&canon_root) {
            return rel.to_string_lossy().into_owned();
        }
    }
    abs_path.to_string()
}

fn repo_path_from_url(url: &str) -> &str {
    url.strip_prefix("file://").unwrap_or(url)
}

fn anchor_matches_repo(anchor_repo: &str, code_repo_url: &str) -> bool {
    let a = repo_path_from_url(anchor_repo);
    let b = repo_path_from_url(code_repo_url);
    if a == b {
        return true;
    }
    // On macOS, /var is a symlink to /private/var. Canonicalize file:// paths
    // so anchors written with TempDir paths match current_dir() paths.
    if anchor_repo.starts_with("file://") && code_repo_url.starts_with("file://") {
        let canon_a = Path::new(a).canonicalize();
        let canon_b = Path::new(b).canonicalize();
        if let (Ok(ca), Ok(cb)) = (canon_a, canon_b) {
            return ca == cb;
        }
    }
    false
}

pub fn detect_drift(
    code_repo_path: &Path,
    docs_dir: impl AsRef<Path>,
    code_repo_url: &str,
    doc_repo_url: &str,
    doc_repo_root: &Path,
    repo_name: &str,
    exclude_dirs: &[String],
) -> Result<DriftReport> {
    let docs_dir = docs_dir.as_ref();
    let current_sha = head_sha(code_repo_path)?;
    let canon_repo = code_repo_path
        .canonicalize()
        .unwrap_or_else(|_| code_repo_path.to_path_buf());
    let docs = scan_docs(docs_dir, doc_repo_url, None, exclude_dirs);

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

            safety::validate_provenance(&anchor.provenance)
                .with_context(|| format!("anchor {} in {}", anchor.path, doc.path))?;

            let anchor_file = code_repo_path.join(&anchor.path);
            safety::validate_path_within_canon(&canon_repo, &anchor_file)
                .with_context(|| format!("anchor path in {}", doc.path))?;

            let (is_drifted, head_sig) = if anchor.provenance.starts_with(SIG_PREFIX) {
                // No git history needed — compare fingerprint directly
                let current_content = std::fs::read_to_string(&anchor_file).or_else(|_| {
                    // Fall back to reading from HEAD commit
                    read_file_at_rev(code_repo_path, &current_sha, &anchor.path)
                })?;

                let current_sig = compute_sig(&current_content, &anchor.path, symbol);
                let drifted = current_sig != anchor.provenance;
                (drifted, current_sig)
            } else {
                let content_at_provenance =
                    read_file_at_rev(code_repo_path, &anchor.provenance, &anchor.path)?;
                let content_at_head = read_file_at_rev(code_repo_path, &current_sha, &anchor.path)?;

                let sig_provenance = compute_sig(&content_at_provenance, &anchor.path, symbol);
                let sig_head = compute_sig(&content_at_head, &anchor.path, symbol);
                (sig_provenance != sig_head, sig_head)
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
                    current_sig: head_sig,
                    current_commit: current_sha.clone(),
                    diff_summary: diff_sum,
                    diff,
                });
            }
        }

        let relative_doc = make_repo_relative(&doc.path, doc_repo_root);

        if drifted_anchors.is_empty() {
            clean.push(CleanDoc {
                doc: relative_doc,
                anchor_count: relevant_anchors.len(),
            });
        } else {
            drifted.push(DriftedDoc {
                doc: relative_doc,
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
