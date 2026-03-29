use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub detection: DetectionConfig,
    pub triage: TriageConfig,
    pub remediation: RemediationConfig,
    pub repos: ReposConfig,
    #[serde(default)]
    pub agents: Vec<AgentPlatform>,
}

#[derive(Debug, Deserialize)]
pub struct DetectionConfig {
    pub languages: Vec<String>,
    #[serde(default = "default_fallback")]
    pub fallback: String,
    #[serde(default = "default_exclude_dirs")]
    pub exclude_dirs: Vec<String>,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            languages: vec![],
            fallback: default_fallback(),
            exclude_dirs: default_exclude_dirs(),
        }
    }
}

fn default_fallback() -> String {
    "content-hash".to_string()
}

fn default_exclude_dirs() -> Vec<String> {
    vec![
        ".git".into(),
        "node_modules".into(),
        "target".into(),
        ".venv".into(),
        "__pycache__".into(),
        ".tox".into(),
        "vendor".into(),
    ]
}

#[derive(Debug, Deserialize)]
pub struct TriageConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub api_key_env: String,
    #[serde(default)]
    pub triage_command: String,
    #[serde(default = "default_triage_timeout")]
    pub triage_timeout: u64,
    #[serde(default)]
    pub triage_env: HashMap<String, String>,
    #[serde(default = "default_severity_levels")]
    pub severity_levels: Vec<String>,
}

fn default_provider() -> String {
    "command".to_string()
}

impl Default for TriageConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: String::new(),
            api_url: String::new(),
            api_key_env: String::new(),
            triage_command: String::new(),
            triage_timeout: default_triage_timeout(),
            triage_env: HashMap::new(),
            severity_levels: default_severity_levels(),
        }
    }
}

fn default_triage_timeout() -> u64 {
    120
}

fn default_severity_levels() -> Vec<String> {
    vec!["no_update".into(), "minor".into(), "major".into()]
}

#[derive(Debug, Deserialize)]
pub struct RemediationConfig {
    pub agent_command: String,
    #[serde(default)]
    pub auto_merge_severities: Vec<String>,
    #[serde(default)]
    pub batch: bool,
    #[serde(default = "default_agent_timeout")]
    pub agent_timeout: u64,
    #[serde(default)]
    pub agent_env: HashMap<String, String>,
}

fn default_agent_timeout() -> u64 {
    300
}

#[derive(Debug, Deserialize)]
pub struct ReposConfig {
    pub docs: Vec<DocRepo>,
    #[serde(default = "default_git_timeout")]
    pub git_timeout: u64,
}

fn default_git_timeout() -> u64 {
    300
}

#[derive(Debug, Deserialize)]
pub struct DocRepo {
    pub url: String,
    pub path: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    #[serde(default = "default_remote_name")]
    pub remote_name: String,
}

fn default_remote_name() -> String {
    "origin".to_string()
}

#[derive(Debug, Deserialize)]
pub struct AgentPlatform {
    pub name: String,
    pub global_steering: String,
    pub workspace_steering: String,
    pub agents_file: String,
    #[serde(default)]
    pub skill_dir: String,
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config: {}", path.display()))?;
        Ok(config)
    }

    pub fn find_agent(&self, name: &str) -> Option<&AgentPlatform> {
        self.agents.iter().find(|a| a.name == name)
    }
}
