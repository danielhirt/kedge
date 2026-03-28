use kedge::install;
use std::path::Path;
use tempfile::TempDir;

fn make_symlink(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[test]
fn copies_steering_files_in_workspace_mode() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();

    // Create source structure: group + shared + _kedge
    let group_dir = source_dir.path().join("payments-platform");
    let shared_dir = source_dir.path().join("shared");
    let meta_dir = source_dir.path().join("_kedge");
    std::fs::create_dir_all(&group_dir).unwrap();
    std::fs::create_dir_all(&shared_dir).unwrap();
    std::fs::create_dir_all(&meta_dir).unwrap();

    std::fs::write(group_dir.join("auth.md"), "# Auth").unwrap();
    std::fs::write(shared_dir.join("conventions.md"), "# Conventions").unwrap();
    std::fs::write(meta_dir.join("AGENTS.md"), "# Steer instructions").unwrap();
    std::fs::write(meta_dir.join("skill.md"), "# Skill").unwrap();

    let target = workspace_dir.path().join(".kiro/steering");

    install::install_to_workspace(
        source_dir.path(),
        &target,
        Some("payments-platform"),
        Some("AGENTS.md"),
        None,
    )
    .unwrap();

    assert!(target.join("auth.md").exists());
    assert!(target.join("conventions.md").exists());
    assert!(target.join("AGENTS.md").exists());
}

#[test]
fn filters_by_group() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();

    let group_a = source_dir.path().join("team-a");
    let group_b = source_dir.path().join("team-b");
    std::fs::create_dir_all(&group_a).unwrap();
    std::fs::create_dir_all(&group_b).unwrap();

    std::fs::write(group_a.join("a.md"), "# A").unwrap();
    std::fs::write(group_b.join("b.md"), "# B").unwrap();

    let target = workspace_dir.path().join(".kiro/steering");

    install::install_to_workspace(source_dir.path(), &target, Some("team-a"), None, None).unwrap();

    assert!(target.join("a.md").exists());
    assert!(!target.join("b.md").exists());
}

#[test]
fn creates_symlinks_in_link_mode() {
    let source_dir = TempDir::new().unwrap();
    let link_dir = TempDir::new().unwrap();

    let group = source_dir.path().join("my-group");
    std::fs::create_dir_all(&group).unwrap();
    std::fs::write(group.join("doc.md"), "# Doc").unwrap();

    let target = link_dir.path().join("steering");

    install::install_as_links(source_dir.path(), &target, Some("my-group"), None, None).unwrap();

    let link = target.join("doc.md");
    assert!(link.exists());
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
}

#[test]
fn install_to_workspace_rejects_group_with_path_traversal() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();
    let target = workspace_dir.path().join("steering");

    let result =
        install::install_to_workspace(source_dir.path(), &target, Some("../evil"), None, None);

    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("group"), "error should mention group: {}", msg);
}

#[test]
fn install_to_workspace_rejects_agents_file_with_path_traversal() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();
    let target = workspace_dir.path().join("steering");

    let result = install::install_to_workspace(
        source_dir.path(),
        &target,
        None,
        Some("../../.bashrc"),
        None,
    );

    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("agents_file"),
        "error should mention agents_file: {}",
        msg
    );
}

#[test]
fn install_as_links_rejects_group_with_path_traversal() {
    let source_dir = TempDir::new().unwrap();
    let link_dir = TempDir::new().unwrap();
    let target = link_dir.path().join("steering");

    let result = install::install_as_links(source_dir.path(), &target, Some("../evil"), None, None);

    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("group"), "error should mention group: {}", msg);
}

#[test]
fn install_to_workspace_handles_missing_shared_dir() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();

    // Create only a group dir, no shared dir
    let group_dir = source_dir.path().join("my-group");
    std::fs::create_dir_all(&group_dir).unwrap();
    std::fs::write(group_dir.join("doc.md"), "# Doc").unwrap();

    let target = workspace_dir.path().join("steering");

    let result =
        install::install_to_workspace(source_dir.path(), &target, Some("my-group"), None, None);

    assert!(result.is_ok());
    assert!(target.join("doc.md").exists());
}

#[test]
fn install_to_workspace_handles_missing_kedge_dir() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();

    // Create only a group dir, no _kedge dir
    let group_dir = source_dir.path().join("my-group");
    std::fs::create_dir_all(&group_dir).unwrap();
    std::fs::write(group_dir.join("doc.md"), "# Doc").unwrap();

    let target = workspace_dir.path().join("steering");

    // Request agents_file even though _kedge/AGENTS.md doesn't exist
    let result = install::install_to_workspace(
        source_dir.path(),
        &target,
        Some("my-group"),
        Some("AGENTS.md"),
        None,
    );

    assert!(result.is_ok());
    assert!(target.join("doc.md").exists());
    assert!(!target.join("AGENTS.md").exists());
}

#[cfg(unix)]
#[test]
fn install_skips_symlinked_md_files_in_group() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();
    let external = TempDir::new().unwrap();

    let group_dir = source_dir.path().join("team");
    std::fs::create_dir_all(&group_dir).unwrap();
    std::fs::write(group_dir.join("real.md"), "# Real").unwrap();

    // Symlink pointing outside the repo
    let external_file = external.path().join("secret.txt");
    std::fs::write(&external_file, "sensitive data").unwrap();
    make_symlink(&external_file, &group_dir.join("evil.md"));

    let target = workspace_dir.path().join("steering");
    install::install_to_workspace(source_dir.path(), &target, Some("team"), None, None).unwrap();

    assert!(target.join("real.md").exists());
    assert!(
        !target.join("evil.md").exists(),
        "symlinked file should be skipped"
    );
}

#[cfg(unix)]
#[test]
fn install_skips_symlinked_md_files_in_shared() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();
    let external = TempDir::new().unwrap();

    let shared_dir = source_dir.path().join("shared");
    std::fs::create_dir_all(&shared_dir).unwrap();
    std::fs::write(shared_dir.join("real.md"), "# Real").unwrap();

    let external_file = external.path().join("secret.txt");
    std::fs::write(&external_file, "sensitive data").unwrap();
    make_symlink(&external_file, &shared_dir.join("evil.md"));

    let target = workspace_dir.path().join("steering");
    install::install_to_workspace(source_dir.path(), &target, None, None, None).unwrap();

    assert!(target.join("real.md").exists());
    assert!(
        !target.join("evil.md").exists(),
        "symlinked file should be skipped"
    );
}

#[cfg(unix)]
#[test]
fn install_skips_symlinked_agents_file() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();
    let external = TempDir::new().unwrap();

    let meta_dir = source_dir.path().join("_kedge");
    std::fs::create_dir_all(&meta_dir).unwrap();

    let external_file = external.path().join("etc_hosts");
    std::fs::write(&external_file, "127.0.0.1 localhost").unwrap();
    make_symlink(&external_file, &meta_dir.join("AGENTS.md"));

    let target = workspace_dir.path().join("steering");
    install::install_to_workspace(source_dir.path(), &target, None, Some("AGENTS.md"), None)
        .unwrap();

    assert!(
        !target.join("AGENTS.md").exists(),
        "symlinked AGENTS.md should be skipped"
    );
}
