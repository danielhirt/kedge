use steer::models::*;
use steer::triage;

#[test]
fn builds_triage_prompt_from_drift_report() {
    let drifted_doc = DriftedDoc {
        doc: "auth.md".to_string(),
        doc_repo: "git@example.com:docs.git".to_string(),
        anchors: vec![DriftedAnchor {
            path: "src/Auth.java".to_string(),
            symbol: Some("Auth#validate".to_string()),
            provenance: "abc123".to_string(),
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
                    current_commit: "abc123".to_string(),
                    diff_summary: "Added param".to_string(),
                    diff: "+param".to_string(),
                },
                DriftedAnchor {
                    path: "src/Auth.java".to_string(),
                    symbol: Some("Auth#refresh".to_string()),
                    provenance: "old2".to_string(),
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

    let triaged = triage::apply_classifications(&drift_report, &classifications, "Auth was updated with scopes");
    assert_eq!(triaged.drifted.len(), 1);
    assert_eq!(triaged.drifted[0].severity, Severity::Major); // max of anchors
    assert_eq!(triaged.drifted[0].anchors.len(), 2); // both anchors preserved
    assert_eq!(triaged.drifted[0].anchors[0].severity, Severity::Major);
    assert_eq!(triaged.drifted[0].anchors[1].severity, Severity::NoUpdate);
}
