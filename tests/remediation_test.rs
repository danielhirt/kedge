use steer::models::*;
use steer::remediation;

#[test]
fn builds_agent_payload_from_triaged_doc() {
    let triaged_doc = TriagedDoc {
        doc: "steering/auth.md".to_string(),
        doc_repo: "git@example.com:docs.git".to_string(),
        severity: Severity::Major,
        summary: "Auth model changed to scope-based".to_string(),
        anchors: vec![TriagedAnchor {
            path: "src/Auth.java".to_string(),
            symbol: Some("Auth#validate".to_string()),
            severity: Severity::Major,
            provenance: "abc123".to_string(),
            diff: "+scopes param".to_string(),
        }],
    };

    let payload = remediation::build_agent_payload(&triaged_doc, "def456", false);

    assert_eq!(payload.action, "update_docs");
    assert_eq!(payload.severity, Severity::Major);
    assert!(!payload.auto_merge);
    assert_eq!(payload.target.repo, "git@example.com:docs.git");
    assert_eq!(payload.target.path, "steering/auth.md");
    assert!(payload.target.branch_prefix.starts_with("steer/auto-update"));
    assert_eq!(payload.drifted_anchors.len(), 1);
    assert!(payload.instructions.contains("def456"));
}

#[test]
fn filters_no_update_from_remediation() {
    let triaged = TriagedReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc".to_string(),
        drifted: vec![TriagedDoc {
            doc: "auth.md".to_string(),
            doc_repo: "git@example.com:docs.git".to_string(),
            severity: Severity::NoUpdate,
            summary: "No real changes".to_string(),
            anchors: vec![TriagedAnchor {
                path: "src/Auth.java".to_string(),
                symbol: None,
                severity: Severity::NoUpdate,
                provenance: "old".to_string(),
                diff: " ".to_string(),
            }],
        }],
    };

    let (to_remediate, to_sync) = remediation::partition_by_action(&triaged);
    assert!(to_remediate.is_empty());
    assert_eq!(to_sync.len(), 1);
}

#[test]
fn auto_merge_flag_set_for_configured_severities() {
    let auto_merge_severities = vec!["minor".to_string()];
    assert!(remediation::should_auto_merge(Severity::Minor, &auto_merge_severities));
    assert!(!remediation::should_auto_merge(Severity::Major, &auto_merge_severities));
    assert!(!remediation::should_auto_merge(Severity::NoUpdate, &auto_merge_severities));
}
