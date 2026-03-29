use kedge::frontmatter::parse_doc_file;
use std::path::Path;

#[test]
fn parses_kedge_frontmatter_with_anchors() {
    let doc = parse_doc_file(
        Path::new("tests/fixtures/steering_with_anchors.md"),
        "git@gitlab.example.com:team/docs.git",
    )
    .unwrap();
    let fm = doc.frontmatter;
    assert_eq!(fm.group.as_deref(), Some("payments-platform"));
    assert_eq!(fm.anchors.len(), 2);
    assert_eq!(fm.anchors[0].path, "src/auth/AuthService.java");
    assert_eq!(
        fm.anchors[0].symbol.as_deref(),
        Some("AuthService#validateToken")
    );
    assert_eq!(fm.anchors[0].provenance, "a1b2c3d4");
    assert_eq!(
        fm.anchors[0].repo,
        "git@gitlab.example.com:team/backend.git"
    );
}

#[test]
fn returns_none_for_no_kedge_block() {
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

#[test]
fn update_provenance_updates_single_anchor() {
    use kedge::frontmatter::update_provenance;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(
        &path,
        "\
---
kedge:
  group: payments-platform
  anchors:
    - repo: git@gitlab.example.com:team/backend.git
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: a1b2c3d4
---

# Auth docs
",
    )
    .unwrap();

    update_provenance(
        &path,
        "src/auth/AuthService.java",
        Some("AuthService#validateToken"),
        "sig:deadbeef12345678",
    )
    .unwrap();

    let updated = std::fs::read_to_string(&path).unwrap();
    assert!(
        updated.contains("sig:deadbeef12345678"),
        "provenance should be updated: {}",
        updated
    );
    assert!(
        !updated.contains("a1b2c3d4"),
        "old provenance should be gone: {}",
        updated
    );
}

#[test]
fn update_provenance_batch_updates_multiple_anchors() {
    use kedge::frontmatter::update_provenance_batch;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(
        &path,
        "\
---
kedge:
  group: payments-platform
  anchors:
    - repo: git@gitlab.example.com:team/backend.git
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: a1b2c3d4
    - repo: git@gitlab.example.com:team/backend.git
      path: src/auth/AuthService.java
      symbol: AuthService#refreshSession
      provenance: e5f6a7b8
---

# Auth docs
",
    )
    .unwrap();

    update_provenance_batch(
        &path,
        &[
            (
                "src/auth/AuthService.java",
                Some("AuthService#validateToken"),
                "sig:aaaa111122223333",
            ),
            (
                "src/auth/AuthService.java",
                Some("AuthService#refreshSession"),
                "sig:bbbb444455556666",
            ),
        ],
    )
    .unwrap();

    let updated = std::fs::read_to_string(&path).unwrap();
    assert!(
        updated.contains("sig:aaaa111122223333"),
        "first anchor provenance should be updated: {}",
        updated
    );
    assert!(
        updated.contains("sig:bbbb444455556666"),
        "second anchor provenance should be updated: {}",
        updated
    );
    assert!(
        !updated.contains("a1b2c3d4"),
        "old first provenance should be gone: {}",
        updated
    );
    assert!(
        !updated.contains("e5f6a7b8"),
        "old second provenance should be gone: {}",
        updated
    );
}

#[test]
fn update_provenance_batch_non_matching_anchor_leaves_file_unchanged() {
    use kedge::frontmatter::update_provenance_batch;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    let original = "\
---
kedge:
  group: payments-platform
  anchors:
    - repo: git@gitlab.example.com:team/backend.git
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: a1b2c3d4
---

# Auth docs
";
    std::fs::write(&path, original).unwrap();

    // Update a non-existent anchor path
    update_provenance_batch(
        &path,
        &[(
            "src/nonexistent/File.java",
            Some("Foo#bar"),
            "sig:0000000000000000",
        )],
    )
    .unwrap();

    let updated = std::fs::read_to_string(&path).unwrap();
    assert!(
        updated.contains("a1b2c3d4"),
        "original provenance should be preserved: {}",
        updated
    );
    assert!(
        !updated.contains("sig:0000000000000000"),
        "non-matching update should not appear: {}",
        updated
    );
}

#[test]
fn scan_docs_excludes_directories() {
    let dir = tempfile::tempdir().unwrap();

    // Create a regular doc
    let doc_dir = dir.path().join("docs");
    std::fs::create_dir_all(&doc_dir).unwrap();
    std::fs::write(
        doc_dir.join("good.md"),
        "---\nkedge:\n  anchors:\n    - repo: git@example.com:r.git\n      path: src/main.rs\n      provenance: sig:abcdef1234567890\n---\n# Good",
    )
    .unwrap();

    // Create a doc inside node_modules (should be excluded)
    let nm_dir = dir.path().join("node_modules").join("pkg");
    std::fs::create_dir_all(&nm_dir).unwrap();
    std::fs::write(
        nm_dir.join("bad.md"),
        "---\nkedge:\n  anchors:\n    - repo: git@example.com:r.git\n      path: src/lib.rs\n      provenance: sig:1234567890abcdef\n---\n# Bad",
    )
    .unwrap();

    // Create a doc inside .git (should be excluded)
    let git_dir = dir.path().join(".git").join("hooks");
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::write(
        git_dir.join("readme.md"),
        "---\nkedge:\n  anchors:\n    - repo: git@example.com:r.git\n      path: src/foo.rs\n      provenance: sig:deadbeefcafebabe\n---\n# Git",
    )
    .unwrap();

    let exclude = vec!["node_modules".into(), ".git".into()];
    let docs = kedge::frontmatter::scan_docs(dir.path(), "git@example.com:r.git", None, &exclude);

    assert_eq!(docs.len(), 1);
    assert!(docs[0].path.contains("good.md"));
}

#[test]
fn scan_docs_with_empty_exclusions_finds_all() {
    let dir = tempfile::tempdir().unwrap();

    let nm_dir = dir.path().join("node_modules");
    std::fs::create_dir_all(&nm_dir).unwrap();
    std::fs::write(
        nm_dir.join("doc.md"),
        "---\nkedge:\n  anchors:\n    - repo: git@example.com:r.git\n      path: src/main.rs\n      provenance: sig:abcdef1234567890\n---\n# Doc",
    )
    .unwrap();

    let docs = kedge::frontmatter::scan_docs(dir.path(), "git@example.com:r.git", None, &[]);
    assert_eq!(docs.len(), 1);
}
