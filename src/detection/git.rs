use anyhow::{Context, Result};
use git2::Repository;
use std::path::Path;
use std::process::Command;

/// Reads the content of a file at a specific git revision.
pub fn read_file_at_rev(repo_path: &Path, rev: &str, file_path: &str) -> Result<String> {
    let repo = Repository::open(repo_path)
        .with_context(|| format!("Failed to open repository at {:?}", repo_path))?;

    let obj = repo
        .revparse_single(rev)
        .with_context(|| format!("Failed to parse revision '{}'", rev))?;

    let commit = obj
        .peel_to_commit()
        .with_context(|| format!("Failed to peel '{}' to commit", rev))?;

    let tree = commit
        .tree()
        .with_context(|| format!("Failed to get tree for commit '{}'", rev))?;

    let entry = tree
        .get_path(Path::new(file_path))
        .with_context(|| format!("File '{}' not found in tree at '{}'", file_path, rev))?;

    let blob = repo
        .find_blob(entry.id())
        .with_context(|| format!("Failed to find blob for '{}'", file_path))?;

    let content = std::str::from_utf8(blob.content())
        .with_context(|| format!("File '{}' is not valid UTF-8", file_path))?
        .to_string();

    Ok(content)
}

/// Returns the unified diff and a stat summary for a file between a revision
/// and HEAD in a single git subprocess.
pub fn diff_with_summary(
    repo_path: &Path,
    from_rev: &str,
    file_path: &str,
) -> Result<(String, String)> {
    let output = Command::new("git")
        .args(["diff", "--stat", "-p", from_rev, "HEAD", "--", file_path])
        .current_dir(repo_path)
        .output()
        .with_context(|| format!("failed to run git diff for '{}' from '{}'", file_path, from_rev))?;

    let full = String::from_utf8(output.stdout)
        .context("git diff output is not valid UTF-8")?;

    // --stat output comes before the patch; split at the first "diff --git" line
    let (summary, diff) = match full.find("\ndiff --git ") {
        Some(pos) => (full[..pos].to_string(), full[pos + 1..].to_string()),
        None => (full.clone(), full),
    };

    Ok((diff, summary))
}

/// Returns the SHA of HEAD.
pub fn head_sha(repo_path: &Path) -> Result<String> {
    let repo = Repository::open(repo_path)
        .with_context(|| format!("failed to open repository at {}", repo_path.display()))?;

    let head = repo.head().context("failed to get HEAD reference")?;

    let oid = head
        .peel_to_commit()
        .context("failed to peel HEAD to commit")?
        .id();

    Ok(oid.to_string())
}
