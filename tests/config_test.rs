use kedge::config::Config;
use std::path::Path;

#[test]
fn parses_full_config() {
    let config = Config::from_file(Path::new("tests/fixtures/kedge.toml")).unwrap();
    assert_eq!(config.triage.provider, "anthropic");
    assert_eq!(config.triage.model, "claude-haiku-4-5-20251001");
    assert_eq!(
        config.remediation.agent_command,
        "kiro --agent drift-updater"
    );
    assert!(config.remediation.auto_merge_severities.is_empty());
}

#[test]
fn parses_doc_repos() {
    let config = Config::from_file(Path::new("tests/fixtures/kedge.toml")).unwrap();
    assert_eq!(config.repos.docs.len(), 1);
    assert_eq!(
        config.repos.docs[0].url,
        "git@gitlab.example.com:team/docs.git"
    );
    assert_eq!(config.repos.docs[0].path, "steering/");
    assert_eq!(config.repos.docs[0].git_ref, "main");
}

#[test]
fn parses_agent_platforms() {
    let config = Config::from_file(Path::new("tests/fixtures/kedge.toml")).unwrap();
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
    let config = Config::from_file(Path::new("tests/fixtures/kedge.toml")).unwrap();
    let kiro = config.find_agent("kiro");
    assert!(kiro.is_some());
    assert_eq!(kiro.unwrap().global_steering, "~/.kiro/steering/");
    let missing = config.find_agent("nonexistent");
    assert!(missing.is_none());
}

#[test]
fn triage_config_default_values() {
    let tc = kedge::config::TriageConfig::default();
    assert_eq!(tc.provider, "command");
    assert!(tc.model.is_empty());
    assert!(tc.api_url.is_empty());
    assert!(tc.api_key_env.is_empty());
    assert!(tc.triage_command.is_empty());
    assert_eq!(tc.triage_timeout, 120);
}

#[test]
fn parses_config_with_batch() {
    let toml = r#"
[detection]

[triage]

[remediation]
agent_command = "agent"
batch = true

[repos]
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.remediation.batch);
}

#[test]
fn parses_config_with_agent_timeout_and_env() {
    let toml = r#"
[detection]

[triage]

[remediation]
agent_command = "agent"
agent_timeout = 600
agent_env = { API_KEY = "secret" }

[repos]
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.remediation.agent_timeout, 600);
    assert_eq!(
        config.remediation.agent_env.get("API_KEY").unwrap(),
        "secret"
    );
}

#[test]
fn parses_config_with_agent_instructions() {
    let toml = r#"
[detection]

[triage]

[remediation]
agent_command = "agent"
agent_instructions = "Follow our internal style guide. Use JIRA ticket format."

[repos]
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(
        config.remediation.agent_instructions,
        "Follow our internal style guide. Use JIRA ticket format."
    );
}

#[test]
fn agent_instructions_defaults_to_empty() {
    let toml = r#"
[detection]

[triage]

[remediation]
agent_command = "agent"

[repos]
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.remediation.agent_instructions.is_empty());
}

#[test]
fn parses_config_with_triage_command_timeout_env() {
    let toml = r#"
[detection]

[triage]
triage_command = "my-triage-cmd"
triage_timeout = 60
triage_env = { MODEL = "gpt-4" }

[remediation]
agent_command = "agent"

[repos]
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.triage.triage_command, "my-triage-cmd");
    assert_eq!(config.triage.triage_timeout, 60);
    assert_eq!(config.triage.triage_env.get("MODEL").unwrap(), "gpt-4");
}

#[test]
fn defaults_when_optional_fields_omitted() {
    let toml = r#"
[detection]

[triage]

[remediation]
agent_command = "agent"

[repos]
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(!config.remediation.batch);
    assert_eq!(config.remediation.agent_timeout, 300);
    assert_eq!(config.triage.triage_timeout, 120);
    assert_eq!(config.repos.git_timeout, 300);
}

#[test]
fn parses_config_with_git_timeout() {
    let toml = r#"
[detection]

[triage]

[remediation]
agent_command = "agent"

[repos]
git_timeout = 600
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.repos.git_timeout, 600);
}

#[test]
fn parses_config_with_api_url_and_key_env() {
    let toml = r#"
[detection]

[triage]
provider = "openai"
model = "gpt-4o-mini"
api_url = "https://ai-gateway.corp.com/v1"
api_key_env = "CORP_AI_KEY"

[remediation]
agent_command = "agent"

[repos]
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.triage.provider, "openai");
    assert_eq!(config.triage.model, "gpt-4o-mini");
    assert_eq!(config.triage.api_url, "https://ai-gateway.corp.com/v1");
    assert_eq!(config.triage.api_key_env, "CORP_AI_KEY");
}

#[test]
fn doc_repo_defaults_remote_name_to_origin() {
    let toml = r#"
[detection]

[triage]

[remediation]
agent_command = "agent"

[repos]
docs = [
  { url = "https://github.com/org/docs.git", path = ".", ref = "main" },
]
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.repos.docs[0].remote_name, "origin");
}

#[test]
fn doc_repo_accepts_custom_remote_name() {
    let toml = r#"
[detection]

[triage]

[remediation]
agent_command = "agent"

[repos]
docs = [
  { url = "https://github.com/org/docs.git", path = ".", ref = "main", remote_name = "upstream" },
]
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.repos.docs[0].remote_name, "upstream");
}

#[test]
fn fixture_parses_with_default_remote_name() {
    let config = Config::from_file(Path::new("tests/fixtures/kedge.toml")).unwrap();
    assert_eq!(config.repos.docs[0].remote_name, "origin");
}

#[test]
fn parses_multiple_doc_repos() {
    let toml = r#"
[detection]

[triage]

[remediation]
agent_command = "agent"

[[repos.docs]]
url = "https://github.com/org/docs1.git"
path = "steering/"
ref = "main"

[[repos.docs]]
url = "https://github.com/org/docs2.git"
path = "."
ref = "develop"

"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.repos.docs.len(), 2);
    assert_eq!(config.repos.docs[0].url, "https://github.com/org/docs1.git");
    assert_eq!(config.repos.docs[0].path, "steering/");
    assert_eq!(config.repos.docs[0].git_ref, "main");
    assert_eq!(config.repos.docs[1].url, "https://github.com/org/docs2.git");
    assert_eq!(config.repos.docs[1].path, ".");
    assert_eq!(config.repos.docs[1].git_ref, "develop");
}

#[test]
fn missing_required_section_returns_error() {
    let toml = r#"
[detection]

[triage]
"#;
    let result: Result<Config, _> = toml::from_str(toml);
    assert!(result.is_err());
}

#[test]
fn detection_config_has_default_exclude_dirs() {
    let toml = r#"
[detection]

[triage]

[remediation]
agent_command = "agent"

[repos]
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.detection.exclude_dirs.contains(&".git".to_string()));
    assert!(config
        .detection
        .exclude_dirs
        .contains(&"node_modules".to_string()));
    assert!(config
        .detection
        .exclude_dirs
        .contains(&"target".to_string()));
}

#[test]
fn detection_config_accepts_custom_exclude_dirs() {
    let toml = r#"
[detection]
exclude_dirs = ["custom_dir", "another"]

[triage]

[remediation]
agent_command = "agent"

[repos]
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.detection.exclude_dirs, vec!["custom_dir", "another"]);
}

#[test]
fn old_config_with_removed_fields_still_parses() {
    let toml = r#"
[detection]
languages = ["java", "go"]
fallback = "content-hash"

[triage]
severity_levels = ["no_update", "minor", "major"]

[remediation]
agent_command = "agent"

[repos]
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.triage.provider, "command");
}

#[test]
fn config_without_detection_section_uses_defaults() {
    let toml = r#"
[triage]

[remediation]
agent_command = "agent"

[repos]
docs = []
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.detection.exclude_dirs.contains(&".git".to_string()));
}
