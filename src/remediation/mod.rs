pub mod agent;

use crate::models::{
    AgentAnchor, AgentPayload, AgentTarget, Severity, TriagedDoc, TriagedReport,
};

/// Build an [`AgentPayload`] from a single [`TriagedDoc`].
///
/// * `current_commit` — the HEAD commit SHA of the code repo; embedded in
///   `instructions` for provenance stamping by the agent.
/// * `auto_merge` — whether the agent should open the MR in auto-merge mode.
///
/// Anchors classified as [`Severity::NoUpdate`] are filtered out: they don't
/// require any documentation change.
pub fn build_agent_payload(
    doc: &TriagedDoc,
    current_commit: &str,
    auto_merge: bool,
) -> AgentPayload {
    let drifted_anchors: Vec<AgentAnchor> = doc
        .anchors
        .iter()
        .filter(|a| a.severity != Severity::NoUpdate)
        .map(|a| AgentAnchor {
            path: a.path.clone(),
            symbol: a.symbol.clone(),
            severity: a.severity,
            summary: doc.summary.clone(),
            diff: a.diff.clone(),
        })
        .collect();

    let instructions = format!(
        "Update documentation for commit {}. \
         Apply the changes described in the drifted anchors and stamp provenance with {}.",
        current_commit, current_commit
    );

    AgentPayload {
        action: "update_docs".to_string(),
        severity: doc.severity,
        auto_merge,
        target: AgentTarget {
            repo: doc.doc_repo.clone(),
            branch_prefix: "steer/auto-update".to_string(),
            path: doc.doc.clone(),
        },
        drifted_anchors,
        instructions,
    }
}

/// Split a [`TriagedReport`] into docs that need agent remediation and docs
/// that only need a provenance sync (all anchors are [`Severity::NoUpdate`]).
///
/// Returns `(to_remediate, to_sync)`.
pub fn partition_by_action<'a>(
    report: &'a TriagedReport,
) -> (Vec<&'a TriagedDoc>, Vec<&'a TriagedDoc>) {
    let mut to_remediate: Vec<&TriagedDoc> = Vec::new();
    let mut to_sync: Vec<&TriagedDoc> = Vec::new();

    for doc in &report.drifted {
        let needs_update = doc
            .anchors
            .iter()
            .any(|a| a.severity != Severity::NoUpdate);

        if needs_update {
            to_remediate.push(doc);
        } else {
            to_sync.push(doc);
        }
    }

    (to_remediate, to_sync)
}

/// Returns `true` if `severity` is listed in `auto_merge_severities`.
///
/// The comparison is case-insensitive against the snake_case serde
/// representation (`"minor"`, `"major"`, `"no_update"`).
pub fn should_auto_merge(severity: Severity, auto_merge_severities: &[String]) -> bool {
    let severity_str = match severity {
        Severity::NoUpdate => "no_update",
        Severity::Minor => "minor",
        Severity::Major => "major",
    };
    auto_merge_severities
        .iter()
        .any(|s| s.eq_ignore_ascii_case(severity_str))
}
