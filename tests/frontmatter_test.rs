use steer::frontmatter::parse_doc_file;
use std::path::Path;

#[test]
fn parses_steer_frontmatter_with_anchors() {
    let doc = parse_doc_file(
        Path::new("tests/fixtures/steering_with_anchors.md"),
        "git@gitlab.example.com:team/docs.git",
    )
    .unwrap();
    let fm = doc.frontmatter;
    assert_eq!(fm.group.as_deref(), Some("payments-platform"));
    assert_eq!(fm.anchors.len(), 2);
    assert_eq!(fm.anchors[0].path, "src/auth/AuthService.java");
    assert_eq!(fm.anchors[0].symbol.as_deref(), Some("AuthService#validateToken"));
    assert_eq!(fm.anchors[0].provenance, "a1b2c3d4");
    assert_eq!(fm.anchors[0].repo, "git@gitlab.example.com:team/backend.git");
}

#[test]
fn returns_none_for_no_steer_block() {
    let result = parse_doc_file(
        Path::new("tests/fixtures/steering_no_steer.md"),
        "git@gitlab.example.com:team/docs.git",
    );
    assert!(result.is_none());
}

#[test]
fn preserves_doc_content_after_frontmatter() {
    let doc = parse_doc_file(
        Path::new("tests/fixtures/steering_with_anchors.md"),
        "git@gitlab.example.com:team/docs.git",
    )
    .unwrap();
    assert!(doc.content.contains("# Authentication Steering"));
    assert!(doc.content.contains("JWT validation"));
}

#[test]
fn preserves_raw_frontmatter_for_kiro_fields() {
    let doc = parse_doc_file(
        Path::new("tests/fixtures/steering_with_anchors.md"),
        "git@gitlab.example.com:team/docs.git",
    )
    .unwrap();
    assert!(doc.raw_frontmatter.contains("inclusion: fileMatch"));
    assert!(doc.raw_frontmatter.contains("fileMatchPattern"));
}

#[test]
fn returns_none_for_file_without_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no_fm.md");
    std::fs::write(&path, "# Just a heading\n\nSome content.").unwrap();
    let result = parse_doc_file(&path, "git@example.com:repo.git");
    assert!(result.is_none());
}
