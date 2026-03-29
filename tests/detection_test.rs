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
    let doc_repo_url = format!("file://{}", docs_dir.path().display());
    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        &doc_repo_url,
        docs_dir.path(),
        "test-repo",
        &[],
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

    let doc_repo_url = format!("file://{}", docs_dir.path().display());
    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        &doc_repo_url,
        docs_dir.path(),
        "test-repo",
        &[],
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

    let doc_repo_url = format!("file://{}", docs_dir.path().display());
    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        &doc_repo_url,
        docs_dir.path(),
        "test-repo",
        &[],
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

    let doc_repo_url = format!("file://{}", docs_dir.path().display());
    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        &doc_repo_url,
        docs_dir.path(),
        "test-repo",
        &[],
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

    let doc_repo_url = format!("file://{}", docs_dir.path().display());
    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        &doc_repo_url,
        docs_dir.path(),
        "test-repo",
        &[],
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

    let doc_repo_url = format!("file://{}", docs_dir.path().display());
    let result = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        &doc_repo_url,
        docs_dir.path(),
        "test-repo",
        &[],
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

    let doc_repo_url = format!("file://{}", docs_dir.path().display());
    let result = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        &doc_repo_url,
        docs_dir.path(),
        "test-repo",
        &[],
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

#[test]
fn detect_drift_across_multiple_doc_dirs() {
    // Create a code repo with two Java files in the same initial commit
    let dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::fs::create_dir_all(dir.path().join("src/auth")).unwrap();
    std::fs::write(
        dir.path().join("src/auth/AuthService.java"),
        "public class AuthService { public boolean validate(String t) { return true; } }",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src/api")).unwrap();
    std::fs::write(
        dir.path().join("src/api/ApiController.java"),
        "public class ApiController { public void handle() {} }",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial with both files"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let sha = String::from_utf8(output.stdout).unwrap().trim().to_string();

    let code_repo_url = format!("file://{}", dir.path().display());

    // Doc dir 1: anchors to AuthService
    let docs_dir1 = TempDir::new().unwrap();
    let steering1 = format!(
        "---\nkedge:\n  group: auth\n  anchors:\n    - repo: \"{repo}\"\n      path: src/auth/AuthService.java\n      provenance: {sha}\n---\n\n# Auth docs\n",
        repo = code_repo_url,
        sha = sha,
    );
    std::fs::create_dir_all(docs_dir1.path().join("auth")).unwrap();
    std::fs::write(docs_dir1.path().join("auth/auth.md"), &steering1).unwrap();

    // Doc dir 2: anchors to ApiController
    let docs_dir2 = TempDir::new().unwrap();
    let steering2 = format!(
        "---\nkedge:\n  group: api\n  anchors:\n    - repo: \"{repo}\"\n      path: src/api/ApiController.java\n      provenance: {sha}\n---\n\n# API docs\n",
        repo = code_repo_url,
        sha = sha,
    );
    std::fs::create_dir_all(docs_dir2.path().join("api")).unwrap();
    std::fs::write(docs_dir2.path().join("api/api.md"), &steering2).unwrap();

    // Now modify both files to trigger drift
    std::fs::write(
        dir.path().join("src/auth/AuthService.java"),
        "public class AuthService { public boolean validate(String t, List<String> scopes) { return true; } }",
    ).unwrap();
    std::fs::write(
        dir.path().join("src/api/ApiController.java"),
        "public class ApiController { public void handle(Request req) {} }",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "modify both files"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Run detect_drift on each doc dir and merge (mimicking multi-repo check)
    let doc_repo_url1 = format!("file://{}", docs_dir1.path().display());
    let report1 = kedge::detection::detect_drift(
        dir.path(),
        docs_dir1.path().join("auth"),
        &code_repo_url,
        &doc_repo_url1,
        docs_dir1.path(),
        "test-repo",
        &[],
    )
    .unwrap();

    let doc_repo_url2 = format!("file://{}", docs_dir2.path().display());
    let report2 = kedge::detection::detect_drift(
        dir.path(),
        docs_dir2.path().join("api"),
        &code_repo_url,
        &doc_repo_url2,
        docs_dir2.path(),
        "test-repo",
        &[],
    )
    .unwrap();

    // Each should have one drifted doc
    assert_eq!(
        report1.drifted.len(),
        1,
        "report1 should have 1 drifted doc"
    );
    assert_eq!(
        report2.drifted.len(),
        1,
        "report2 should have 1 drifted doc"
    );

    // Merge reports
    let mut merged_drifted = report1.drifted;
    merged_drifted.extend(report2.drifted);
    let mut merged_clean = report1.clean;
    merged_clean.extend(report2.clean);

    // Merged report should have drifted docs from both dirs
    assert_eq!(merged_drifted.len(), 2, "merged should have 2 drifted docs");
    assert!(merged_clean.is_empty(), "no clean docs expected");

    // Verify the drifted docs reference different files
    let drifted_paths: Vec<&str> = merged_drifted
        .iter()
        .flat_map(|d| d.anchors.iter().map(|a| a.path.as_str()))
        .collect();
    assert!(
        drifted_paths.contains(&"src/auth/AuthService.java"),
        "should contain auth anchor"
    );
    assert!(
        drifted_paths.contains(&"src/api/ApiController.java"),
        "should contain api anchor"
    );
}

#[test]
fn detect_drift_multiple_dirs_mixed_clean_and_drifted() {
    // Code repo with one file that will drift and one that won't
    let (dir, _sha) = setup_git_repo(
        "src/Stable.java",
        "public class Stable { public void run() {} }",
    );
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/Changing.java"),
        "public class Changing { public void exec() {} }",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add changing"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Get the SHA after both files are committed
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let both_sha = String::from_utf8(output.stdout).unwrap().trim().to_string();

    let code_repo_url = format!("file://{}", dir.path().display());

    // Doc dir 1: anchors to Stable.java (will remain clean)
    let docs_dir1 = TempDir::new().unwrap();
    let steering1 = format!(
        "---\nkedge:\n  anchors:\n    - repo: \"{repo}\"\n      path: src/Stable.java\n      provenance: {sha}\n---\n\n# Stable docs\n",
        repo = code_repo_url,
        sha = both_sha,
    );
    std::fs::write(docs_dir1.path().join("stable.md"), &steering1).unwrap();

    // Doc dir 2: anchors to Changing.java (will drift)
    let docs_dir2 = TempDir::new().unwrap();
    let steering2 = format!(
        "---\nkedge:\n  anchors:\n    - repo: \"{repo}\"\n      path: src/Changing.java\n      provenance: {sha}\n---\n\n# Changing docs\n",
        repo = code_repo_url,
        sha = both_sha,
    );
    std::fs::write(docs_dir2.path().join("changing.md"), &steering2).unwrap();

    // Only modify Changing.java
    std::fs::write(
        dir.path().join("src/Changing.java"),
        "public class Changing { public void exec(String arg) {} }",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "modify changing"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Run detection on each
    let doc_repo_url1 = format!("file://{}", docs_dir1.path().display());
    let report1 = kedge::detection::detect_drift(
        dir.path(),
        docs_dir1.path(),
        &code_repo_url,
        &doc_repo_url1,
        docs_dir1.path(),
        "test-repo",
        &[],
    )
    .unwrap();

    let doc_repo_url2 = format!("file://{}", docs_dir2.path().display());
    let report2 = kedge::detection::detect_drift(
        dir.path(),
        docs_dir2.path(),
        &code_repo_url,
        &doc_repo_url2,
        docs_dir2.path(),
        "test-repo",
        &[],
    )
    .unwrap();

    // Merge
    let total_drifted = report1.drifted.len() + report2.drifted.len();
    let total_clean = report1.clean.len() + report2.clean.len();

    assert_eq!(total_clean, 1, "one doc should be clean (Stable)");
    assert_eq!(total_drifted, 1, "one doc should be drifted (Changing)");
}

#[test]
fn detect_drift_sets_doc_repo_to_docs_url_not_code_url() {
    let (dir, sha) = setup_git_repo(
        "src/auth/AuthService.java",
        "public class AuthService { public boolean validate(String t) { return true; } }",
    );
    let code_repo_url = format!("file://{}", dir.path().display());
    let docs_repo_url = "https://github.com/myorg/docs.git";

    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        "---\nkedge:\n  anchors:\n    - repo: \"{code}\"\n      path: src/auth/AuthService.java\n      symbol: AuthService#validate\n      provenance: {sha}\n---\n\n# Auth docs\n",
        code = code_repo_url,
        sha = sha,
    );
    std::fs::write(docs_dir.path().join("auth.md"), &steering).unwrap();

    // Modify code to cause drift
    std::fs::write(
        dir.path().join("src/auth/AuthService.java"),
        "public class AuthService { public boolean validate(String t, int flags) { return true; } }",
    ).unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add flags param"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path(),
        &code_repo_url,
        docs_repo_url,
        docs_dir.path(),
        "test-repo",
        &[],
    )
    .unwrap();

    assert_eq!(report.drifted.len(), 1);
    // doc_repo must be the docs repo URL, not the code repo URL
    assert_eq!(
        report.drifted[0].doc_repo, docs_repo_url,
        "doc_repo should be the docs repo URL, not the code repo URL"
    );
    assert_ne!(
        report.drifted[0].doc_repo, code_repo_url,
        "doc_repo must not be the code repo URL"
    );
}

#[test]
fn detect_drift_stores_repo_relative_doc_paths() {
    let (dir, sha) = setup_git_repo("src/main.rs", "fn main() { println!(\"hello\"); }");
    let code_repo_url = format!("file://{}", dir.path().display());

    // Create docs in a subdirectory to simulate repo_root/docs/steering/file.md
    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        "---\nkedge:\n  anchors:\n    - repo: \"{code}\"\n      path: src/main.rs\n      provenance: {sha}\n---\n\n# Main docs\n",
        code = code_repo_url,
        sha = sha,
    );
    std::fs::create_dir_all(docs_dir.path().join("steering")).unwrap();
    std::fs::write(docs_dir.path().join("steering/main.md"), &steering).unwrap();

    // Modify code
    std::fs::write(
        dir.path().join("src/main.rs"),
        "fn main() { println!(\"changed\"); }",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "change main"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // repo_root is the TempDir root, scan_dir is steering/ subdirectory
    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("steering"),
        &code_repo_url,
        "https://example.com/docs.git",
        docs_dir.path(), // repo root is parent of scan dir
        "test-repo",
        &[],
    )
    .unwrap();

    assert_eq!(report.drifted.len(), 1);
    // Path must be relative to repo root (steering/main.md), not absolute
    assert_eq!(
        report.drifted[0].doc, "steering/main.md",
        "doc path should be relative to repo root"
    );
    assert!(
        !report.drifted[0].doc.starts_with('/'),
        "doc path must not be absolute: {}",
        report.drifted[0].doc
    );
}

#[test]
fn detect_drift_clean_docs_also_have_relative_paths() {
    let (dir, sha) = setup_git_repo("src/lib.rs", "pub fn greet() -> &'static str { \"hello\" }");
    let code_repo_url = format!("file://{}", dir.path().display());

    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        "---\nkedge:\n  anchors:\n    - repo: \"{code}\"\n      path: src/lib.rs\n      provenance: {sha}\n---\n\n# Lib docs\n",
        code = code_repo_url,
        sha = sha,
    );
    std::fs::create_dir_all(docs_dir.path().join("docs")).unwrap();
    std::fs::write(docs_dir.path().join("docs/lib.md"), &steering).unwrap();

    // No code changes — should be clean
    let report = kedge::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("docs"),
        &code_repo_url,
        "https://example.com/docs.git",
        docs_dir.path(),
        "test-repo",
        &[],
    )
    .unwrap();

    assert_eq!(report.clean.len(), 1);
    assert_eq!(
        report.clean[0].doc, "docs/lib.md",
        "clean doc path should also be relative to repo root"
    );
}
