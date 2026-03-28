use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;
use std::process;
use tempfile::TempDir;

fn init_git_repo(path: &Path) {
    for (args, _) in [
        (vec!["init"], ""),
        (vec!["config", "user.email", "test@test.com"], ""),
        (vec!["config", "user.name", "Test"], ""),
    ] {
        process::Command::new("git")
            .args(&args)
            .current_dir(path)
            .output()
            .unwrap();
    }
}

fn git_commit(path: &Path, msg: &str) {
    process::Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    process::Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(path)
        .output()
        .unwrap();
}

fn head_sha(path: &Path) -> String {
    let out = process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(path)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Sets up a code repo with a Java file and a docs dir with a steering file
/// pointing at it. Returns (code_dir, docs_dir).
fn setup_code_and_docs() -> (TempDir, TempDir) {
    let code_dir = TempDir::new().unwrap();
    let code_path = code_dir.path();
    init_git_repo(code_path);

    std::fs::create_dir_all(code_path.join("src/auth")).unwrap();
    std::fs::write(
        code_path.join("src/auth/Auth.java"),
        "public class Auth { public boolean check(String t) { return true; } }",
    )
    .unwrap();
    git_commit(code_path, "init");

    let sha = head_sha(code_path);

    let docs_dir = TempDir::new().unwrap();
    let steering_content = format!(
        "---\nkedge:\n  anchors:\n    - repo: \"file://{code}\"\n      path: src/auth/Auth.java\n      provenance: {sha}\n---\n\n# Auth\n",
        code = code_path.display(),
    );
    std::fs::write(docs_dir.path().join("auth.md"), &steering_content).unwrap();

    (code_dir, docs_dir)
}

fn write_config(code_path: &Path) {
    let config = format!(
        "[detection]\nlanguages = [\"java\"]\n\n[triage]\nprovider = \"command\"\ntriage_command = \"echo '[]'\"\n\n[remediation]\nagent_command = \"echo done\"\nauto_merge_severities = []\n\n[[repos.docs]]\nurl = \"file://{code}\"\npath = \"\"\nref = \"main\"\n",
        code = code_path.display(),
    );
    std::fs::write(code_path.join("kedge.toml"), &config).unwrap();
}

// ---------------------------------------------------------------------------
// Bug regression: kedge check with KEDGE_DOCS_PATH and no kedge.toml
// ---------------------------------------------------------------------------

#[test]
fn check_works_with_env_only_no_config() {
    let (code_dir, docs_dir) = setup_code_and_docs();

    // No kedge.toml exists — KEDGE_DOCS_PATH should be sufficient
    Command::cargo_bin("kedge")
        .unwrap()
        .args(["check"])
        .current_dir(code_dir.path())
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .assert()
        .success();
}

#[test]
fn check_with_config_and_env_prefers_env() {
    let (code_dir, docs_dir) = setup_code_and_docs();
    write_config(code_dir.path());

    // Both config and env exist — should work and use the env path
    Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "check"])
        .current_dir(code_dir.path())
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .assert()
        .success();
}

#[test]
fn check_without_config_or_env_fails_with_helpful_error() {
    let code_dir = TempDir::new().unwrap();
    init_git_repo(code_dir.path());
    std::fs::write(code_dir.path().join("f.txt"), "x").unwrap();
    git_commit(code_dir.path(), "init");

    Command::cargo_bin("kedge")
        .unwrap()
        .args(["check"])
        .current_dir(code_dir.path())
        .env_remove("KEDGE_DOCS_PATH")
        .assert()
        .failure()
        .stderr(predicate::str::contains("kedge.toml"));
}

// ---------------------------------------------------------------------------
// Bug regression: kedge triage without config
// ---------------------------------------------------------------------------

#[test]
fn triage_without_config_fails_with_helpful_error() {
    let dir = TempDir::new().unwrap();
    let report = r#"{"repo":"t","ref":"HEAD","commit":"abc","drifted":[{"doc":"a.md","doc_repo":"r","anchors":[{"path":"f.java","provenance":"abc1234","current_commit":"def","diff_summary":"s","diff":"d"}]}],"clean":[]}"#;
    let report_path = dir.path().join("drift.json");
    std::fs::write(&report_path, report).unwrap();

    Command::cargo_bin("kedge")
        .unwrap()
        .args(["triage", "--report"])
        .arg(&report_path)
        .current_dir(dir.path())
        .env_remove("KEDGE_DOCS_PATH")
        .assert()
        .failure()
        .stderr(predicate::str::contains("triage requires"));
}

// ---------------------------------------------------------------------------
// kedge check: end-to-end drift detection
// ---------------------------------------------------------------------------

#[test]
fn kedge_check_detects_drift_end_to_end() {
    let (code_dir, docs_dir) = setup_code_and_docs();
    let code_path = code_dir.path();
    write_config(code_path);

    // Clean — exit 0
    Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "check"])
        .current_dir(code_path)
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\"drifted\": []")
                .or(predicate::str::contains("\"drifted\":[]")),
        );

    // Modify code and commit
    std::fs::write(
        code_path.join("src/auth/Auth.java"),
        "public class Auth { public boolean check(String t, List<String> scopes) { return true; } }",
    )
    .unwrap();
    git_commit(code_path, "add scopes");

    // Drifted — exit 1
    Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "check"])
        .current_dir(code_path)
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("src/auth/Auth.java"));
}

#[test]
fn check_writes_report_to_file() {
    let (code_dir, docs_dir) = setup_code_and_docs();
    write_config(code_dir.path());

    let report_path = code_dir.path().join("report.json");
    Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "check", "--report"])
        .arg(&report_path)
        .current_dir(code_dir.path())
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .assert()
        .success();

    let content = std::fs::read_to_string(&report_path).unwrap();
    assert!(content.contains("\"drifted\""));
    assert!(content.contains("\"clean\""));
}

// ---------------------------------------------------------------------------
// kedge init
// ---------------------------------------------------------------------------

#[test]
fn init_creates_config() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "init"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));

    assert!(dir.path().join("kedge.toml").exists());
    let content = std::fs::read_to_string(dir.path().join("kedge.toml")).unwrap();
    assert!(content.contains("[detection]"));
    assert!(content.contains("[triage]"));
    assert!(content.contains("[remediation]"));
}

#[test]
fn init_skips_existing_config() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("kedge.toml"), "existing").unwrap();

    Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "init"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("already exists"));

    // File should not be overwritten
    let content = std::fs::read_to_string(dir.path().join("kedge.toml")).unwrap();
    assert_eq!(content, "existing");
}

// ---------------------------------------------------------------------------
// kedge link / sync
// ---------------------------------------------------------------------------

#[test]
fn link_stamps_sig_provenance() {
    let code_dir = TempDir::new().unwrap();
    let code_path = code_dir.path();
    init_git_repo(code_path);

    std::fs::create_dir_all(code_path.join("src")).unwrap();
    std::fs::write(
        code_path.join("src/App.java"),
        "public class App { public void run() {} }",
    )
    .unwrap();
    git_commit(code_path, "init");

    let doc_content = format!(
        "---\nkedge:\n  anchors:\n    - repo: \"file://{code}\"\n      path: src/App.java\n      provenance: placeholder\n---\n\n# App\n",
        code = code_path.display(),
    );
    let doc_path = code_path.join("app.md");
    std::fs::write(&doc_path, &doc_content).unwrap();

    Command::cargo_bin("kedge")
        .unwrap()
        .args(["link"])
        .arg(&doc_path)
        .current_dir(code_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("1/1 anchors stamped"));

    let updated = std::fs::read_to_string(&doc_path).unwrap();
    assert!(
        updated.contains("sig:"),
        "provenance should be sig:-prefixed, got: {}",
        updated
    );
    assert!(!updated.contains("placeholder"));
}

#[test]
fn sync_advances_provenance() {
    let code_dir = TempDir::new().unwrap();
    let code_path = code_dir.path();
    init_git_repo(code_path);

    std::fs::create_dir_all(code_path.join("src")).unwrap();
    std::fs::write(
        code_path.join("src/Svc.java"),
        "public class Svc { public void go() {} }",
    )
    .unwrap();
    git_commit(code_path, "init");

    let doc_content = format!(
        "---\nkedge:\n  anchors:\n    - repo: \"file://{code}\"\n      path: src/Svc.java\n      provenance: old\n---\n\n# Svc\n",
        code = code_path.display(),
    );
    let doc_path = code_path.join("svc.md");
    std::fs::write(&doc_path, &doc_content).unwrap();

    Command::cargo_bin("kedge")
        .unwrap()
        .args(["sync"])
        .arg(&doc_path)
        .current_dir(code_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Synced 1 anchors"));

    let updated = std::fs::read_to_string(&doc_path).unwrap();
    assert!(updated.contains("sig:"));
    assert!(!updated.contains("old"));
}

// ---------------------------------------------------------------------------
// kedge status
// ---------------------------------------------------------------------------

#[test]
fn status_lists_anchors() {
    let dir = TempDir::new().unwrap();
    let doc = "---\nkedge:\n  group: mygroup\n  anchors:\n    - repo: \"file:///repo\"\n      path: src/X.java\n      symbol: X#run\n      provenance: sig:abc123\n---\n\n# X\n";
    std::fs::write(dir.path().join("x.md"), doc).unwrap();

    Command::cargo_bin("kedge")
        .unwrap()
        .args(["status"])
        .current_dir(dir.path())
        .env("KEDGE_DOCS_PATH", dir.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("mygroup")
                .and(predicate::str::contains("src/X.java"))
                .and(predicate::str::contains("#X#run"))
                .and(predicate::str::contains("sig:abc123")),
        );
}

#[test]
fn status_with_no_docs_shows_message() {
    let dir = TempDir::new().unwrap();

    Command::cargo_bin("kedge")
        .unwrap()
        .args(["status"])
        .current_dir(dir.path())
        .env("KEDGE_DOCS_PATH", dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No docs with kedge frontmatter"));
}
