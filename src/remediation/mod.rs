pub mod agent;

use crate::models::{AgentAnchor, AgentPayload, AgentTarget, Severity, TriagedDoc, TriagedReport};

const ACTION_UPDATE_DOCS: &str = "update_docs";
const ACTION_UPDATE_DOCS_BATCH: &str = "update_docs_batch";
const BRANCH_PREFIX: &str = "kedge/auto-update";

fn drifted_agent_anchors(doc: &TriagedDoc) -> Vec<AgentAnchor> {
    doc.anchors
        .iter()
        .filter(|a| a.severity != Severity::NoUpdate)
        .map(|a| AgentAnchor {
            path: a.path.clone(),
            symbol: a.symbol.clone(),
            severity: a.severity,
            current_sig: a.current_sig.clone(),
            summary: doc.summary.clone(),
            diff: a.diff.clone(),
        })
        .collect()
}

const DEFAULT_INSTRUCTIONS: &str = "Update the documentation to reflect the code changes described in the drifted anchors. After updating the prose, run `kedge sync` to advance provenance automatically. If kedge is not available in your environment, set each anchor's provenance in the frontmatter to the corresponding current_sig value as a fallback.";

pub fn build_agent_payload(
    doc: &TriagedDoc,
    _current_commit: &str,
    auto_merge: bool,
    custom_instructions: &str,
) -> AgentPayload {
    let instructions = if custom_instructions.is_empty() {
        DEFAULT_INSTRUCTIONS.to_string()
    } else {
        custom_instructions.to_string()
    };

    AgentPayload {
        action: ACTION_UPDATE_DOCS.to_string(),
        severity: doc.severity,
        auto_merge,
        target: AgentTarget {
            repo: doc.doc_repo.clone(),
            branch_prefix: BRANCH_PREFIX.to_string(),
            path: doc.doc.clone(),
        },
        drifted_anchors: drifted_agent_anchors(doc),
        instructions,
    }
}

pub fn partition_by_action(report: &TriagedReport) -> (Vec<&TriagedDoc>, Vec<&TriagedDoc>) {
    let mut to_remediate: Vec<&TriagedDoc> = Vec::new();
    let mut to_sync: Vec<&TriagedDoc> = Vec::new();

    for doc in &report.drifted {
        let needs_update = doc.anchors.iter().any(|a| a.severity != Severity::NoUpdate);

        if needs_update {
            to_remediate.push(doc);
        } else {
            to_sync.push(doc);
        }
    }

    (to_remediate, to_sync)
}

/// `auto_merge` is true only if every target qualifies individually.
pub fn build_batch_agent_payload(
    docs: &[&TriagedDoc],
    _current_commit: &str,
    auto_merge_severities: &[String],
    custom_instructions: &str,
) -> crate::models::BatchAgentPayload {
    let mut all_qualify_for_auto_merge = true;
    let mut targets: Vec<crate::models::BatchTarget> = Vec::with_capacity(docs.len());

    for doc in docs {
        if !should_auto_merge(doc.severity, auto_merge_severities) {
            all_qualify_for_auto_merge = false;
        }
        targets.push(crate::models::BatchTarget {
            target: AgentTarget {
                repo: doc.doc_repo.clone(),
                branch_prefix: BRANCH_PREFIX.to_string(),
                path: doc.doc.clone(),
            },
            severity: doc.severity,
            drifted_anchors: drifted_agent_anchors(doc),
        });
    }

    let instructions = if custom_instructions.is_empty() {
        DEFAULT_INSTRUCTIONS.to_string()
    } else {
        custom_instructions.to_string()
    };

    crate::models::BatchAgentPayload {
        action: ACTION_UPDATE_DOCS_BATCH.to_string(),
        auto_merge: all_qualify_for_auto_merge && !docs.is_empty(),
        targets,
        instructions,
    }
}

/// Case-insensitive match against snake_case severity names.
pub fn should_auto_merge(severity: Severity, auto_merge_severities: &[String]) -> bool {
    auto_merge_severities
        .iter()
        .any(|s| s.eq_ignore_ascii_case(severity.as_str()))
}
