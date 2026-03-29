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
        "[detection]\n\n[triage]\nprovider = \"command\"\ntriage_command = \"echo '[{{\\\"path\\\": \\\"src/auth/Auth.java\\\", \\\"severity\\\": \\\"no_update\\\"}}]'\"\n\n[remediation]\nagent_command = \"echo done\"\nauto_merge_severities = []\n\n[[repos.docs]]\nurl = \"file://{code}\"\npath = \"\"\nref = \"main\"\n",
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
    let report = r#"{"repo":"t","ref":"HEAD","commit":"abc","drifted":[{"doc":"a.md","doc_repo":"r","anchors":[{"path":"f.java","provenance":"abc1234","current_sig":"sig:0000000000000000","current_commit":"def","diff_summary":"s","diff":"d"}]}],"clean":[]}"#;
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
        "---\nkedge:\n  anchors:\n    - repo: \"file://{code}\"\n      path: src/Svc.java\n      provenance: stale0placeholder\n---\n\n# Svc\n",
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
    assert!(!updated.contains("stale0placeholder"));
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

// ---------------------------------------------------------------------------
// kedge update --no-stamp
// ---------------------------------------------------------------------------

/// Creates a code repo with drift and a docs dir with a steering file.
/// Returns (code_dir, docs_dir, original_provenance_sha).
fn setup_drifted_code_and_docs() -> (TempDir, TempDir, String) {
    let (code_dir, docs_dir) = setup_code_and_docs();
    let code_path = code_dir.path();

    // Modify code to create drift
    std::fs::write(
        code_path.join("src/auth/Auth.java"),
        "public class Auth { public boolean check(String t, List<String> scopes) { return true; } }",
    )
    .unwrap();
    git_commit(code_path, "add scopes param");

    // Read back the original provenance from the steering file
    let doc_content = std::fs::read_to_string(docs_dir.path().join("auth.md")).unwrap();
    let prov = doc_content
        .lines()
        .find(|l| l.contains("provenance:"))
        .unwrap()
        .split("provenance:")
        .nth(1)
        .unwrap()
        .trim()
        .to_string();

    write_config(code_path);

    (code_dir, docs_dir, prov)
}

#[test]
fn update_default_stamps_provenance() {
    let (code_dir, docs_dir, original_prov) = setup_drifted_code_and_docs();

    let output = Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "update"])
        .current_dir(code_dir.path())
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "kedge update failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Provenance should have been stamped with a sig: value
    let doc_after = std::fs::read_to_string(docs_dir.path().join("auth.md")).unwrap();
    assert!(
        doc_after.contains("sig:"),
        "provenance should be stamped with sig:, got: {}",
        doc_after
    );
    assert!(
        !doc_after.contains(&original_prov),
        "original provenance should be replaced"
    );

    // Summary should report anchors_synced >= 1
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"anchors_synced\": 1"),
        "summary should report 1 anchor synced, got: {}",
        stdout
    );
    assert!(
        stdout.contains("did not affect documentation accuracy"),
        "reason should indicate no_update stamp, got: {}",
        stdout
    );
}

#[test]
fn update_no_stamp_skips_provenance_write() {
    let (code_dir, docs_dir, original_prov) = setup_drifted_code_and_docs();

    let output = Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "update", "--no-stamp"])
        .current_dir(code_dir.path())
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "kedge update --no-stamp failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Provenance should NOT have been updated — still the original SHA
    let doc_after = std::fs::read_to_string(docs_dir.path().join("auth.md")).unwrap();
    assert!(
        doc_after.contains(&original_prov),
        "provenance should be unchanged with --no-stamp, got: {}",
        doc_after
    );
    assert!(
        !doc_after.contains("sig:"),
        "no sig: should be stamped with --no-stamp, got: {}",
        doc_after
    );
}

#[test]
fn update_no_stamp_summary_reports_deferred_docs() {
    let (code_dir, docs_dir, _) = setup_drifted_code_and_docs();

    let output = Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "update", "--no-stamp"])
        .current_dir(code_dir.path())
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // provenance_advanced should still list the doc
    assert!(
        stdout.contains("\"provenance_advanced\""),
        "summary should include provenance_advanced, got: {}",
        stdout
    );
    assert!(
        stdout.contains("auth.md"),
        "deferred doc should appear in summary, got: {}",
        stdout
    );

    // anchors_synced should be 0
    assert!(
        stdout.contains("\"anchors_synced\": 0"),
        "anchors_synced should be 0 with --no-stamp, got: {}",
        stdout
    );

    // reason should mention not stamped
    assert!(
        stdout.contains("not stamped"),
        "reason should mention 'not stamped', got: {}",
        stdout
    );
}

// ---------------------------------------------------------------------------
// kedge update / triage with provider = "none" (skip triage)
// ---------------------------------------------------------------------------

#[test]
fn update_with_provider_none_skips_triage_and_invokes_agent() {
    let (code_dir, docs_dir) = setup_code_and_docs();
    let code_path = code_dir.path();

    // Modify code to create drift
    std::fs::write(
        code_path.join("src/auth/Auth.java"),
        "public class Auth { public boolean check(String t, List<String> scopes) { return true; } }",
    )
    .unwrap();
    git_commit(code_path, "add scopes param");

    // Config with provider = "none" — no triage_command needed
    let config = format!(
        "[detection]\n\n[triage]\nprovider = \"none\"\n\n[remediation]\nagent_command = \"echo done\"\nauto_merge_severities = []\n\n[[repos.docs]]\nurl = \"file://{code}\"\npath = \"\"\nref = \"main\"\n",
        code = code_path.display(),
    );
    std::fs::write(code_path.join("kedge.toml"), &config).unwrap();

    let output = Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "update"])
        .current_dir(code_path)
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "kedge update with provider=none failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Skipping triage"),
        "should print skip message, got stderr: {}",
        stderr
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Agent should have been invoked — doc appears in remediated
    assert!(
        stdout.contains("\"remediated\""),
        "summary should include remediated section, got: {}",
        stdout
    );
    assert!(
        stdout.contains("auth.md"),
        "remediated doc should appear in summary, got: {}",
        stdout
    );

    // No provenance_advanced — nothing classified as no_update
    let summary: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let advanced = summary["provenance_advanced"].as_array().unwrap();
    assert!(
        advanced.is_empty(),
        "no docs should be auto-synced when triage is skipped, got: {:?}",
        advanced
    );
}

#[test]
fn triage_with_provider_none_outputs_all_major() {
    let dir = TempDir::new().unwrap();

    let report = r#"{"repo":"test","ref":"main","commit":"abc123","drifted":[{"doc":"auth.md","doc_repo":"git@example.com:docs.git","anchors":[{"path":"src/Auth.java","symbol":"Auth#validate","provenance":"old1","current_sig":"sig:1111111111111111","current_commit":"abc123","diff_summary":"Added param","diff":"+param"},{"path":"src/Config.java","provenance":"old2","current_sig":"sig:2222222222222222","current_commit":"abc123","diff_summary":"Changed","diff":"+change"}]}],"clean":[]}"#;
    let report_path = dir.path().join("drift.json");
    std::fs::write(&report_path, report).unwrap();

    // Config with provider = "none"
    let config = "[triage]\nprovider = \"none\"\n\n[remediation]\nagent_command = \"echo done\"\n\n[[repos.docs]]\nurl = \"file:///tmp/dummy\"\npath = \"\"\nref = \"main\"\n";
    std::fs::write(dir.path().join("kedge.toml"), config).unwrap();

    let output = Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "triage", "--report"])
        .arg(&report_path)
        .current_dir(dir.path())
        .env("KEDGE_DOCS_PATH", dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "kedge triage with provider=none failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let triaged: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();

    // All anchors should be major
    let anchors = triaged["drifted"][0]["anchors"].as_array().unwrap();
    assert_eq!(anchors.len(), 2);
    assert_eq!(anchors[0]["severity"], "major");
    assert_eq!(anchors[1]["severity"], "major");

    // Doc-level severity should be major
    assert_eq!(triaged["drifted"][0]["severity"], "major");
}

#[test]
fn update_no_stamp_does_not_affect_agent_invocation() {
    let (code_dir, docs_dir) = setup_code_and_docs();
    let code_path = code_dir.path();

    // Modify code to create drift
    std::fs::write(
        code_path.join("src/auth/Auth.java"),
        "public class Auth { public boolean check(String t, List<String> scopes) { return true; } }",
    )
    .unwrap();
    git_commit(code_path, "add scopes param");

    // Triage returns minor severity so the doc goes to the agent path
    let config = format!(
        "[detection]\n\n[triage]\nprovider = \"command\"\ntriage_command = \"echo '[{{\\\"path\\\": \\\"src/auth/Auth.java\\\", \\\"severity\\\": \\\"minor\\\"}}]'\"\n\n[remediation]\nagent_command = \"echo done\"\nauto_merge_severities = []\n\n[[repos.docs]]\nurl = \"file://{code}\"\npath = \"\"\nref = \"main\"\n",
        code = code_path.display(),
    );
    std::fs::write(code_path.join("kedge.toml"), &config).unwrap();

    let output = Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "update", "--no-stamp"])
        .current_dir(code_path)
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "kedge update --no-stamp failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Agent should have been invoked for the minor-severity doc
    assert!(
        stdout.contains("\"remediated\""),
        "summary should include remediated section, got: {}",
        stdout
    );
    assert!(
        stdout.contains("auth.md"),
        "remediated doc should appear in summary, got: {}",
        stdout
    );
}

#[test]
fn agent_payload_has_correct_target_repo_and_relative_path() {
    let code_dir = TempDir::new().unwrap();
    let code_path = code_dir.path();
    init_git_repo(code_path);

    std::fs::create_dir_all(code_path.join("src/api")).unwrap();
    std::fs::write(
        code_path.join("src/api/Handler.java"),
        "public class Handler { public void handle() {} }",
    )
    .unwrap();
    git_commit(code_path, "init");
    let sha = head_sha(code_path);

    // Docs dir with a subdirectory (simulating repo_root/steering/api.md)
    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        "---\nkedge:\n  anchors:\n    - repo: \"file://{code}\"\n      path: src/api/Handler.java\n      symbol: Handler#handle\n      provenance: {sha}\n---\n\n# API handler\n",
        code = code_path.display(),
    );
    std::fs::create_dir_all(docs_dir.path().join("steering")).unwrap();
    std::fs::write(docs_dir.path().join("steering/api.md"), &steering).unwrap();

    // Modify code to cause drift
    std::fs::write(
        code_path.join("src/api/Handler.java"),
        "public class Handler { public void handle(Request req) {} }",
    )
    .unwrap();
    git_commit(code_path, "add request param");

    // Capture agent payload to a temp file (cat captures stdin, exits 0)
    let payload_file = code_path.join("agent-payload.json");
    let agent_cmd = format!("sh -c 'cat > {}'", payload_file.display());
    let docs_repo_url = "https://github.com/myorg/docs.git";
    let config = format!(
        "[detection]\n\n[triage]\nprovider = \"none\"\n\n[remediation]\nagent_command = \"{agent}\"\nauto_merge_severities = []\n\n[[repos.docs]]\nurl = \"file://{code}\"\npath = \"\"\nref = \"main\"\n",
        agent = agent_cmd,
        code = code_path.display(),
    );
    std::fs::write(code_path.join("kedge.toml"), &config).unwrap();

    let output = Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "update"])
        .current_dir(code_path)
        .env("KEDGE_DOCS_PATH", docs_dir.path().join("steering"))
        .env("KEDGE_DOCS_REPO_URL", docs_repo_url)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "kedge update failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Parse the agent payload
    let payload_str =
        std::fs::read_to_string(&payload_file).expect("agent payload file should exist");
    let payload: serde_json::Value = serde_json::from_str(&payload_str).unwrap();

    // target.repo should be KEDGE_DOCS_REPO_URL, not the code repo URL
    let target_repo = payload["target"]["repo"].as_str().unwrap();
    assert_eq!(
        target_repo, docs_repo_url,
        "target.repo should be the docs repo URL from KEDGE_DOCS_REPO_URL"
    );
    assert!(
        !target_repo.contains(&code_path.display().to_string()),
        "target.repo must not be the code repo URL"
    );

    // target.path should be relative (just "api.md" since KEDGE_DOCS_PATH is the repo root)
    let target_path = payload["target"]["path"].as_str().unwrap();
    assert!(
        !target_path.starts_with('/'),
        "target.path must be relative, got: {}",
        target_path
    );
    assert!(
        target_path.ends_with("api.md"),
        "target.path should reference the steering file, got: {}",
        target_path
    );
}

#[test]
fn update_exits_nonzero_when_agent_fails() {
    let (code_dir, docs_dir) = setup_code_and_docs();
    let code_path = code_dir.path();

    // Modify code to create drift
    std::fs::write(
        code_path.join("src/auth/Auth.java"),
        "public class Auth { public boolean check(String t, int flags) { return true; } }",
    )
    .unwrap();
    git_commit(code_path, "add flags");

    // Agent command that always fails (exit 1)
    let config = format!(
        "[detection]\n\n[triage]\nprovider = \"none\"\n\n[remediation]\nagent_command = \"false\"\nauto_merge_severities = []\n\n[[repos.docs]]\nurl = \"file://{code}\"\npath = \"\"\nref = \"main\"\n",
        code = code_path.display(),
    );
    std::fs::write(code_path.join("kedge.toml"), &config).unwrap();

    let output = Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "update"])
        .current_dir(code_path)
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "kedge update should exit non-zero when agent fails, got exit 0"
    );

    // The summary should still be valid JSON with a populated errors array
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let errors = summary["errors"].as_array().unwrap();
    assert!(
        !errors.is_empty(),
        "errors array should be non-empty when agent fails"
    );
}

#[test]
fn update_exits_zero_when_agent_succeeds() {
    let (code_dir, docs_dir) = setup_code_and_docs();
    let code_path = code_dir.path();

    // Modify code to create drift
    std::fs::write(
        code_path.join("src/auth/Auth.java"),
        "public class Auth { public boolean check(String t, int flags) { return true; } }",
    )
    .unwrap();
    git_commit(code_path, "add flags");

    // Agent command that succeeds
    let config = format!(
        "[detection]\n\n[triage]\nprovider = \"none\"\n\n[remediation]\nagent_command = \"echo done\"\nauto_merge_severities = []\n\n[[repos.docs]]\nurl = \"file://{code}\"\npath = \"\"\nref = \"main\"\n",
        code = code_path.display(),
    );
    std::fs::write(code_path.join("kedge.toml"), &config).unwrap();

    let output = Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "update"])
        .current_dir(code_path)
        .env("KEDGE_DOCS_PATH", docs_dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "kedge update should exit 0 when agent succeeds, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let errors = summary["errors"].as_array().unwrap();
    assert!(errors.is_empty(), "errors array should be empty on success");
}

#[test]
fn triage_rejects_path_traversal_in_drift_report() {
    let dir = TempDir::new().unwrap();

    // Create a secret file outside the docs root
    let secret_file = dir.path().join("secret.txt");
    std::fs::write(&secret_file, "TOP SECRET CONTENT").unwrap();

    // Create a docs subdirectory as the docs root
    let docs_root = dir.path().join("docs");
    std::fs::create_dir_all(&docs_root).unwrap();

    // Crafted drift report with path traversal in drifted[].doc
    let report = r#"{"repo":"test","ref":"main","commit":"abc123","drifted":[{"doc":"../secret.txt","doc_repo":"git@example.com:docs.git","anchors":[{"path":"src/Auth.java","symbol":"Auth#validate","provenance":"old1","current_sig":"sig:1111111111111111","current_commit":"abc123","diff_summary":"Changed","diff":"+change"}]}],"clean":[]}"#;
    let report_path = dir.path().join("drift.json");
    std::fs::write(&report_path, report).unwrap();

    // provider = "none" skips the actual triage call; we're testing that
    // collect_doc_contents rejects the traversal path before triage runs.
    let config = "[triage]\nprovider = \"none\"\n\n[remediation]\nagent_command = \"echo done\"\n\n[[repos.docs]]\nurl = \"file:///tmp/dummy\"\npath = \"\"\nref = \"main\"\n";
    std::fs::write(dir.path().join("kedge.toml"), &config).unwrap();

    let output = Command::cargo_bin("kedge")
        .unwrap()
        .args(["--config", "kedge.toml", "triage", "--report"])
        .arg(&report_path)
        .current_dir(dir.path())
        .env("KEDGE_DOCS_PATH", &docs_root)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "kedge triage should succeed even with traversal path: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Stderr should warn about the skipped path
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("outside") || stderr.contains("skipping"),
        "should warn about path traversal, got stderr: {}",
        stderr
    );

    // The triage output must not contain the secret content
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("TOP SECRET"),
        "path traversal must not leak file contents into triage output"
    );
}
