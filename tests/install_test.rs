use steer::install;
use tempfile::TempDir;

#[test]
fn copies_steering_files_in_workspace_mode() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();

    // Create source structure: group + shared + _steer
    let group_dir = source_dir.path().join("payments-platform");
    let shared_dir = source_dir.path().join("shared");
    let meta_dir = source_dir.path().join("_steer");
    std::fs::create_dir_all(&group_dir).unwrap();
    std::fs::create_dir_all(&shared_dir).unwrap();
    std::fs::create_dir_all(&meta_dir).unwrap();

    std::fs::write(group_dir.join("auth.md"), "# Auth").unwrap();
    std::fs::write(shared_dir.join("conventions.md"), "# Conventions").unwrap();
    std::fs::write(meta_dir.join("AGENTS.md"), "# Steer instructions").unwrap();
    std::fs::write(meta_dir.join("skill.md"), "# Skill").unwrap();

    let target = workspace_dir.path().join(".kiro/steering");

    install::install_to_workspace(
        source_dir.path(), &target, Some("payments-platform"),
        Some("AGENTS.md"), None,
    ).unwrap();

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

    install::install_to_workspace(
        source_dir.path(), &target, Some("team-a"), None, None,
    ).unwrap();

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

    install::install_as_links(
        source_dir.path(), &target, Some("my-group"), None, None,
    ).unwrap();

    let link = target.join("doc.md");
    assert!(link.exists());
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
}
