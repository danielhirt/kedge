use kedge::models::{AgentResponse, Severity};

#[test]
fn severity_ordering_no_update_lt_minor_lt_major() {
    assert!(Severity::NoUpdate < Severity::Minor);
    assert!(Severity::Minor < Severity::Major);
    assert!(Severity::NoUpdate < Severity::Major);
}

#[test]
fn agent_response_deserialize_mr_url_only() {
    let json = r#"{"mr_url": "https://gitlab.com/mr/1"}"#;
    let resp: AgentResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.mr_url.as_deref(), Some("https://gitlab.com/mr/1"));
    assert!(resp.mr_urls.is_none());
}

#[test]
fn agent_response_deserialize_mr_urls_array() {
    let json = r#"{"mr_urls": ["https://gitlab.com/mr/1", "https://gitlab.com/mr/2"]}"#;
    let resp: AgentResponse = serde_json::from_str(json).unwrap();
    assert!(resp.mr_url.is_none());
    let urls = resp.mr_urls.unwrap();
    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0], "https://gitlab.com/mr/1");
}

#[test]
fn agent_response_deserialize_empty_json_all_defaults() {
    let json = "{}";
    let resp: AgentResponse = serde_json::from_str(json).unwrap();
    assert!(resp.mr_url.is_none());
    assert!(resp.mr_urls.is_none());
    assert_eq!(resp.status, "");
}
