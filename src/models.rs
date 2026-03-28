use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Anchor {
    pub repo: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KedgeFrontmatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub anchors: Vec<Anchor>,
}

#[derive(Debug, Clone)]
pub struct DocFile {
    pub path: String,
    pub doc_repo: String,
    pub frontmatter: KedgeFrontmatter,
    pub content: String,
    pub raw_frontmatter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftedAnchor {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub provenance: String,
    pub current_commit: String,
    pub diff_summary: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftedDoc {
    pub doc: String,
    pub doc_repo: String,
    pub anchors: Vec<DriftedAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanDoc {
    pub doc: String,
    pub anchor_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub repo: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub commit: String,
    pub drifted: Vec<DriftedDoc>,
    pub clean: Vec<CleanDoc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    NoUpdate,
    Minor,
    Major,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriagedAnchor {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub severity: Severity,
    pub provenance: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriagedDoc {
    pub doc: String,
    pub doc_repo: String,
    pub severity: Severity,
    pub summary: String,
    pub anchors: Vec<TriagedAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriagedReport {
    pub repo: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub commit: String,
    pub drifted: Vec<TriagedDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTarget {
    pub repo: String,
    pub branch_prefix: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnchor {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub severity: Severity,
    pub summary: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPayload {
    pub action: String,
    pub severity: Severity,
    pub auto_merge: bool,
    pub target: AgentTarget,
    pub drifted_anchors: Vec<AgentAnchor>,
    pub instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTarget {
    pub target: AgentTarget,
    pub severity: Severity,
    pub drifted_anchors: Vec<AgentAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAgentPayload {
    pub action: String,
    pub auto_merge: bool,
    pub targets: Vec<BatchTarget>,
    pub instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediatedDoc {
    pub doc: String,
    pub repo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mr_url: Option<String>,
    pub severity: Severity,
    pub auto_merged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceSynced {
    pub doc: String,
    pub anchors_synced: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationSummary {
    pub remediated: Vec<RemediatedDoc>,
    pub provenance_advanced: Vec<ProvenanceSynced>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentResponse {
    #[serde(default)]
    pub mr_url: Option<String>,
    #[serde(default)]
    pub mr_urls: Option<Vec<String>>,
    #[serde(default)]
    pub status: String,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::NoUpdate => "no_update",
            Severity::Minor => "minor",
            Severity::Major => "major",
        }
    }
}
