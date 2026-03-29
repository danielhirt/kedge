use kedge::models::*;
use kedge::remediation;

fn make_anchor(path: &str, symbol: Option<&str>, severity: Severity) -> TriagedAnchor {
    TriagedAnchor {
        path: path.to_string(),
        symbol: symbol.map(|s| s.to_string()),
        severity,
        provenance: "prov".to_string(),
        diff: "+changed".to_string(),
    }
}

fn make_doc(doc: &str, severity: Severity, anchors: Vec<TriagedAnchor>) -> TriagedDoc {
    TriagedDoc {
        doc: doc.to_string(),
        doc_repo: "git@example.com:docs.git".to_string(),
        severity,
        summary: "drift summary".to_string(),
        anchors,
    }
}

fn make_report(drifted: Vec<TriagedDoc>) -> TriagedReport {
    TriagedReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc".to_string(),
        drifted,
    }
}

#[test]
fn builds_agent_payload_from_triaged_doc() {
    let doc = make_doc(
        "steering/auth.md",
        Severity::Major,
        vec![make_anchor(
            "src/Auth.java",
            Some("Auth#validate"),
            Severity::Major,
        )],
    );

    let payload = remediation::build_agent_payload(&doc, "def456", false, "");

    assert_eq!(payload.action, "update_docs");
    assert_eq!(payload.severity, Severity::Major);
    assert!(!payload.auto_merge);
    assert_eq!(payload.target.repo, "git@example.com:docs.git");
    assert_eq!(payload.target.path, "steering/auth.md");
    assert!(payload
        .target
        .branch_prefix
        .starts_with("kedge/auto-update"));
    assert_eq!(payload.drifted_anchors.len(), 1);
    assert!(payload.instructions.contains("Update the documentation"));
}

#[test]
fn filters_no_update_from_remediation() {
    let report = make_report(vec![make_doc(
        "auth.md",
        Severity::NoUpdate,
        vec![make_anchor("src/Auth.java", None, Severity::NoUpdate)],
    )]);

    let (to_remediate, to_sync) = remediation::partition_by_action(&report);
    assert!(to_remediate.is_empty());
    assert_eq!(to_sync.len(), 1);
}

#[test]
fn auto_merge_flag_set_for_configured_severities() {
    let auto_merge_severities = vec!["minor".to_string()];
    assert!(remediation::should_auto_merge(
        Severity::Minor,
        &auto_merge_severities
    ));
    assert!(!remediation::should_auto_merge(
        Severity::Major,
        &auto_merge_severities
    ));
    assert!(!remediation::should_auto_merge(
        Severity::NoUpdate,
        &auto_merge_severities
    ));
}

// --- build_batch_agent_payload ---

#[test]
fn batch_payload_creates_targets_from_multiple_docs() {
    let doc_a = make_doc(
        "auth.md",
        Severity::Minor,
        vec![make_anchor("src/Auth.java", None, Severity::Minor)],
    );
    let doc_b = make_doc(
        "billing.md",
        Severity::Major,
        vec![make_anchor(
            "src/Billing.java",
            Some("Billing#charge"),
            Severity::Major,
        )],
    );

    let docs: Vec<&TriagedDoc> = vec![&doc_a, &doc_b];
    let payload = remediation::build_batch_agent_payload(
        &docs,
        "commit99",
        &["minor".to_string(), "major".to_string()],
        "",
    );

    assert_eq!(payload.targets.len(), 2);
    assert_eq!(payload.targets[0].target.path, "auth.md");
    assert_eq!(payload.targets[1].target.path, "billing.md");
    assert_eq!(payload.targets[0].severity, Severity::Minor);
    assert_eq!(payload.targets[1].severity, Severity::Major);
}

#[test]
fn batch_payload_auto_merge_true_when_all_qualify() {
    let doc = make_doc(
        "api.md",
        Severity::Minor,
        vec![make_anchor("src/Api.java", None, Severity::Minor)],
    );

    let docs: Vec<&TriagedDoc> = vec![&doc];
    let payload = remediation::build_batch_agent_payload(&docs, "c1", &["minor".to_string()], "");

    assert!(payload.auto_merge);
}

#[test]
fn batch_payload_auto_merge_false_when_any_doc_doesnt_qualify() {
    let minor_doc = make_doc(
        "a.md",
        Severity::Minor,
        vec![make_anchor("src/A.java", None, Severity::Minor)],
    );
    let major_doc = make_doc(
        "b.md",
        Severity::Major,
        vec![make_anchor("src/B.java", None, Severity::Major)],
    );

    let docs: Vec<&TriagedDoc> = vec![&minor_doc, &major_doc];
    let payload = remediation::build_batch_agent_payload(&docs, "c1", &["minor".to_string()], "");

    assert!(!payload.auto_merge);
}

#[test]
fn batch_payload_auto_merge_false_for_empty_docs() {
    let docs: Vec<&TriagedDoc> = vec![];
    let payload = remediation::build_batch_agent_payload(
        &docs,
        "c1",
        &["minor".to_string(), "major".to_string()],
        "",
    );

    assert!(!payload.auto_merge);
    assert!(payload.targets.is_empty());
}

#[test]
fn batch_payload_filters_no_update_anchors_from_targets() {
    let doc = make_doc(
        "mixed.md",
        Severity::Minor,
        vec![
            make_anchor("src/Clean.java", None, Severity::NoUpdate),
            make_anchor("src/Dirty.java", Some("Dirty#run"), Severity::Minor),
        ],
    );

    let docs: Vec<&TriagedDoc> = vec![&doc];
    let payload = remediation::build_batch_agent_payload(&docs, "c1", &["minor".to_string()], "");

    assert_eq!(payload.targets.len(), 1);
    assert_eq!(payload.targets[0].drifted_anchors.len(), 1);
    assert_eq!(payload.targets[0].drifted_anchors[0].path, "src/Dirty.java");
}

#[test]
fn batch_payload_uses_update_docs_batch_action() {
    let doc = make_doc(
        "any.md",
        Severity::Minor,
        vec![make_anchor("src/X.java", None, Severity::Minor)],
    );

    let docs: Vec<&TriagedDoc> = vec![&doc];
    let payload = remediation::build_batch_agent_payload(&docs, "c1", &["minor".to_string()], "");

    assert_eq!(payload.action, "update_docs_batch");
}

// Regression: batch auto_merge must be consistent across all docs.
// In a mixed-severity batch where only "minor" qualifies for auto-merge,
// the batch-level flag should be false — and ALL docs in the summary must
// reflect that batch-level decision, not per-doc recalculation.
#[test]
fn batch_auto_merge_is_all_or_nothing() {
    let minor_doc = make_doc(
        "a.md",
        Severity::Minor,
        vec![make_anchor("src/A.java", None, Severity::Minor)],
    );
    let major_doc = make_doc(
        "b.md",
        Severity::Major,
        vec![make_anchor("src/B.java", None, Severity::Major)],
    );

    let only_minor = vec!["minor".to_string()];
    let docs: Vec<&TriagedDoc> = vec![&minor_doc, &major_doc];
    let payload = remediation::build_batch_agent_payload(&docs, "c1", &only_minor, "");

    // Batch-level: false because major_doc doesn't qualify
    assert!(!payload.auto_merge);

    // The caller (main.rs) must use payload.auto_merge for ALL docs,
    // not recalculate per-doc. Verify the per-doc check would disagree:
    assert!(remediation::should_auto_merge(Severity::Minor, &only_minor));
    assert!(!remediation::should_auto_merge(
        Severity::Major,
        &only_minor
    ));

    // If the caller used per-doc checks, minor_doc would incorrectly be auto_merged=true
    // while the batch was sent with auto_merge=false. The batch flag is authoritative.
}

// --- should_auto_merge edge cases ---

#[test]
fn should_auto_merge_is_case_insensitive() {
    let severities = vec!["MINOR".to_string(), "Major".to_string()];
    assert!(remediation::should_auto_merge(Severity::Minor, &severities));
    assert!(remediation::should_auto_merge(Severity::Major, &severities));
    assert!(!remediation::should_auto_merge(
        Severity::NoUpdate,
        &severities
    ));
}

#[test]
fn should_auto_merge_empty_list_matches_nothing() {
    let empty: Vec<String> = vec![];
    assert!(!remediation::should_auto_merge(Severity::Minor, &empty));
    assert!(!remediation::should_auto_merge(Severity::Major, &empty));
    assert!(!remediation::should_auto_merge(Severity::NoUpdate, &empty));
}

// --- build_agent_payload edge cases ---

#[test]
fn drifted_agent_anchors_filters_all_no_update() {
    // Doc has Major severity but all anchors are NoUpdate
    let doc = make_doc(
        "edge.md",
        Severity::Major,
        vec![
            make_anchor("src/A.java", None, Severity::NoUpdate),
            make_anchor("src/B.java", None, Severity::NoUpdate),
        ],
    );

    let payload = remediation::build_agent_payload(&doc, "abc", false, "");
    assert!(payload.drifted_anchors.is_empty());
}

// --- partition_by_action (additional cases) ---

#[test]
fn partition_empty_report_returns_empty_lists() {
    let report = make_report(vec![]);

    let (to_remediate, to_sync) = remediation::partition_by_action(&report);
    assert!(to_remediate.is_empty());
    assert!(to_sync.is_empty());
}

#[test]
fn partition_all_docs_need_remediation() {
    let report = make_report(vec![
        make_doc(
            "a.md",
            Severity::Minor,
            vec![make_anchor("src/A.java", None, Severity::Minor)],
        ),
        make_doc(
            "b.md",
            Severity::Major,
            vec![make_anchor("src/B.java", None, Severity::Major)],
        ),
    ]);

    let (to_remediate, to_sync) = remediation::partition_by_action(&report);
    assert_eq!(to_remediate.len(), 2);
    assert!(to_sync.is_empty());
}

#[test]
fn partition_mixed_severities() {
    let report = make_report(vec![
        make_doc(
            "clean.md",
            Severity::NoUpdate,
            vec![make_anchor("src/A.java", None, Severity::NoUpdate)],
        ),
        make_doc(
            "dirty.md",
            Severity::Minor,
            vec![make_anchor("src/B.java", None, Severity::Minor)],
        ),
        make_doc(
            "critical.md",
            Severity::Major,
            vec![make_anchor("src/C.java", None, Severity::Major)],
        ),
    ]);

    let (to_remediate, to_sync) = remediation::partition_by_action(&report);
    assert_eq!(to_remediate.len(), 2);
    assert_eq!(to_sync.len(), 1);
    assert_eq!(to_sync[0].doc, "clean.md");
}

// --- custom agent_instructions ---

#[test]
fn agent_payload_uses_default_instructions_when_empty() {
    let doc = make_doc(
        "auth.md",
        Severity::Minor,
        vec![make_anchor("src/Auth.java", None, Severity::Minor)],
    );

    let payload = remediation::build_agent_payload(&doc, "abc123", false, "");

    assert!(payload.instructions.contains("Update the documentation"));
}

#[test]
fn agent_payload_overrides_with_custom_instructions() {
    let doc = make_doc(
        "auth.md",
        Severity::Minor,
        vec![make_anchor("src/Auth.java", None, Severity::Minor)],
    );

    let payload =
        remediation::build_agent_payload(&doc, "abc123", false, "Follow our style guide.");

    assert_eq!(payload.instructions, "Follow our style guide.");
}

#[test]
fn batch_payload_overrides_with_custom_instructions() {
    let doc = make_doc(
        "api.md",
        Severity::Minor,
        vec![make_anchor("src/Api.java", None, Severity::Minor)],
    );

    let docs: Vec<&TriagedDoc> = vec![&doc];
    let payload = remediation::build_batch_agent_payload(
        &docs,
        "def456",
        &["minor".to_string()],
        "Use conventional commits.",
    );

    assert_eq!(payload.instructions, "Use conventional commits.");
}
