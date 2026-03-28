pub mod repo_cache;

use anyhow::{Context, Result};
use std::path::Path;

/// Copy steering files from source_dir to target_dir.
///
/// - Files from `source_dir/<group>/` are copied flat into target_dir
/// - Files from `source_dir/shared/` are always copied
/// - `source_dir/_steer/<agents_file>` is copied as agents_file name
/// - `source_dir/_steer/skill.md` is copied to skill_dir if provided
pub fn install_to_workspace(
    source_dir: &Path,
    target_dir: &Path,
    group: Option<&str>,
    agents_file: Option<&str>,
    skill_dir: Option<&Path>,
) -> Result<()> {
    std::fs::create_dir_all(target_dir)
        .with_context(|| format!("failed to create target dir {}", target_dir.display()))?;

    // Copy group-specific files
    if let Some(group_name) = group {
        let group_dir = source_dir.join(group_name);
        if group_dir.exists() {
            copy_md_files_flat(&group_dir, target_dir)?;
        }
    }

    // Copy shared files
    let shared_dir = source_dir.join("shared");
    if shared_dir.exists() {
        copy_md_files_flat(&shared_dir, target_dir)?;
    }

    // Copy _steer/AGENTS.md as the platform's agents_file name
    let meta_dir = source_dir.join("_steer");
    if let Some(af) = agents_file {
        let src = meta_dir.join("AGENTS.md");
        if src.exists() {
            let dst = target_dir.join(af);
            std::fs::copy(&src, &dst)
                .with_context(|| format!("failed to copy {} to {}", src.display(), dst.display()))?;
        }
    }

    // Copy _steer/skill.md to skill_dir
    if let Some(sd) = skill_dir {
        let src = meta_dir.join("skill.md");
        if src.exists() {
            std::fs::create_dir_all(sd)
                .with_context(|| format!("failed to create skill dir {}", sd.display()))?;
            let dst = sd.join("skill.md");
            std::fs::copy(&src, &dst)
                .with_context(|| format!("failed to copy skill.md to {}", dst.display()))?;
        }
    }

    Ok(())
}

/// Create symlinks to steering files from source_dir in target_dir.
/// Same selection logic as install_to_workspace, but uses symlinks.
pub fn install_as_links(
    source_dir: &Path,
    target_dir: &Path,
    group: Option<&str>,
    agents_file: Option<&str>,
    skill_dir: Option<&Path>,
) -> Result<()> {
    std::fs::create_dir_all(target_dir)
        .with_context(|| format!("failed to create target dir {}", target_dir.display()))?;

    // Link group-specific files
    if let Some(group_name) = group {
        let group_dir = source_dir.join(group_name);
        if group_dir.exists() {
            link_md_files_flat(&group_dir, target_dir)?;
        }
    }

    // Link shared files
    let shared_dir = source_dir.join("shared");
    if shared_dir.exists() {
        link_md_files_flat(&shared_dir, target_dir)?;
    }

    // Link _steer/AGENTS.md as the platform's agents_file name
    let meta_dir = source_dir.join("_steer");
    if let Some(af) = agents_file {
        let src = meta_dir.join("AGENTS.md");
        if src.exists() {
            let dst = target_dir.join(af);
            create_symlink(&src, &dst)?;
        }
    }

    // Link _steer/skill.md to skill_dir
    if let Some(sd) = skill_dir {
        let src = meta_dir.join("skill.md");
        if src.exists() {
            std::fs::create_dir_all(sd)
                .with_context(|| format!("failed to create skill dir {}", sd.display()))?;
            let dst = sd.join("skill.md");
            create_symlink(&src, &dst)?;
        }
    }

    Ok(())
}

/// Appends `dir_to_exclude` to `.git/info/exclude` if not already present.
pub fn add_to_git_exclude(workspace_root: &Path, dir_to_exclude: &str) -> Result<()> {
    let exclude_path = workspace_root.join(".git").join("info").join("exclude");

    // Read existing content (if any)
    let existing = if exclude_path.exists() {
        std::fs::read_to_string(&exclude_path)
            .with_context(|| format!("failed to read {}", exclude_path.display()))?
    } else {
        String::new()
    };

    // Only append if not already present
    if existing.lines().any(|line| line.trim() == dir_to_exclude) {
        return Ok(());
    }

    // Ensure directory exists
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

// --- Helpers ---

fn copy_md_files_flat(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    for entry in std::fs::read_dir(src_dir)
        .with_context(|| format!("failed to read dir {}", src_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "md" {
                    let file_name = path.file_name().unwrap();
                    let dst = dst_dir.join(file_name);
                    std::fs::copy(&path, &dst).with_context(|| {
                        format!("failed to copy {} to {}", path.display(), dst.display())
                    })?;
                }
            }
        }
    }
    Ok(())
}

fn link_md_files_flat(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    for entry in std::fs::read_dir(src_dir)
        .with_context(|| format!("failed to read dir {}", src_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "md" {
                    let file_name = path.file_name().unwrap();
                    let dst = dst_dir.join(file_name);
                    create_symlink(&path, &dst)?;
                }
            }
        }
    }
    Ok(())
}

fn create_symlink(src: &Path, dst: &Path) -> Result<()> {
    // Remove existing symlink or file before creating new one
    if dst.exists() || dst.symlink_metadata().is_ok() {
        std::fs::remove_file(dst)
            .with_context(|| format!("failed to remove existing {}", dst.display()))?;
    }

    // Use absolute path for symlink target
    let abs_src = src.canonicalize()
        .with_context(|| format!("failed to canonicalize {}", src.display()))?;

    std::os::unix::fs::symlink(&abs_src, dst)
        .with_context(|| format!("failed to create symlink {} -> {}", dst.display(), abs_src.display()))?;

    Ok(())
}
