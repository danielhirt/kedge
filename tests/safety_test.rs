use kedge::safety::{
    sanitize_url, validate_bare_name, validate_git_ref, validate_path_within,
    validate_path_within_canon, validate_provenance, validate_repo_url,
};
use tempfile::TempDir;

// ── validate_provenance ─────────────────────────────────────────────────

#[test]
fn provenance_accepts_valid_40_char_hex_sha() {
    let sha = "a".repeat(40);
    assert!(validate_provenance(&sha).is_ok());
}

#[test]
fn provenance_accepts_valid_7_char_short_sha() {
    assert!(validate_provenance("abcdef0").is_ok());
}

#[test]
fn provenance_accepts_sig_with_valid_hex() {
    assert!(validate_provenance("sig:deadbeef01234567").is_ok());
}

#[test]
fn provenance_rejects_sig_with_empty_hex() {
    assert!(validate_provenance("sig:").is_err());
}

#[test]
fn provenance_rejects_sig_with_non_hex_chars() {
    assert!(validate_provenance("sig:xyz123").is_err());
}

#[test]
fn provenance_rejects_short_sha_under_7_chars() {
    assert!(validate_provenance("abc").is_err());
}

#[test]
fn provenance_rejects_string_starting_with_dash() {
    assert!(validate_provenance("--output=file").is_err());
}

#[test]
fn provenance_rejects_revision_expression_head_tilde() {
    assert!(validate_provenance("HEAD~1").is_err());
}

#[test]
fn provenance_rejects_revision_expression_range() {
    assert!(validate_provenance("main..dev").is_err());
}

#[test]
fn provenance_rejects_empty_string() {
    assert!(validate_provenance("").is_err());
}

// ── validate_git_ref ────────────────────────────────────────────────────

#[test]
fn git_ref_accepts_main() {
    assert!(validate_git_ref("main").is_ok());
}

#[test]
fn git_ref_accepts_version_tag() {
    assert!(validate_git_ref("v1.0").is_ok());
}

#[test]
fn git_ref_accepts_feature_branch() {
    assert!(validate_git_ref("feature/foo").is_ok());
}

#[test]
fn git_ref_rejects_starting_with_dash() {
    assert!(validate_git_ref("--symbolic").is_err());
}

#[test]
fn git_ref_rejects_double_dot() {
    assert!(validate_git_ref("main..dev").is_err());
}

#[test]
fn git_ref_rejects_empty_string() {
    assert!(validate_git_ref("").is_err());
}

// ── validate_repo_url ───────────────────────────────────────────────────

#[test]
fn repo_url_accepts_https() {
    assert!(validate_repo_url("https://github.com/org/repo.git").is_ok());
}

#[test]
fn repo_url_accepts_ssh() {
    assert!(validate_repo_url("git@github.com:org/repo.git").is_ok());
}

#[test]
fn repo_url_accepts_file_scheme() {
    assert!(validate_repo_url("file:///home/user/repo").is_ok());
}

#[test]
fn repo_url_rejects_starting_with_dash() {
    assert!(validate_repo_url("--upload-pack=evil").is_err());
}

#[test]
fn repo_url_rejects_empty_string() {
    assert!(validate_repo_url("").is_err());
}

// ── validate_path_within ────────────────────────────────────────────────

#[test]
fn path_within_allows_path_inside_base() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();
    let child = base.join("subdir").join("file.txt");

    // Create the subdir so canonicalize works on the base.
    std::fs::create_dir_all(base.join("subdir")).unwrap();
    std::fs::write(&child, "").unwrap();

    assert!(validate_path_within(base, &child).is_ok());
}

#[test]
fn path_within_rejects_traversal_escaping_base() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("inner");
    std::fs::create_dir_all(&base).unwrap();

    // Construct a path that escapes via `..`
    let escaped = base.join("..").join("..").join("etc").join("passwd");
    assert!(validate_path_within(&base, &escaped).is_err());
}

// ── validate_path_within_canon ──────────────────────────────────────────

#[test]
fn path_within_canon_allows_path_inside_base() {
    let tmp = TempDir::new().unwrap();
    let canon_base = tmp.path().canonicalize().unwrap();
    let child = canon_base.join("file.txt");
    std::fs::write(&child, "").unwrap();

    assert!(validate_path_within_canon(&canon_base, &child).is_ok());
}

#[test]
fn path_within_canon_rejects_traversal_escaping_base() {
    let tmp = TempDir::new().unwrap();
    let inner = tmp.path().join("inner");
    std::fs::create_dir_all(&inner).unwrap();
    let canon_base = inner.canonicalize().unwrap();

    let escaped = canon_base.join("..").join("..").join("etc").join("passwd");
    assert!(validate_path_within_canon(&canon_base, &escaped).is_err());
}

// ── validate_bare_name ──────────────────────────────────────────────────

#[test]
fn bare_name_accepts_plain_name() {
    assert!(validate_bare_name("payments", "group").is_ok());
}

#[test]
fn bare_name_accepts_name_with_extension() {
    assert!(validate_bare_name("AGENTS.md", "agents_file").is_ok());
}

#[test]
fn bare_name_rejects_forward_slash() {
    assert!(validate_bare_name("path/to/file", "group").is_err());
}

#[test]
fn bare_name_rejects_backslash() {
    assert!(validate_bare_name("path\\file", "group").is_err());
}

#[test]
fn bare_name_rejects_double_dot() {
    assert!(validate_bare_name("..", "group").is_err());
}

// ── sanitize_url ────────────────────────────────────────────────────────

#[test]
fn sanitize_url_strips_userinfo_from_https() {
    let result = sanitize_url("https://user:pass@host/path");
    assert_eq!(result, "https://***@host/path");
}

#[test]
fn sanitize_url_preserves_url_without_credentials() {
    let url = "https://github.com/org/repo.git";
    assert_eq!(sanitize_url(url), url);
}

#[test]
fn sanitize_url_leaves_ssh_style_url_unchanged() {
    let url = "git@github.com:org/repo.git";
    assert_eq!(sanitize_url(url), url);
}

#[test]
fn sanitize_url_leaves_file_scheme_unchanged() {
    let url = "file:///home/user/repo";
    assert_eq!(sanitize_url(url), url);
}
