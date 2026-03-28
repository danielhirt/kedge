use assert_cmd::Command;
use predicates::prelude::*;
use std::process;
use tempfile::TempDir;

#[test]
fn steer_check_detects_drift_end_to_end() {
    // 1. Create a "code repo" with a Java file
    let code_dir = TempDir::new().unwrap();
    let code_path = code_dir.path();

    process::Command::new("git")
        .args(["init"])
        .current_dir(code_path)
        .output()
        .unwrap();
    process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(code_path)
        .output()
        .unwrap();
    process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(code_path)
        .output()
        .unwrap();

    std::fs::create_dir_all(code_path.join("src/auth")).unwrap();
    std::fs::write(
        code_path.join("src/auth/Auth.java"),
        "public class Auth { public boolean check(String t) { return true; } }",
    )
    .unwrap();
    process::Command::new("git")
        .args(["add", "."])
        .current_dir(code_path)
        .output()
        .unwrap();
    process::Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(code_path)
        .output()
        .unwrap();

    let sha_output = process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(code_path)
        .output()
        .unwrap();
    let sha = String::from_utf8(sha_output.stdout)
        .unwrap()
        .trim()
        .to_string();

    // 2. Create a "docs" directory with steering file
    let docs_dir = TempDir::new().unwrap();
    std::fs::create_dir_all(docs_dir.path().join("test-group")).unwrap();
    let steering_content = format!(
        "---\nsteer:\n  group: test-group\n  anchors:\n    - repo: \"file://{code}\"\n      path: src/auth/Auth.java\n      provenance: {sha}\n---\n\n# Auth Architecture\nToken validation returns boolean.\n",
        code = code_path.display(),
        sha = sha,
    );
    std::fs::write(
        docs_dir.path().join("test-group/auth.md"),
        &steering_content,
    )
    .unwrap();

    // 3. Create steer.toml in code repo
    let config = format!(
        "[detection]\nlanguages = [\"java\"]\nfallback = \"content-hash\"\n\n[triage]\nprovider = \"anthropic\"\nmodel = \"claude-haiku-4-5-20251001\"\nseverity_levels = [\"no_update\", \"minor\", \"major\"]\n\n[remediation]\nagent_command = \"echo done\"\nauto_merge_severities = []\n\n[[repos.docs]]\nurl = \"file://{code}\"\npath = \"\"\nref = \"main\"\n",
        code = code_path.display()
    );
    std::fs::write(code_path.join("steer.toml"), &config).unwrap();

    // 4. Check with no changes — should exit 0
    Command::cargo_bin("steer")
        .unwrap()
        .args(["--config", "steer.toml", "check"])
        .current_dir(code_path)
        .env("STEER_DOCS_PATH", docs_dir.path().join("test-group"))
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\"drifted\": []")
                .or(predicate::str::contains("\"drifted\":[]")),
        );

    // 5. Now change the code
    std::fs::write(
        code_path.join("src/auth/Auth.java"),
        "public class Auth { public boolean check(String t, List<String> scopes) { return true; } }",
    )
    .unwrap();
    process::Command::new("git")
        .args(["add", "."])
        .current_dir(code_path)
        .output()
        .unwrap();
    process::Command::new("git")
        .args(["commit", "-m", "add scopes"])
        .current_dir(code_path)
        .output()
        .unwrap();

    // 6. Check again — should exit 1 (drift found) and report the drifted file path
    Command::cargo_bin("steer")
        .unwrap()
        .args(["--config", "steer.toml", "check"])
        .current_dir(code_path)
        .env("STEER_DOCS_PATH", docs_dir.path().join("test-group"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("src/auth/Auth.java"));
}
