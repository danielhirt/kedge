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
fn apply_classifications_unmatched_anchor_defaults_to_minor() {
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
    assert_eq!(triaged.drifted[0].anchors[0].severity, Severity::Minor);
}

#[test]
fn apply_classifications_empty_classifications_all_default_to_minor() {
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
    assert_eq!(triaged.drifted[0].anchors[0].severity, Severity::Minor);
    assert_eq!(triaged.drifted[0].anchors[1].severity, Severity::Minor);
    assert_eq!(triaged.drifted[0].severity, Severity::Minor);
}

// --- parse_triage_response edge cases ---

#[test]
fn parses_response_with_null_symbol() {
    let response = r#"[{"path": "src/Config.java", "symbol": null, "severity": "minor"}]"#;
    let classifications = triage::parse_triage_response(response).unwrap();
    assert_eq!(classifications.len(), 1);
    assert!(classifications[0].symbol.is_none());
    assert_eq!(classifications[0].severity, Severity::Minor);
}

#[test]
fn parses_response_with_all_severity_levels() {
    let response = r#"[
        {"path": "a.java", "symbol": null, "severity": "no_update"},
        {"path": "b.java", "symbol": null, "severity": "minor"},
        {"path": "c.java", "symbol": null, "severity": "major"}
    ]"#;
    let classifications = triage::parse_triage_response(response).unwrap();
    assert_eq!(classifications.len(), 3);
    assert_eq!(classifications[0].severity, Severity::NoUpdate);
    assert_eq!(classifications[1].severity, Severity::Minor);
    assert_eq!(classifications[2].severity, Severity::Major);
}

#[test]
fn parses_response_with_leading_trailing_whitespace() {
    let response =
        "   \n\n  [{\"path\": \"a.java\", \"symbol\": null, \"severity\": \"major\"}]  \n  ";
    let classifications = triage::parse_triage_response(response).unwrap();
    assert_eq!(classifications.len(), 1);
    assert_eq!(classifications[0].severity, Severity::Major);
}

#[test]
fn parses_response_with_bare_code_fences() {
    // No "json" tag, just ```
    let response = "```\n[{\"path\": \"a.java\", \"symbol\": null, \"severity\": \"minor\"}]\n```";
    let classifications = triage::parse_triage_response(response).unwrap();
    assert_eq!(classifications.len(), 1);
    assert_eq!(classifications[0].severity, Severity::Minor);
}

#[test]
fn parses_response_with_multiple_anchors() {
    let response = r#"[
        {"path": "src/Auth.java", "symbol": "Auth#validate", "severity": "major"},
        {"path": "src/Auth.java", "symbol": "Auth#refresh", "severity": "no_update"},
        {"path": "src/Config.java", "symbol": null, "severity": "minor"}
    ]"#;
    let classifications = triage::parse_triage_response(response).unwrap();
    assert_eq!(classifications.len(), 3);
    assert_eq!(classifications[0].path, "src/Auth.java");
    assert_eq!(classifications[0].symbol.as_deref(), Some("Auth#validate"));
    assert_eq!(classifications[2].path, "src/Config.java");
    assert!(classifications[2].symbol.is_none());
}

#[test]
fn parse_response_ignores_extra_fields() {
    let response = r#"[{"path": "a.java", "symbol": null, "severity": "minor", "reasoning": "trivial change", "confidence": 0.95}]"#;
    let classifications = triage::parse_triage_response(response).unwrap();
    assert_eq!(classifications.len(), 1);
    assert_eq!(classifications[0].severity, Severity::Minor);
}

#[test]
fn parse_response_rejects_missing_path() {
    let response = r#"[{"symbol": null, "severity": "minor"}]"#;
    assert!(triage::parse_triage_response(response).is_err());
}

#[test]
fn parse_response_rejects_missing_severity() {
    let response = r#"[{"path": "a.java", "symbol": null}]"#;
    assert!(triage::parse_triage_response(response).is_err());
}

#[test]
fn parse_response_rejects_invalid_severity_value() {
    let response = r#"[{"path": "a.java", "symbol": null, "severity": "critical"}]"#;
    assert!(triage::parse_triage_response(response).is_err());
}

#[test]
fn parse_response_rejects_json_object_instead_of_array() {
    let response = r#"{"path": "a.java", "symbol": null, "severity": "minor"}"#;
    assert!(triage::parse_triage_response(response).is_err());
}

// --- promote_drift_report (provider = "none") ---

#[test]
fn promote_drift_report_sets_all_anchors_to_major() {
    let drift_report = DriftReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc123".to_string(),
        drifted: vec![
            DriftedDoc {
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
            },
            DriftedDoc {
                doc: "billing.md".to_string(),
                doc_repo: "git@example.com:docs.git".to_string(),
                anchors: vec![DriftedAnchor {
                    path: "src/Billing.java".to_string(),
                    symbol: None,
                    provenance: "old3".to_string(),
                    current_sig: "sig:3333333333333333".to_string(),
                    current_commit: "abc123".to_string(),
                    diff_summary: "Changed".to_string(),
                    diff: "+change".to_string(),
                }],
            },
        ],
        clean: vec![],
    };

    let triaged = triage::promote_drift_report(&drift_report);

    assert_eq!(triaged.drifted.len(), 2);
    // All doc-level severities are Major
    assert_eq!(triaged.drifted[0].severity, Severity::Major);
    assert_eq!(triaged.drifted[1].severity, Severity::Major);
    // All anchor-level severities are Major
    assert_eq!(triaged.drifted[0].anchors[0].severity, Severity::Major);
    assert_eq!(triaged.drifted[0].anchors[1].severity, Severity::Major);
    assert_eq!(triaged.drifted[1].anchors[0].severity, Severity::Major);
    // Report-level fields preserved
    assert_eq!(triaged.repo, "test");
    assert_eq!(triaged.git_ref, "main");
    assert_eq!(triaged.commit, "abc123");
}

#[test]
fn promote_drift_report_preserves_anchor_fields() {
    let drift_report = DriftReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc123".to_string(),
        drifted: vec![DriftedDoc {
            doc: "auth.md".to_string(),
            doc_repo: "git@example.com:docs.git".to_string(),
            anchors: vec![DriftedAnchor {
                path: "src/Auth.java".to_string(),
                symbol: Some("Auth#validate".to_string()),
                provenance: "sig:oldoldoldoldold1".to_string(),
                current_sig: "sig:newnewnewnewnew1".to_string(),
                current_commit: "def456".to_string(),
                diff_summary: "Added scopes param".to_string(),
                diff: "+public boolean validate(String t, List<String> scopes)".to_string(),
            }],
        }],
        clean: vec![],
    };

    let triaged = triage::promote_drift_report(&drift_report);
    let anchor = &triaged.drifted[0].anchors[0];

    assert_eq!(anchor.path, "src/Auth.java");
    assert_eq!(anchor.symbol.as_deref(), Some("Auth#validate"));
    assert_eq!(anchor.provenance, "sig:oldoldoldoldold1");
    assert_eq!(anchor.current_sig, "sig:newnewnewnewnew1");
    assert_eq!(
        anchor.diff,
        "+public boolean validate(String t, List<String> scopes)"
    );
    assert_eq!(triaged.drifted[0].doc, "auth.md");
    assert_eq!(triaged.drifted[0].doc_repo, "git@example.com:docs.git");
}

#[test]
fn promote_drift_report_empty_report() {
    let drift_report = DriftReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc123".to_string(),
        drifted: vec![],
        clean: vec![],
    };

    let triaged = triage::promote_drift_report(&drift_report);
    assert!(triaged.drifted.is_empty());
    assert_eq!(triaged.repo, "test");
}

// --- apply_classifications: current_sig passthrough ---

#[test]
fn apply_classifications_preserves_current_sig() {
    let drift_report = DriftReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc123".to_string(),
        drifted: vec![DriftedDoc {
            doc: "auth.md".to_string(),
            doc_repo: "git@example.com:docs.git".to_string(),
            anchors: vec![DriftedAnchor {
                path: "src/Auth.java".to_string(),
                symbol: Some("Auth#validate".to_string()),
                provenance: "old_prov".to_string(),
                current_sig: "sig:feedface12345678".to_string(),
                current_commit: "abc123".to_string(),
                diff_summary: "Changed".to_string(),
                diff: "+change".to_string(),
            }],
        }],
        clean: vec![],
    };

    let classifications = vec![triage::AnchorClassification {
        path: "src/Auth.java".to_string(),
        symbol: Some("Auth#validate".to_string()),
        severity: Severity::Minor,
    }];

    let triaged = triage::apply_classifications(&drift_report, &classifications, "test");
    assert_eq!(
        triaged.drifted[0].anchors[0].current_sig,
        "sig:feedface12345678"
    );
}

// --- apply_classifications: multiple drifted docs ---

#[test]
fn apply_classifications_handles_multiple_docs() {
    let drift_report = DriftReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc123".to_string(),
        drifted: vec![
            DriftedDoc {
                doc: "auth.md".to_string(),
                doc_repo: "git@example.com:docs.git".to_string(),
                anchors: vec![DriftedAnchor {
                    path: "src/Auth.java".to_string(),
                    symbol: None,
                    provenance: "old".to_string(),
                    current_sig: "sig:aaaa".to_string(),
                    current_commit: "abc123".to_string(),
                    diff_summary: "Changed".to_string(),
                    diff: "+a".to_string(),
                }],
            },
            DriftedDoc {
                doc: "billing.md".to_string(),
                doc_repo: "git@example.com:docs.git".to_string(),
                anchors: vec![DriftedAnchor {
                    path: "src/Billing.java".to_string(),
                    symbol: Some("Billing#charge".to_string()),
                    provenance: "old".to_string(),
                    current_sig: "sig:bbbb".to_string(),
                    current_commit: "abc123".to_string(),
                    diff_summary: "Changed".to_string(),
                    diff: "+b".to_string(),
                }],
            },
        ],
        clean: vec![],
    };

    let classifications = vec![
        triage::AnchorClassification {
            path: "src/Auth.java".to_string(),
            symbol: None,
            severity: Severity::Minor,
        },
        triage::AnchorClassification {
            path: "src/Billing.java".to_string(),
            symbol: Some("Billing#charge".to_string()),
            severity: Severity::Major,
        },
    ];

    let triaged = triage::apply_classifications(&drift_report, &classifications, "test");
    assert_eq!(triaged.drifted.len(), 2);
    assert_eq!(triaged.drifted[0].severity, Severity::Minor);
    assert_eq!(triaged.drifted[1].severity, Severity::Major);
}
