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

/// Generates a unified diff for a file between a revision and current HEAD.
pub fn diff_since(repo_path: &Path, from_rev: &str, file_path: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", from_rev, "HEAD", "--", file_path])
        .current_dir(repo_path)
        .output()
        .with_context(|| format!("Failed to run git diff from '{}' for '{}'", from_rev, file_path))?;

    let diff = String::from_utf8(output.stdout)
        .context("git diff output is not valid UTF-8")?;

    Ok(diff)
}

/// Returns the SHA of HEAD.
pub fn head_sha(repo_path: &Path) -> Result<String> {
    let repo = Repository::open(repo_path)
        .with_context(|| format!("Failed to open repository at {:?}", repo_path))?;

    let head = repo.head().context("Failed to get HEAD reference")?;

    let oid = head
        .peel_to_commit()
        .context("Failed to peel HEAD to commit")?
        .id();

    Ok(oid.to_string())
}

/// Returns a short diff summary (--stat) for a file between a revision and current HEAD.
pub fn diff_summary(repo_path: &Path, from_rev: &str, file_path: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "--stat", from_rev, "HEAD", "--", file_path])
        .current_dir(repo_path)
        .output()
        .with_context(|| format!("Failed to run git diff --stat from '{}' for '{}'", from_rev, file_path))?;

    let summary = String::from_utf8(output.stdout)
        .context("git diff --stat output is not valid UTF-8")?;

    Ok(summary)
}
