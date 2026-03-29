pub mod repo_cache;

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::safety;

enum InstallMode {
    Copy,
    #[cfg(unix)]
    Link,
}

/// Collect `.md` files from a directory, optionally recursing into subdirectories.
/// Returns `(absolute_path, relative_path_from_base)` pairs.
fn walk_md_files(dir: &Path, recursive: bool) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut result = Vec::new();
    walk_md_files_inner(dir, dir, recursive, &mut result)?;
    Ok(result)
}

fn walk_md_files_inner(
    base: &Path,
    current: &Path,
    recursive: bool,
    result: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<()> {
    for entry in std::fs::read_dir(current)
        .with_context(|| format!("failed to read dir {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_symlink() {
            eprintln!("warning: skipping symlink {}", path.display());
            continue;
        }
        if path.is_dir() && recursive {
            walk_md_files_inner(base, &path, recursive, result)?;
        } else if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            let rel = path.strip_prefix(base).unwrap_or(&path).to_path_buf();
            result.push((path, rel));
        }
    }
    Ok(())
}

fn install_steering(
    mode: &InstallMode,
    source_dir: &Path,
    target_dir: &Path,
    group: Option<&str>,
    agents_file: Option<&str>,
    skill_dir: Option<&Path>,
    recursive: bool,
) -> Result<()> {
    if let Some(group_name) = group {
        safety::validate_bare_name(group_name, "group")?;
    }
    if let Some(af) = agents_file {
        safety::validate_bare_name(af, "agents_file")?;
    }

    std::fs::create_dir_all(target_dir)
        .with_context(|| format!("failed to create target dir {}", target_dir.display()))?;

    let place_file = |src: &Path, dst: &Path| -> Result<()> {
        match mode {
            InstallMode::Copy => {
                std::fs::copy(src, dst).with_context(|| {
                    format!("failed to copy {} to {}", src.display(), dst.display())
                })?;
            }
            #[cfg(unix)]
            InstallMode::Link => {
                create_symlink(src, dst)?;
            }
        }
        Ok(())
    };

    let place_md_files = |src_dir: &Path| -> Result<()> {
        for (src_path, rel_path) in walk_md_files(src_dir, recursive)? {
            let dst = target_dir.join(&rel_path);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            place_file(&src_path, &dst)?;
        }
        Ok(())
    };

    if let Some(group_name) = group {
        let group_dir = source_dir.join(group_name);
        if group_dir.exists() {
            place_md_files(&group_dir)?;
        }
    }

    let shared_dir = source_dir.join("shared");
    if shared_dir.exists() {
        place_md_files(&shared_dir)?;
    }

    // _kedge/AGENTS.md as the platform's agents_file name
    let meta_dir = source_dir.join("_kedge");
    if let Some(af) = agents_file {
        let src = meta_dir.join("AGENTS.md");
        if src.exists() && !src.is_symlink() {
            place_file(&src, &target_dir.join(af))?;
        }
    }

    if let Some(sd) = skill_dir {
        let src = meta_dir.join("skill.md");
        if src.exists() && !src.is_symlink() {
            std::fs::create_dir_all(sd)
                .with_context(|| format!("failed to create skill dir {}", sd.display()))?;
            place_file(&src, &sd.join("skill.md"))?;
        }
    }

    Ok(())
}

pub fn install_to_workspace(
    source_dir: &Path,
    target_dir: &Path,
    group: Option<&str>,
    agents_file: Option<&str>,
    skill_dir: Option<&Path>,
    recursive: bool,
) -> Result<()> {
    install_steering(
        &InstallMode::Copy,
        source_dir,
        target_dir,
        group,
        agents_file,
        skill_dir,
        recursive,
    )
}

#[cfg(unix)]
pub fn install_as_links(
    source_dir: &Path,
    target_dir: &Path,
    group: Option<&str>,
    agents_file: Option<&str>,
    skill_dir: Option<&Path>,
    recursive: bool,
) -> Result<()> {
    install_steering(
        &InstallMode::Link,
        source_dir,
        target_dir,
        group,
        agents_file,
        skill_dir,
        recursive,
    )
}

pub fn add_to_git_exclude(workspace_root: &Path, dir_to_exclude: &str) -> Result<()> {
    let exclude_path = workspace_root.join(".git").join("info").join("exclude");

    let existing = if exclude_path.exists() {
        std::fs::read_to_string(&exclude_path)
            .with_context(|| format!("failed to read {}", exclude_path.display()))?
    } else {
        String::new()
    };

    if existing.lines().any(|line| line.trim() == dir_to_exclude) {
        return Ok(());
    }

    if let Some(parent) = exclude_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let entry = if existing.ends_with('\n') || existing.is_empty() {
        format!("{}\n", dir_to_exclude)
    } else {
        format!("\n{}\n", dir_to_exclude)
    };

    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&exclude_path)
        .with_context(|| format!("failed to open {}", exclude_path.display()))?;

    file.write_all(entry.as_bytes())
        .with_context(|| format!("failed to write to {}", exclude_path.display()))?;

    Ok(())
}

#[cfg(unix)]
fn create_symlink(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() || dst.symlink_metadata().is_ok() {
        std::fs::remove_file(dst)
            .with_context(|| format!("failed to remove existing {}", dst.display()))?;
    }

    let abs_src = src
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", src.display()))?;

    std::os::unix::fs::symlink(&abs_src, dst).with_context(|| {
        format!(
            "failed to create symlink {} -> {}",
            dst.display(),
            abs_src.display()
        )
    })?;

    Ok(())
}
