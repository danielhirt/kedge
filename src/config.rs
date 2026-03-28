use anyhow::{Context, Result};
use serde::Deserialize;
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
}

fn default_fallback() -> String {
    "content-hash".to_string()
}

#[derive(Debug, Deserialize)]
pub struct TriageConfig {
    pub provider: String,
    pub model: String,
    #[serde(default = "default_severity_levels")]
    pub severity_levels: Vec<String>,
}

fn default_severity_levels() -> Vec<String> {
    vec!["no_update".into(), "minor".into(), "major".into()]
}

#[derive(Debug, Deserialize)]
pub struct RemediationConfig {
    pub agent_command: String,
    #[serde(default)]
    pub auto_merge_severities: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReposConfig {
    pub docs: Vec<DocRepo>,
}

#[derive(Debug, Deserialize)]
pub struct DocRepo {
    pub url: String,
    pub path: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
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
