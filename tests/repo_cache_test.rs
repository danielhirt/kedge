use std::process::Command;
use tempfile::TempDir;

fn init_git_repo(path: &std::path::Path) {
    Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
}

fn git_commit(path: &std::path::Path, msg: &str) {
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", msg])
        .current_dir(path)
        .output()
        .unwrap();
}

#[test]
fn get_or_clone_works_with_tag_ref() {
    let remote_dir = TempDir::new().unwrap();
    init_git_repo(remote_dir.path());

    std::fs::write(remote_dir.path().join("doc.md"), "v1 content").unwrap();
    git_commit(remote_dir.path(), "initial");

    // Create a tag
    Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(remote_dir.path())
        .output()
        .unwrap();

    // Add another commit so HEAD moves past the tag
    std::fs::write(remote_dir.path().join("doc.md"), "v2 content").unwrap();
    git_commit(remote_dir.path(), "post-tag");

    let repo_url = format!("file://{}", remote_dir.path().display());

    // First clone via tag
    let cache_dir =
        kedge::install::repo_cache::get_or_clone(&repo_url, "v1.0.0", 30, "origin").unwrap();
    let content = std::fs::read_to_string(cache_dir.join("doc.md")).unwrap();
    assert_eq!(
        content, "v1 content",
        "first clone should get tagged content"
    );

    // Refresh (second call hits the cached path)
    let cache_dir2 =
        kedge::install::repo_cache::get_or_clone(&repo_url, "v1.0.0", 30, "origin").unwrap();
    let content2 = std::fs::read_to_string(cache_dir2.join("doc.md")).unwrap();
    assert_eq!(
        content2, "v1 content",
        "refresh via tag should still get tagged content"
    );
}

#[test]
fn get_or_clone_refresh_picks_up_new_branch_commits() {
    let remote_dir = TempDir::new().unwrap();
    init_git_repo(remote_dir.path());

    std::fs::write(remote_dir.path().join("doc.md"), "original").unwrap();
    git_commit(remote_dir.path(), "initial");

    let repo_url = format!("file://{}", remote_dir.path().display());

    // First clone
    let cache_dir =
        kedge::install::repo_cache::get_or_clone(&repo_url, "main", 30, "origin").unwrap();
    assert_eq!(
        std::fs::read_to_string(cache_dir.join("doc.md")).unwrap(),
        "original"
    );

    // Push new content to remote
    std::fs::write(remote_dir.path().join("doc.md"), "updated").unwrap();
    git_commit(remote_dir.path(), "update");

    // Refresh should pick up the new content
    let cache_dir2 =
        kedge::install::repo_cache::get_or_clone(&repo_url, "main", 30, "origin").unwrap();
    assert_eq!(
        std::fs::read_to_string(cache_dir2.join("doc.md")).unwrap(),
        "updated"
    );
}

#[test]
fn same_url_different_refs_get_separate_cache_dirs() {
    let remote_dir = TempDir::new().unwrap();
    init_git_repo(remote_dir.path());

    std::fs::write(remote_dir.path().join("doc.md"), "main content").unwrap();
    git_commit(remote_dir.path(), "initial on main");

    // Create a branch with different content
    Command::new("git")
        .args(["checkout", "-b", "develop"])
        .current_dir(remote_dir.path())
        .output()
        .unwrap();
    std::fs::write(remote_dir.path().join("doc.md"), "develop content").unwrap();
    git_commit(remote_dir.path(), "develop commit");

    // Go back to main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(remote_dir.path())
        .output()
        .unwrap();

    let repo_url = format!("file://{}", remote_dir.path().display());

    // Clone with ref=main
    let main_cache =
        kedge::install::repo_cache::get_or_clone(&repo_url, "main", 30, "origin").unwrap();
    // Clone with ref=develop
    let dev_cache =
        kedge::install::repo_cache::get_or_clone(&repo_url, "develop", 30, "origin").unwrap();

    // Cache dirs must be different
    assert_ne!(
        main_cache, dev_cache,
        "same URL with different refs must use separate cache directories"
    );

    // Each should have its own content
    assert_eq!(
        std::fs::read_to_string(main_cache.join("doc.md")).unwrap(),
        "main content",
        "main cache should have main content"
    );
    assert_eq!(
        std::fs::read_to_string(dev_cache.join("doc.md")).unwrap(),
        "develop content",
        "develop cache should have develop content"
    );
}
