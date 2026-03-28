use anyhow::{bail, Result};
use std::path::Path;

/// Rejects values that could be git flags or arbitrary revision expressions.
pub fn validate_provenance(prov: &str) -> Result<()> {
    if let Some(hex) = prov.strip_prefix("sig:") {
        if !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(());
        }
        bail!("invalid sig provenance: {}", prov);
    }

    // Legacy SHA provenance: must be hex-only, at least 7 chars (short SHA)
    if prov.len() >= 7 && prov.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(());
    }

    bail!("invalid provenance (expected hex SHA or sig:...): {}", prov);
}

pub fn validate_git_ref(git_ref: &str) -> Result<()> {
    if git_ref.starts_with('-') {
        bail!("invalid git ref (starts with '-'): {}", git_ref);
    }
    if git_ref.contains("..") {
        bail!("invalid git ref (contains '..'): {}", git_ref);
    }
    if git_ref.is_empty() {
        bail!("git ref must not be empty");
    }
    Ok(())
}

pub fn validate_repo_url(url: &str) -> Result<()> {
    if url.starts_with('-') {
        bail!("invalid repo URL (starts with '-'): {}", url);
    }
    if url.is_empty() {
        bail!("repo URL must not be empty");
    }
    Ok(())
}

pub fn validate_path_within(base: &Path, joined: &Path) -> Result<()> {
    let canon_base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    validate_path_within_canon(&canon_base, joined)
}

/// Like [`validate_path_within`] but skips re-canonicalizing the base.
pub fn validate_path_within_canon(canon_base: &Path, joined: &Path) -> Result<()> {
    let canon_joined = joined
        .canonicalize()
        .unwrap_or_else(|_| normalize_path(joined));

    if !canon_joined.starts_with(canon_base) {
        bail!(
            "path escapes base directory: {} is outside {}",
            joined.display(),
            canon_base.display()
        );
    }
    Ok(())
}

pub fn validate_bare_name(name: &str, field: &str) -> Result<()> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        bail!(
            "{} must be a plain name without path separators, got: {}",
            field,
            name
        );
    }
    Ok(())
}

/// `https://user:token@host/path` → `https://***@host/path`
pub fn sanitize_url(url: &str) -> std::borrow::Cow<'_, str> {
    if let Some(scheme_end) = url.find("://") {
        let after_scheme = &url[scheme_end + 3..];
        if let Some(at_pos) = after_scheme.find('@') {
            return format!(
                "{}://***@{}",
                &url[..scheme_end],
                &after_scheme[at_pos + 1..]
            )
            .into();
        }
    }
    url.into()
}

/// Resolve `.` and `..` without touching the filesystem.
fn normalize_path(path: &Path) -> std::path::PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}
