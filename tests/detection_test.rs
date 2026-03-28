use std::process::Command;
use tempfile::TempDir;

/// Helper: create a git repo with an initial commit containing a file.
fn setup_git_repo(file_name: &str, content: &str) -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let repo_path = dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Handle nested directories in file_name
    if let Some(parent) = std::path::Path::new(file_name).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(repo_path.join(parent)).unwrap();
        }
    }

    std::fs::write(repo_path.join(file_name), content).unwrap();
    Command::new("git")
        .args(["add", file_name])
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let sha = String::from_utf8(output.stdout).unwrap().trim().to_string();

    (dir, sha)
}

#[test]
fn reads_file_at_provenance_commit() {
    let (dir, sha) = setup_git_repo("hello.txt", "original content");
    // Modify the file after commit
    std::fs::write(dir.path().join("hello.txt"), "modified content").unwrap();
    let content = kedge::detection::git::read_file_at_rev(dir.path(), &sha, "hello.txt").unwrap();
    assert_eq!(content, "original content");
}

#[test]
fn generates_diff_between_commits() {
    let (dir, sha1) = setup_git_repo("code.java", "class Foo {}");
    // Make a second commit with changes
    std::fs::write(dir.path().join("code.java"), "class Foo { int x; }").unwrap();
    Command::new("git")
        .args(["add", "code.java"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add field"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let (diff, _summary) =
        kedge::detection::git::diff_with_summary(dir.path(), &sha1, "code.java").unwrap();
    assert!(diff.contains("class Foo"));
    assert!(diff.contains("int x"));
}

#[test]
fn gets_head_sha() {
    let (dir, sha) = setup_git_repo("f.txt", "content");
    let head = kedge::detection::git::head_sha(dir.path()).unwrap();
    assert_eq!(head, sha);
}

#[test]
fn detection_pipeline_finds_drift_when_code_changed() {
    // Setup: git repo with a Java file
    let (dir, sha) = setup_git_repo(
        "src/auth/AuthService.java",
        "public class AuthService { public boolean validate(String t) { return true; } }",
    );

    // Create a steering doc that anchors to this file
    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        "---\nkedge:\n  group: test\n  anchors:\n    - repo: \"file://{repo}\"\n      path: src/auth/AuthService.java\n      symbol: AuthService#validate\n      provenance: {sha}\n---\n\n# Auth docs\n",
        repo = dir.path().display(),
        sha = sha,
    );
    std::fs::create_dir_all(docs_dir.path().join("test")).unwrap();
    std::fs::write(docs_dir.path().join("test/auth.md"), &steering).unwrap();

    // Now modify the Java file (change the method signature)
    std::fs::write(
        dir.path().join("src/auth/AuthService.java"),
        "public class AuthService { public boolean validate(String t, List<String> scopes) { return true; } }",
    ).unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add scopes param"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Run detection
    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        "test-repo",
    )
    .unwrap();

    assert_eq!(report.drifted.len(), 1);
    assert_eq!(report.drifted[0].anchors.len(), 1);
    assert_eq!(
        report.drifted[0].anchors[0].path,
        "src/auth/AuthService.java"
    );
}

#[test]
fn detection_pipeline_reports_clean_when_no_changes() {
    let (dir, sha) = setup_git_repo(
        "src/auth/AuthService.java",
        "public class AuthService { public boolean validate(String t) { return true; } }",
    );

    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        "---\nkedge:\n  group: test\n  anchors:\n    - repo: \"file://{repo}\"\n      path: src/auth/AuthService.java\n      provenance: {sha}\n---\n\n# Auth docs\n",
        repo = dir.path().display(),
        sha = sha,
    );
    std::fs::create_dir_all(docs_dir.path().join("test")).unwrap();
    std::fs::write(docs_dir.path().join("test/auth.md"), &steering).unwrap();

    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        "test-repo",
    )
    .unwrap();

    assert!(report.drifted.is_empty());
    assert_eq!(report.clean.len(), 1);
}

#[test]
fn sig_provenance_detects_drift_without_git_history() {
    let original_content =
        "public class AuthService { public boolean validate(String t) { return true; } }";

    // Compute the sig for the original content
    let sig = kedge::detection::fingerprint::compute_sig(
        original_content,
        "src/auth/AuthService.java",
        Some("AuthService#validate"),
    );
    assert!(sig.starts_with("sig:"));

    // Create a git repo with MODIFIED content (simulating post-rebase state)
    let (dir, _sha) = setup_git_repo(
        "src/auth/AuthService.java",
        "public class AuthService { public boolean validate(String t, List<String> scopes) { return true; } }",
    );

    // Steering doc uses sig: provenance from original content
    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        "---\nkedge:\n  group: test\n  anchors:\n    - repo: \"file://{repo}\"\n      path: src/auth/AuthService.java\n      symbol: AuthService#validate\n      provenance: \"{sig}\"\n---\n\n# Auth docs\n",
        repo = dir.path().display(),
        sig = sig,
    );
    std::fs::create_dir_all(docs_dir.path().join("test")).unwrap();
    std::fs::write(docs_dir.path().join("test/auth.md"), &steering).unwrap();

    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        "test-repo",
    )
    .unwrap();

    // Should detect drift — content fingerprint changed
    assert_eq!(report.drifted.len(), 1);
}

#[test]
fn sig_provenance_reports_clean_when_content_matches() {
    let content = "public class AuthService { public boolean validate(String t) { return true; } }";

    let sig =
        kedge::detection::fingerprint::compute_sig(content, "src/auth/AuthService.java", None);

    let (dir, _sha) = setup_git_repo("src/auth/AuthService.java", content);

    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        "---\nkedge:\n  group: test\n  anchors:\n    - repo: \"file://{repo}\"\n      path: src/auth/AuthService.java\n      provenance: \"{sig}\"\n---\n\n# Auth docs\n",
        repo = dir.path().display(),
        sig = sig,
    );
    std::fs::create_dir_all(docs_dir.path().join("test")).unwrap();
    std::fs::write(docs_dir.path().join("test/auth.md"), &steering).unwrap();

    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        "test-repo",
    )
    .unwrap();

    assert!(report.drifted.is_empty());
    assert_eq!(report.clean.len(), 1);
}

#[test]
fn sig_provenance_survives_rebase() {
    let content = "public class AuthService { public boolean validate(String t) { return true; } }";

    // Compute sig from content
    let sig =
        kedge::detection::fingerprint::compute_sig(content, "src/auth/AuthService.java", None);

    // Create repo, commit, then rebase (amend) — SHA changes but content stays same
    let (dir, _original_sha) = setup_git_repo("src/auth/AuthService.java", content);

    // Simulate rebase: amend the commit (SHA changes, content identical)
    Command::new("git")
        .args(["commit", "--amend", "-m", "rebased commit"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let new_sha_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let new_sha = String::from_utf8(new_sha_output.stdout)
        .unwrap()
        .trim()
        .to_string();
    assert_ne!(_original_sha, new_sha, "SHA should change after amend");

    // Steering doc uses sig: provenance — immune to SHA change
    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        "---\nkedge:\n  group: test\n  anchors:\n    - repo: \"file://{repo}\"\n      path: src/auth/AuthService.java\n      provenance: \"{sig}\"\n---\n\n# Auth docs\n",
        repo = dir.path().display(),
        sig = sig,
    );
    std::fs::create_dir_all(docs_dir.path().join("test")).unwrap();
    std::fs::write(docs_dir.path().join("test/auth.md"), &steering).unwrap();

    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        "test-repo",
    )
    .unwrap();

    // Should be clean — content hasn't changed, even though SHA did
    assert!(
        report.drifted.is_empty(),
        "sig: provenance should survive rebase"
    );
}

#[test]
fn detect_drift_rejects_anchor_with_path_traversal() {
    let (dir, _sha) = setup_git_repo("src/auth/AuthService.java", "public class AuthService {}");

    // Create a steering doc with an anchor whose path escapes the repo
    let docs_dir = TempDir::new().unwrap();
    let sig = kedge::detection::fingerprint::compute_sig("irrelevant", "../../../etc/passwd", None);
    let steering = format!(
        "---\nkedge:\n  group: test\n  anchors:\n    - repo: \"file://{repo}\"\n      path: \"../../../etc/passwd\"\n      provenance: \"{sig}\"\n---\n\n# Malicious doc\n",
        repo = dir.path().display(),
        sig = sig,
    );
    std::fs::create_dir_all(docs_dir.path().join("test")).unwrap();
    std::fs::write(docs_dir.path().join("test/evil.md"), &steering).unwrap();

    let result = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        "test-repo",
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    let chain = format!("{:#}", err);
    assert!(
        chain.contains("escapes") || chain.contains("outside"),
        "error chain should mention path escape: {}",
        chain
    );
}

#[test]
fn detect_drift_rejects_invalid_provenance_format() {
    let (dir, _sha) = setup_git_repo("src/main.rs", "fn main() {}");

    // Create a steering doc with a provenance that looks like a git flag
    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        "---\nkedge:\n  group: test\n  anchors:\n    - repo: \"file://{repo}\"\n      path: src/main.rs\n      provenance: \"--output=evil\"\n---\n\n# Malicious doc\n",
        repo = dir.path().display(),
    );
    std::fs::create_dir_all(docs_dir.path().join("test")).unwrap();
    std::fs::write(docs_dir.path().join("test/evil.md"), &steering).unwrap();

    let result = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        "test-repo",
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    let chain = format!("{:#}", err);
    assert!(
        chain.contains("provenance") || chain.contains("invalid"),
        "error chain should mention invalid provenance: {}",
        chain
    );
}
