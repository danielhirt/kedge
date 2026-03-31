use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

pub fn read_file_at_rev(repo_path: &Path, rev: &str, file_path: &str) -> Result<String> {
    let rev_path = format!("{}:{}", rev, file_path);
    let output = Command::new("git")
        .args(["show", &rev_path])
        .current_dir(repo_path)
        .output()
        .with_context(|| format!("failed to run git show for '{}' at '{}'", file_path, rev))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "file '{}' not found at revision '{}': {}",
            file_path,
            rev,
            stderr.trim()
        );
    }

    String::from_utf8(output.stdout)
        .with_context(|| format!("file '{}' is not valid UTF-8", file_path))
}

/// Returns (diff, stat_summary) in a single git subprocess.
pub fn diff_with_summary(
    repo_path: &Path,
    from_rev: &str,
    file_path: &str,
) -> Result<(String, String)> {
    let output = Command::new("git")
        .args(["diff", "--stat", "-p", from_rev, "HEAD", "--", file_path])
        .current_dir(repo_path)
        .output()
        .with_context(|| {
            format!(
                "failed to run git diff for '{}' from '{}'",
                file_path, from_rev
            )
        })?;

    let full = String::from_utf8(output.stdout).context("git diff output is not valid UTF-8")?;

    // --stat output comes before the patch; split at the first "diff --git" line
    let (summary, diff) = match full.find("\ndiff --git ") {
        Some(pos) => (full[..pos].to_string(), full[pos + 1..].to_string()),
        None => (full.clone(), full),
    };

    Ok((diff, summary))
}

/// Returns the URL of the `origin` remote, or `None` if unavailable.
pub fn remote_url(repo_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if url.is_empty() {
        None
    } else {
        Some(url)
    }
}

pub fn head_sha(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .context("failed to run git rev-parse HEAD")?;

    if !output.status.success() {
        bail!("git rev-parse HEAD failed in {}", repo_path.display());
    }

    let sha = String::from_utf8(output.stdout)
        .context("git rev-parse output is not valid UTF-8")?
        .trim()
        .to_string();

    Ok(sha)
}
