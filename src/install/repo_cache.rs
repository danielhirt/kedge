use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::Command;

/// Returns the cache directory path for a given repo URL.
/// Uses the first 12 characters of the SHA-256 hash of the URL.
pub fn cache_dir_for(repo_url: &str) -> Result<PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(repo_url.as_bytes());
    let hash = hasher.finalize();
    let hash_hex = format!("{:x}", hash);
    let short_hash = &hash_hex[..12];

    let base = dirs::cache_dir()
        .context("could not determine cache directory")?;

    Ok(base.join("steer").join("repos").join(short_hash))
}

/// Returns path to a cached clone of the repo, cloning or updating as needed.
pub fn get_or_clone(repo_url: &str, git_ref: &str) -> Result<PathBuf> {
    let cache_dir = cache_dir_for(repo_url)?;

    if cache_dir.exists() {
        // Fetch and checkout the ref
        let fetch_status = Command::new("git")
            .args(["-C", &cache_dir.to_string_lossy(), "fetch", "origin", git_ref])
            .status()
            .context("failed to run git fetch")?;

        if !fetch_status.success() {
            anyhow::bail!("git fetch failed for ref '{}' in {}", git_ref, cache_dir.display());
        }

        let checkout_ref = format!("origin/{}", git_ref);
        let reset_status = Command::new("git")
            .args(["-C", &cache_dir.to_string_lossy(), "reset", "--hard", &checkout_ref])
            .status()
            .context("failed to run git reset")?;

        if !reset_status.success() {
            anyhow::bail!("git reset --hard '{}' failed in {}", checkout_ref, cache_dir.display());
        }
    } else {
        // Create the parent directory only — git clone creates the final dir itself
        if let Some(parent) = cache_dir.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create cache parent dir {}", parent.display()))?;
        }

        let clone_status = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--branch",
                git_ref,
                repo_url,
                &cache_dir.to_string_lossy(),
            ])
            .status()
            .context("failed to run git clone")?;

        if !clone_status.success() {
            anyhow::bail!("git clone failed for '{}'", repo_url);
        }
    }

    Ok(cache_dir)
}

/// Returns true if the local cached clone HEAD matches the remote HEAD for the given ref.
pub fn is_up_to_date(repo_url: &str, git_ref: &str) -> Result<bool> {
    let cache_dir = cache_dir_for(repo_url)?;

    if !cache_dir.exists() {
        return Ok(false);
    }

    // Get local HEAD SHA
    let local_output = Command::new("git")
        .args(["-C", &cache_dir.to_string_lossy(), "rev-parse", "HEAD"])
        .output()
        .context("failed to run git rev-parse HEAD")?;

    if !local_output.status.success() {
        return Ok(false);
    }

    let local_sha = String::from_utf8_lossy(&local_output.stdout).trim().to_string();

    // Get remote HEAD SHA for ref
    let remote_output = Command::new("git")
        .args(["ls-remote", repo_url, git_ref])
        .output()
        .context("failed to run git ls-remote")?;

    if !remote_output.status.success() {
        return Ok(false);
    }

    let remote_out = String::from_utf8_lossy(&remote_output.stdout);
    let remote_sha = remote_out
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or("")
        .to_string();

    Ok(local_sha == remote_sha && !local_sha.is_empty())
}
