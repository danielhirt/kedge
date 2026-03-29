use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn fixture_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

/// Copy the java-project fixture into a fresh git repo so we have real history.
fn setup_code_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let repo = dir.path();

    // Copy fixture files into the temp dir
    copy_dir_recursive(&fixture_path("java-project"), repo);

    // Init git repo and commit
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo)
        .output()
        .unwrap();

    dir
}

/// Copy the java-project-docs steering files into a temp dir, rewriting the
/// repo URL to point at the given code repo path.
fn setup_docs_dir(code_repo_url: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let src = fixture_path("java-project-docs/steering");

    copy_dir_recursive(&src, dir.path());

    // Rewrite repo URLs in all .md files to point at our temp code repo
    rewrite_repo_urls(
        dir.path(),
        "git@github.com:nexus-corp/payment-platform.git",
        code_repo_url,
    );

    dir
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            std::fs::create_dir_all(&dst_path).unwrap();
            copy_dir_recursive(&src_path, &dst_path);
        } else {
            std::fs::copy(&src_path, &dst_path).unwrap();
        }
    }
}

fn rewrite_repo_urls(dir: &Path, old_url: &str, new_url: &str) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            rewrite_repo_urls(&path, old_url, new_url);
        } else if path.extension().is_some_and(|e| e == "md") {
            let content = std::fs::read_to_string(&path).unwrap();
            if content.contains(old_url) {
                std::fs::write(&path, content.replace(old_url, new_url)).unwrap();
            }
        }
    }
}

/// Stamp real sigs on all docs in the docs dir using the code repo.
fn stamp_docs(code_repo: &Path, docs_dir: &Path) {
    let code_url = format!("file://{}", code_repo.display());
    let docs = kedge::frontmatter::scan_docs(docs_dir, &code_url, None, &[]);

    for doc in &docs {
        let doc_path = docs_dir.join(&doc.path);
        let mut updates: Vec<(&str, Option<&str>, String)> = Vec::new();
        for anchor in &doc.frontmatter.anchors {
            let code_file = code_repo.join(&anchor.path);
            if let Ok(content) = std::fs::read_to_string(&code_file) {
                let sig = kedge::detection::fingerprint::compute_sig(
                    &content,
                    &anchor.path,
                    anchor.symbol.as_deref(),
                );
                updates.push((&anchor.path, anchor.symbol.as_deref(), sig));
            }
        }
        if !updates.is_empty() {
            let batch: Vec<(&str, Option<&str>, &str)> = updates
                .iter()
                .map(|(p, s, sig)| (*p, *s, sig.as_str()))
                .collect();
            kedge::frontmatter::update_provenance_batch(&doc_path, &batch).unwrap();
        }
    }
}

#[test]
fn java_project_clean_after_stamping() {
    let code_dir = setup_code_repo();
    let code_url = format!("file://{}", code_dir.path().display());
    let docs_dir = setup_docs_dir(&code_url);

    // Stamp real signatures
    stamp_docs(code_dir.path(), docs_dir.path());

    // Detection should find no drift
    let doc_url = format!("file://{}", docs_dir.path().display());
    let report = kedge::detection::detect_drift(
        code_dir.path(),
        docs_dir.path(),
        &code_url,
        &doc_url,
        docs_dir.path(),
        "nexus-platform",
        &[],
    )
    .unwrap();

    assert!(
        report.drifted.is_empty(),
        "expected no drift after stamping, got {} drifted docs: {:?}",
        report.drifted.len(),
        report.drifted.iter().map(|d| &d.doc).collect::<Vec<_>>()
    );
    assert!(report.clean.len() > 0, "should have clean docs");
}

#[test]
fn java_project_detects_drift_after_code_change() {
    let code_dir = setup_code_repo();
    let code_url = format!("file://{}", code_dir.path().display());
    let docs_dir = setup_docs_dir(&code_url);

    // Stamp, then modify code and commit
    stamp_docs(code_dir.path(), docs_dir.path());

    // Change processPayment signature — add a riskScore parameter
    let service_path = code_dir
        .path()
        .join("core/src/main/java/com/nexus/platform/service/PaymentService.java");
    let original = std::fs::read_to_string(&service_path).unwrap();
    let modified = original.replace(
        "String cardToken, String idempotencyKey",
        "String cardToken, String idempotencyKey, int riskScore",
    );
    std::fs::write(&service_path, &modified).unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(code_dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add risk score param"])
        .current_dir(code_dir.path())
        .output()
        .unwrap();

    let doc_url = format!("file://{}", docs_dir.path().display());
    let report = kedge::detection::detect_drift(
        code_dir.path(),
        docs_dir.path(),
        &code_url,
        &doc_url,
        docs_dir.path(),
        "nexus-platform",
        &[],
    )
    .unwrap();

    assert!(
        !report.drifted.is_empty(),
        "expected drift after changing processPayment signature"
    );

    // The payment-service.md doc should be drifted since it anchors PaymentService#processPayment
    let drifted_docs: Vec<&str> = report.drifted.iter().map(|d| d.doc.as_str()).collect();
    assert!(
        drifted_docs.iter().any(|d| d.contains("payment-service")),
        "payment-service.md should be drifted, got: {:?}",
        drifted_docs
    );
}

#[test]
fn java_project_non_anchored_change_stays_clean() {
    let code_dir = setup_code_repo();
    let code_url = format!("file://{}", code_dir.path().display());
    let docs_dir = setup_docs_dir(&code_url);

    stamp_docs(code_dir.path(), docs_dir.path());

    // Change a method that is NOT anchored by any steering doc (getPayment)
    let service_path = code_dir
        .path()
        .join("core/src/main/java/com/nexus/platform/service/PaymentService.java");
    let original = std::fs::read_to_string(&service_path).unwrap();
    let modified = original.replace(
        "public Optional<Payment> getPayment",
        "public Optional<Payment> findPayment",
    );
    std::fs::write(&service_path, &modified).unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(code_dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "rename getPayment to findPayment"])
        .current_dir(code_dir.path())
        .output()
        .unwrap();

    let doc_url = format!("file://{}", docs_dir.path().display());
    let report = kedge::detection::detect_drift(
        code_dir.path(),
        docs_dir.path(),
        &code_url,
        &doc_url,
        docs_dir.path(),
        "nexus-platform",
        &[],
    )
    .unwrap();

    // processPayment and refundPayment didn't change, so payment-service.md should be clean
    let drifted_docs: Vec<&str> = report.drifted.iter().map(|d| d.doc.as_str()).collect();
    assert!(
        !drifted_docs.iter().any(|d| d.contains("payment-service")),
        "payment-service.md should NOT be drifted for unrelated method rename, got: {:?}",
        drifted_docs
    );
}

#[test]
fn java_project_install_copies_group_and_shared() {
    let docs_dir = setup_docs_dir("git@example.com:org/code.git");
    let target = TempDir::new().unwrap();

    // Install only the payments-platform group
    kedge::install::install_to_workspace(
        docs_dir.path(),
        target.path(),
        Some("payments-platform"),
        Some("AGENTS.md"),
        None,
        false,
    )
    .unwrap();

    // Should have the group-specific docs
    assert!(target.path().join("payment-service.md").exists());
    assert!(target.path().join("transaction-processing.md").exists());
    assert!(target.path().join("workflow-engine.md").exists());

    // Should have shared docs
    assert!(target.path().join("coding-standards.md").exists());
    assert!(target.path().join("api-conventions.md").exists());

    // Should have AGENTS.md
    assert!(target.path().join("AGENTS.md").exists());

    // Should NOT have platform-security docs
    assert!(!target.path().join("auth-middleware.md").exists());
    assert!(!target.path().join("crypto-standards.md").exists());
}

#[test]
fn java_project_install_filters_by_security_group() {
    let docs_dir = setup_docs_dir("git@example.com:org/code.git");
    let target = TempDir::new().unwrap();

    kedge::install::install_to_workspace(
        docs_dir.path(),
        target.path(),
        Some("platform-security"),
        None,
        None,
        false,
    )
    .unwrap();

    // Should have security docs
    assert!(target.path().join("auth-middleware.md").exists());
    assert!(target.path().join("crypto-standards.md").exists());

    // Should have shared docs
    assert!(target.path().join("coding-standards.md").exists());

    // Should NOT have payments docs
    assert!(!target.path().join("payment-service.md").exists());
}

#[test]
fn install_from_subdirectory_finds_steering_files() {
    // Simulate a docs repo where steering files live under a subdirectory,
    // matching the [[repos.docs]] path = "steering" config pattern.
    let repo_root = TempDir::new().unwrap();
    let steering_dir = repo_root.path().join("steering");

    // Copy the java-project-docs fixture into the steering/ subdirectory
    std::fs::create_dir_all(&steering_dir).unwrap();
    copy_dir_recursive(&fixture_path("java-project-docs/steering"), &steering_dir);

    let target = TempDir::new().unwrap();

    // Install from the subdirectory (not the repo root)
    kedge::install::install_to_workspace(
        &steering_dir,
        target.path(),
        Some("payments-platform"),
        Some("AGENTS.md"),
        None,
        false,
    )
    .unwrap();

    // Should find files within the subdirectory
    assert!(
        target.path().join("payment-service.md").exists(),
        "install from subdirectory should find group files"
    );
    assert!(
        target.path().join("coding-standards.md").exists(),
        "install from subdirectory should find shared files"
    );

    // Installing from repo root should NOT find them (they're nested)
    let target2 = TempDir::new().unwrap();
    kedge::install::install_to_workspace(
        repo_root.path(),
        target2.path(),
        Some("payments-platform"),
        None,
        None,
        false,
    )
    .unwrap();

    assert!(
        !target2.path().join("payment-service.md").exists(),
        "install from repo root should NOT find files nested in steering/"
    );
}
