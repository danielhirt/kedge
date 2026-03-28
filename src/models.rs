use serde::{Deserialize, Serialize};

/// A single anchor binding a doc to a code location.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Anchor {
    pub repo: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub provenance: String,
}

/// Parsed steer frontmatter from a markdown file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteerFrontmatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub anchors: Vec<Anchor>,
}

/// A doc file with its parsed steer metadata.
#[derive(Debug, Clone)]
pub struct DocFile {
    pub path: String,
    pub doc_repo: String,
    pub frontmatter: SteerFrontmatter,
    pub content: String,
    pub raw_frontmatter: String,
}

// --- Detection layer output ---

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

// --- Triage layer output ---

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

// --- Remediation layer ---

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

// --- Remediation summary ---

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

impl Severity {
    pub fn max_of(severities: &[Severity]) -> Severity {
        *severities.iter().max().expect("severities must not be empty")
    }
}
