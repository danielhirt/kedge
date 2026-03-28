use std::process::Command;
use tempfile::TempDir;

/// Helper: create a git repo with an initial commit containing a file.
fn setup_git_repo(file_name: &str, content: &str) -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let repo_path = dir.path();

    Command::new("git").args(["init"]).current_dir(repo_path).output().unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo_path).output().unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo_path).output().unwrap();

    // Handle nested directories in file_name
    if let Some(parent) = std::path::Path::new(file_name).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(repo_path.join(parent)).unwrap();
        }
    }

    std::fs::write(repo_path.join(file_name), content).unwrap();
    Command::new("git").args(["add", file_name]).current_dir(repo_path).output().unwrap();
    Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo_path).output().unwrap();

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path).output().unwrap();
    let sha = String::from_utf8(output.stdout).unwrap().trim().to_string();

    (dir, sha)
}

#[test]
fn reads_file_at_provenance_commit() {
    let (dir, sha) = setup_git_repo("hello.txt", "original content");
    // Modify the file after commit
    std::fs::write(dir.path().join("hello.txt"), "modified content").unwrap();
    let content = steer::detection::git::read_file_at_rev(dir.path(), &sha, "hello.txt").unwrap();
    assert_eq!(content, "original content");
}

#[test]
fn generates_diff_between_commits() {
    let (dir, sha1) = setup_git_repo("code.java", "class Foo {}");
    // Make a second commit with changes
    std::fs::write(dir.path().join("code.java"), "class Foo { int x; }").unwrap();
    Command::new("git").args(["add", "code.java"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["commit", "-m", "add field"]).current_dir(dir.path()).output().unwrap();
    let diff = steer::detection::git::diff_since(dir.path(), &sha1, "code.java").unwrap();
    assert!(diff.contains("class Foo"));
    assert!(diff.contains("int x"));
}

#[test]
fn gets_head_sha() {
    let (dir, sha) = setup_git_repo("f.txt", "content");
    let head = steer::detection::git::head_sha(dir.path()).unwrap();
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
        "---\nsteer:\n  group: test\n  anchors:\n    - repo: \"file://{repo}\"\n      path: src/auth/AuthService.java\n      symbol: AuthService#validate\n      provenance: {sha}\n---\n\n# Auth docs\n",
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
    Command::new("git").args(["add", "."]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["commit", "-m", "add scopes param"]).current_dir(dir.path()).output().unwrap();

    // Run detection
    let report = steer::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        "test-repo",
    ).unwrap();

    assert_eq!(report.drifted.len(), 1);
    assert_eq!(report.drifted[0].anchors.len(), 1);
    assert_eq!(report.drifted[0].anchors[0].path, "src/auth/AuthService.java");
}

#[test]
fn detection_pipeline_reports_clean_when_no_changes() {
    let (dir, sha) = setup_git_repo(
        "src/auth/AuthService.java",
        "public class AuthService { public boolean validate(String t) { return true; } }",
    );

    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        "---\nsteer:\n  group: test\n  anchors:\n    - repo: \"file://{repo}\"\n      path: src/auth/AuthService.java\n      provenance: {sha}\n---\n\n# Auth docs\n",
        repo = dir.path().display(),
        sha = sha,
    );
    std::fs::create_dir_all(docs_dir.path().join("test")).unwrap();
    std::fs::write(docs_dir.path().join("test/auth.md"), &steering).unwrap();

    let report = steer::detection::detect_drift(
        dir.path(),
        docs_dir.path().join("test"),
        &format!("file://{}", dir.path().display()),
        "test-repo",
    ).unwrap();

    assert!(report.drifted.is_empty());
    assert_eq!(report.clean.len(), 1);
}
