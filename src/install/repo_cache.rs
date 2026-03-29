use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::safety;

fn cache_dir_for(repo_url: &str) -> Result<PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(repo_url.as_bytes());
    let hash = hasher.finalize();
    let hash_hex = format!("{:x}", hash);
    let short_hash = &hash_hex[..12];

    let base = dirs::cache_dir().context("could not determine cache directory")?;

    Ok(base.join("kedge").join("repos").join(short_hash))
}

/// Run a git command with a timeout. Returns stdout on success.
fn run_git(args: &[&str], timeout_secs: u64, context: &str) -> Result<Vec<u8>> {
    let child = Command::new("git")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn git for {}", context))?;

    let _child_pid = child.id();
    let timeout = Duration::from_secs(timeout_secs);
    let ctx = context.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => {
            let output = result.with_context(|| format!("git failed for {}", ctx))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("git {} failed: {}", ctx, stderr.trim());
            }
            Ok(output.stdout)
        }
        Err(_) => {
            #[cfg(unix)]
            {
                let _ = std::process::Command::new("kill")
                    .args(["-9", &_child_pid.to_string()])
                    .status();
            }
            bail!("git {} timed out after {}s", ctx, timeout_secs);
        }
    }
}

pub fn get_or_clone(
    repo_url: &str,
    git_ref: &str,
    timeout_secs: u64,
    remote_name: &str,
) -> Result<PathBuf> {
    safety::validate_repo_url(repo_url)?;
    safety::validate_git_ref(git_ref)?;

    let cache_dir = cache_dir_for(repo_url)?;
    let cache_str = cache_dir.to_string_lossy();

    if cache_dir.exists() {
        run_git(
            &["-C", &cache_str, "fetch", remote_name, git_ref],
            timeout_secs,
            &format!("fetch {}", git_ref),
        )?;

        let checkout_ref = format!("{}/{}", remote_name, git_ref);
        run_git(
            &["-C", &cache_str, "reset", "--hard", &checkout_ref],
            timeout_secs,
            &format!("reset to {}", checkout_ref),
        )?;
    } else {
        if let Some(parent) = cache_dir.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create cache parent dir {}", parent.display())
            })?;
        }

        run_git(
            &[
                "clone", "--depth", "1", "--branch", git_ref, "--", repo_url, &cache_str,
            ],
            timeout_secs,
            &format!("clone {}", safety::sanitize_url(repo_url)),
        )?;

        if remote_name != "origin" {
            run_git(
                &["-C", &cache_str, "remote", "rename", "origin", remote_name],
                timeout_secs,
                &format!("rename remote to {}", remote_name),
            )?;
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&cache_dir, std::fs::Permissions::from_mode(0o700));
    }

    Ok(cache_dir)
}

pub fn is_up_to_date(
    repo_url: &str,
    git_ref: &str,
    timeout_secs: u64,
    _remote_name: &str,
) -> Result<bool> {
    safety::validate_repo_url(repo_url)?;
    safety::validate_git_ref(git_ref)?;

    let cache_dir = cache_dir_for(repo_url)?;

    if !cache_dir.exists() {
        return Ok(false);
    }

    let local_stdout = match run_git(
        &["-C", &cache_dir.to_string_lossy(), "rev-parse", "HEAD"],
        timeout_secs,
        "rev-parse HEAD",
    ) {
        Ok(out) => out,
        Err(_) => return Ok(false),
    };
    let local_sha = String::from_utf8_lossy(&local_stdout).trim().to_string();

    let remote_stdout = match run_git(
        &["ls-remote", "--", repo_url, git_ref],
        timeout_secs,
        &format!("ls-remote {}", git_ref),
    ) {
        Ok(out) => out,
        Err(_) => return Ok(false),
    };
    let remote_out = String::from_utf8_lossy(&remote_stdout);
    let remote_sha = remote_out
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or("")
        .to_string();

    Ok(local_sha == remote_sha && !local_sha.is_empty())
}
