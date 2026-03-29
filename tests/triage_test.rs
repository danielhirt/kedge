use kedge::models::*;
use kedge::triage;

#[test]
fn builds_triage_prompt_from_drift_report() {
    let drifted_doc = DriftedDoc {
        doc: "auth.md".to_string(),
        doc_repo: "git@example.com:docs.git".to_string(),
        anchors: vec![DriftedAnchor {
            path: "src/Auth.java".to_string(),
            symbol: Some("Auth#validate".to_string()),
            provenance: "abc123".to_string(),
            current_sig: "sig:1111111111111111".to_string(),
            current_commit: "def456".to_string(),
            diff_summary: "Added scopes param".to_string(),
            diff: "+public boolean validate(String t, List<String> scopes)".to_string(),
        }],
    };

    let prompt = triage::build_triage_prompt(&drifted_doc, "# Auth docs\nValidates tokens.");
    assert!(prompt.contains("# Auth docs"));
    assert!(prompt.contains("Auth#validate"));
    assert!(prompt.contains("scopes"));
    assert!(prompt.contains("no_update"));
    assert!(prompt.contains("minor"));
    assert!(prompt.contains("major"));
}

#[test]
fn parses_triage_response_json() {
    let response = r#"[
        {"path": "src/Auth.java", "symbol": "Auth#validate", "severity": "major"}
    ]"#;

    let classifications = triage::parse_triage_response(response).unwrap();
    assert_eq!(classifications.len(), 1);
    assert_eq!(classifications[0].severity, Severity::Major);
}

#[test]
fn parses_triage_response_with_code_fences() {
    let response = "```json\n[{\"path\": \"src/Auth.java\", \"symbol\": \"Auth#validate\", \"severity\": \"minor\"}]\n```";
    let classifications = triage::parse_triage_response(response).unwrap();
    assert_eq!(classifications.len(), 1);
    assert_eq!(classifications[0].severity, Severity::Minor);
}

#[test]
fn applies_triage_to_drift_report() {
    let drift_report = DriftReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc123".to_string(),
        drifted: vec![DriftedDoc {
            doc: "auth.md".to_string(),
            doc_repo: "git@example.com:docs.git".to_string(),
            anchors: vec![
                DriftedAnchor {
                    path: "src/Auth.java".to_string(),
                    symbol: Some("Auth#validate".to_string()),
                    provenance: "old1".to_string(),
                    current_sig: "sig:1111111111111111".to_string(),
                    current_commit: "abc123".to_string(),
                    diff_summary: "Added param".to_string(),
                    diff: "+param".to_string(),
                },
                DriftedAnchor {
                    path: "src/Auth.java".to_string(),
                    symbol: Some("Auth#refresh".to_string()),
                    provenance: "old2".to_string(),
                    current_sig: "sig:2222222222222222".to_string(),
                    current_commit: "abc123".to_string(),
                    diff_summary: "Whitespace".to_string(),
                    diff: " ".to_string(),
                },
            ],
        }],
        clean: vec![],
    };

    let classifications = vec![
        triage::AnchorClassification {
            path: "src/Auth.java".to_string(),
            symbol: Some("Auth#validate".to_string()),
            severity: Severity::Major,
        },
        triage::AnchorClassification {
            path: "src/Auth.java".to_string(),
            symbol: Some("Auth#refresh".to_string()),
            severity: Severity::NoUpdate,
        },
    ];

    let triaged = triage::apply_classifications(
        &drift_report,
        &classifications,
        "Auth was updated with scopes",
    );
    assert_eq!(triaged.drifted.len(), 1);
    assert_eq!(triaged.drifted[0].severity, Severity::Major); // max of anchors
    assert_eq!(triaged.drifted[0].anchors.len(), 2); // both anchors preserved
    assert_eq!(triaged.drifted[0].anchors[0].severity, Severity::Major);
    assert_eq!(triaged.drifted[0].anchors[1].severity, Severity::NoUpdate);
}

#[test]
fn parse_triage_response_malformed_json_returns_error() {
    let result = triage::parse_triage_response("not valid json {{{");
    assert!(result.is_err());
}

#[test]
fn parse_triage_response_empty_string_returns_error() {
    let result = triage::parse_triage_response("");
    assert!(result.is_err());
}

#[test]
fn parse_triage_response_empty_json_array_returns_empty_vec() {
    let classifications = triage::parse_triage_response("[]").unwrap();
    assert!(classifications.is_empty());
}

#[test]
fn apply_classifications_unmatched_anchor_defaults_to_no_update() {
    let drift_report = DriftReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc123".to_string(),
        drifted: vec![DriftedDoc {
            doc: "readme.md".to_string(),
            doc_repo: "git@example.com:docs.git".to_string(),
            anchors: vec![DriftedAnchor {
                path: "src/Unmatched.java".to_string(),
                symbol: Some("Unmatched#method".to_string()),
                provenance: "old".to_string(),
                current_sig: "sig:3333333333333333".to_string(),
                current_commit: "abc123".to_string(),
                diff_summary: "Changed".to_string(),
                diff: "+change".to_string(),
            }],
        }],
        clean: vec![],
    };

    // Classification for a completely different path/symbol.
    let classifications = vec![triage::AnchorClassification {
        path: "src/Other.java".to_string(),
        symbol: Some("Other#foo".to_string()),
        severity: Severity::Major,
    }];

    let triaged = triage::apply_classifications(&drift_report, &classifications, "test");
    assert_eq!(triaged.drifted[0].anchors[0].severity, Severity::NoUpdate);
}

#[test]
fn apply_classifications_empty_classifications_all_default_to_no_update() {
    let drift_report = DriftReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc123".to_string(),
        drifted: vec![DriftedDoc {
            doc: "readme.md".to_string(),
            doc_repo: "git@example.com:docs.git".to_string(),
            anchors: vec![
                DriftedAnchor {
                    path: "src/A.java".to_string(),
                    symbol: Some("A#one".to_string()),
                    provenance: "old1".to_string(),
                    current_sig: "sig:1111111111111111".to_string(),
                    current_commit: "abc123".to_string(),
                    diff_summary: "Changed".to_string(),
                    diff: "+a".to_string(),
                },
                DriftedAnchor {
                    path: "src/B.java".to_string(),
                    symbol: None,
                    provenance: "old2".to_string(),
                    current_sig: "sig:2222222222222222".to_string(),
                    current_commit: "abc123".to_string(),
                    diff_summary: "Changed".to_string(),
                    diff: "+b".to_string(),
                },
            ],
        }],
        clean: vec![],
    };

    let triaged = triage::apply_classifications(&drift_report, &[], "test");
    assert_eq!(triaged.drifted[0].anchors[0].severity, Severity::NoUpdate);
    assert_eq!(triaged.drifted[0].anchors[1].severity, Severity::NoUpdate);
    assert_eq!(triaged.drifted[0].severity, Severity::NoUpdate);
}
