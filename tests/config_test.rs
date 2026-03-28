use steer::config::Config;
use std::path::Path;

#[test]
fn parses_full_config() {
    let config = Config::from_file(Path::new("tests/fixtures/steer.toml")).unwrap();
    assert_eq!(config.detection.languages, vec!["java", "go", "typescript", "xml"]);
    assert_eq!(config.detection.fallback, "content-hash");
    assert_eq!(config.triage.provider, "anthropic");
    assert_eq!(config.triage.model, "claude-haiku-4-5-20251001");
    assert_eq!(config.remediation.agent_command, "kiro --agent drift-updater");
    assert!(config.remediation.auto_merge_severities.is_empty());
}

#[test]
fn parses_doc_repos() {
    let config = Config::from_file(Path::new("tests/fixtures/steer.toml")).unwrap();
    assert_eq!(config.repos.docs.len(), 1);
    assert_eq!(config.repos.docs[0].url, "git@gitlab.example.com:team/docs.git");
    assert_eq!(config.repos.docs[0].path, "steering/");
    assert_eq!(config.repos.docs[0].git_ref, "main");
}

#[test]
fn parses_agent_platforms() {
    let config = Config::from_file(Path::new("tests/fixtures/steer.toml")).unwrap();
    assert_eq!(config.agents.len(), 2);
    assert_eq!(config.agents[0].name, "kiro");
    assert_eq!(config.agents[0].global_steering, "~/.kiro/steering/");
    assert_eq!(config.agents[0].workspace_steering, ".kiro/steering/");
    assert_eq!(config.agents[0].agents_file, "AGENTS.md");
    assert_eq!(config.agents[1].name, "claude");
    assert_eq!(config.agents[1].agents_file, "CLAUDE.md");
}

#[test]
fn returns_error_for_missing_file() {
    let result = Config::from_file(Path::new("nonexistent.toml"));
    assert!(result.is_err());
}

#[test]
fn finds_agent_by_name() {
    let config = Config::from_file(Path::new("tests/fixtures/steer.toml")).unwrap();
    let kiro = config.find_agent("kiro");
    assert!(kiro.is_some());
    assert_eq!(kiro.unwrap().global_steering, "~/.kiro/steering/");
    let missing = config.find_agent("nonexistent");
    assert!(missing.is_none());
}
