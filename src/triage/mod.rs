pub mod provider;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::models::{DriftReport, Severity, TriagedAnchor, TriagedDoc, TriagedReport};

/// Per-anchor classification returned by the AI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorClassification {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub severity: Severity,
}

/// Build the prompt sent to the AI for a single drifted doc.
pub fn build_triage_prompt(drifted_doc: &crate::models::DriftedDoc, doc_content: &str) -> String {
    let mut anchors_section = String::new();
    for anchor in &drifted_doc.anchors {
        anchors_section.push_str(&format!(
            "### Anchor: {} {}\n",
            anchor.path,
            anchor.symbol.as_deref().unwrap_or("(no symbol)")
        ));
        anchors_section.push_str(&format!("Summary: {}\n", anchor.diff_summary));
        anchors_section.push_str(&format!("Diff:\n```\n{}\n```\n\n", anchor.diff));
    }

    format!(
        r#"You are a technical writer assistant helping classify documentation drift severity.

## Documentation Content

{doc_content}

## Code Changes

The following code anchors have changed since the documentation was last updated.

{anchors_section}
## Task

For each anchor above, classify whether the documentation needs an update:

- `no_update` — The change is purely cosmetic (whitespace, formatting, internal refactor) and doesn't affect public API or documented behaviour.
- `minor` — The change is small (e.g. a new optional parameter, a renamed variable) and the documentation needs a small update.
- `major` — The change is significant (e.g. new required parameter, changed return type, removed function, changed semantics) and the documentation needs a substantial update.

Respond with a JSON array (no markdown preamble, or wrap in ```json``` fences). Each element must have:
- `path` (string) — the file path of the anchor
- `symbol` (string or null) — the symbol name, if any
- `severity` (string) — one of `no_update`, `minor`, `major`

Example:
```json
[
  {{"path": "src/Foo.java", "symbol": "Foo#bar", "severity": "minor"}},
  {{"path": "src/Baz.java", "symbol": null, "severity": "no_update"}}
]
```
"#,
        doc_content = doc_content,
        anchors_section = anchors_section,
    )
}

/// Parse the AI response, stripping markdown code fences if present.
pub fn parse_triage_response(response: &str) -> Result<Vec<AnchorClassification>> {
    let trimmed = response.trim();

    // Strip optional ```json ... ``` or ``` ... ``` fences.
    let json_str = if trimmed.starts_with("```") {
        let after_fence = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_start_matches('\n');
        let end = after_fence.rfind("```").unwrap_or(after_fence.len());
        after_fence[..end].trim()
    } else {
        trimmed
    };

    serde_json::from_str(json_str).context("failed to parse triage response as JSON")
}

/// Apply AI classifications back onto the drift report, producing a TriagedReport.
///
/// `summary` is a human-readable note attached to every TriagedDoc (the AI's
/// free-text reasoning, or a brief description of the overall change).
pub fn apply_classifications(
    drift_report: &DriftReport,
    classifications: &[AnchorClassification],
    summary: &str,
) -> TriagedReport {
    let drifted: Vec<TriagedDoc> = drift_report
        .drifted
        .iter()
        .map(|drifted_doc| {
            let triaged_anchors: Vec<TriagedAnchor> = drifted_doc
                .anchors
                .iter()
                .map(|anchor| {
                    // Find the matching classification by path + symbol.
                    let severity = classifications
                        .iter()
                        .find(|c| {
                            c.path == anchor.path && c.symbol == anchor.symbol
                        })
                        .map(|c| c.severity)
                        .unwrap_or(Severity::NoUpdate);

                    TriagedAnchor {
                        path: anchor.path.clone(),
                        symbol: anchor.symbol.clone(),
                        severity,
                        provenance: anchor.provenance.clone(),
                        diff: anchor.diff.clone(),
                    }
                })
                .collect();

            let doc_severity = Severity::max_of(
                &triaged_anchors.iter().map(|a| a.severity).collect::<Vec<_>>(),
            );

            TriagedDoc {
                doc: drifted_doc.doc.clone(),
                doc_repo: drifted_doc.doc_repo.clone(),
                severity: doc_severity,
                summary: summary.to_string(),
                anchors: triaged_anchors,
            }
        })
        .collect();

    TriagedReport {
        repo: drift_report.repo.clone(),
        git_ref: drift_report.git_ref.clone(),
        commit: drift_report.commit.clone(),
        drifted,
    }
}

/// Full async triage pipeline.
///
/// `doc_contents` maps doc path → doc markdown content.
pub async fn triage_drift_report(
    drift_report: &DriftReport,
    provider: &str,
    model: &str,
    doc_contents: &std::collections::HashMap<String, String>,
) -> Result<TriagedReport> {
    let mut all_classifications: Vec<AnchorClassification> = Vec::new();
    let mut summary_parts: Vec<String> = Vec::new();

    for drifted_doc in &drift_report.drifted {
        let doc_content = doc_contents
            .get(&drifted_doc.doc)
            .map(|s| s.as_str())
            .unwrap_or("");

        let prompt = build_triage_prompt(drifted_doc, doc_content);
        let response = provider::classify(provider, model, &prompt)
            .await
            .with_context(|| format!("triage call failed for doc {}", drifted_doc.doc))?;

        let classifications = parse_triage_response(&response)
            .with_context(|| format!("failed to parse triage response for doc {}", drifted_doc.doc))?;

        summary_parts.push(format!("{}: {} anchor(s) classified", drifted_doc.doc, classifications.len()));
        all_classifications.extend(classifications);
    }

    let summary = summary_parts.join("; ");
    Ok(apply_classifications(drift_report, &all_classifications, &summary))
}
