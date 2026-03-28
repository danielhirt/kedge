use kedge::output::{parse_agent_output, scrape_urls};

// --- parse_agent_output ---

#[test]
fn parse_agent_output_json_with_mr_url() {
    let json = r#"{"mr_url": "https://gitlab.com/org/repo/-/merge_requests/42", "status": "ok"}"#;
    let (single, all) = parse_agent_output(json);
    assert_eq!(
        single,
        Some("https://gitlab.com/org/repo/-/merge_requests/42".to_string())
    );
    assert_eq!(all, vec!["https://gitlab.com/org/repo/-/merge_requests/42"]);
}

#[test]
fn parse_agent_output_json_with_mr_urls_array() {
    let json =
        r#"{"mr_urls": ["https://gitlab.com/mr/1", "https://gitlab.com/mr/2"], "status": "ok"}"#;
    let (single, all) = parse_agent_output(json);
    assert_eq!(single, None);
    assert_eq!(
        all,
        vec!["https://gitlab.com/mr/1", "https://gitlab.com/mr/2"]
    );
}

#[test]
fn parse_agent_output_json_with_both_deduplicates() {
    let json = r#"{
        "mr_url": "https://gitlab.com/mr/1",
        "mr_urls": ["https://gitlab.com/mr/1", "https://gitlab.com/mr/2"],
        "status": "ok"
    }"#;
    let (single, all) = parse_agent_output(json);
    assert_eq!(single, Some("https://gitlab.com/mr/1".to_string()));
    // mr_url already present in mr_urls, so no duplicate inserted
    assert_eq!(
        all,
        vec!["https://gitlab.com/mr/1", "https://gitlab.com/mr/2"]
    );
}

#[test]
fn parse_agent_output_invalid_json_falls_back_to_scraping() {
    let output = "Agent finished. See https://gitlab.com/mr/99 for details.";
    let (single, all) = parse_agent_output(output);
    assert_eq!(single, Some("https://gitlab.com/mr/99".to_string()));
    assert_eq!(all, vec!["https://gitlab.com/mr/99"]);
}

#[test]
fn parse_agent_output_empty_string() {
    let (single, all) = parse_agent_output("");
    assert_eq!(single, None);
    assert!(all.is_empty());
}

#[test]
fn parse_agent_output_json_empty_status_no_urls() {
    let json = r#"{"status": ""}"#;
    let (single, all) = parse_agent_output(json);
    assert_eq!(single, None);
    assert!(all.is_empty());
}

// --- scrape_urls ---

#[test]
fn scrape_urls_extracts_https() {
    let text = "Check out https://example.com/path and some other text";
    let urls = scrape_urls(text);
    assert_eq!(urls, vec!["https://example.com/path"]);
}

#[test]
fn scrape_urls_extracts_http() {
    let text = "Visit http://legacy.example.com/page for info";
    let urls = scrape_urls(text);
    assert_eq!(urls, vec!["http://legacy.example.com/page"]);
}

#[test]
fn scrape_urls_strips_trailing_punctuation() {
    let text = concat!(
        "comma: https://a.com/1, ",
        "period: https://a.com/2. ",
        "bracket: https://a.com/3] ",
        "paren: https://a.com/4) ",
        "semicolon: https://a.com/5; ",
        "quote: https://a.com/6\"",
    );
    let urls = scrape_urls(text);
    assert_eq!(
        urls,
        vec![
            "https://a.com/1",
            "https://a.com/2",
            "https://a.com/3",
            "https://a.com/4",
            "https://a.com/5",
            "https://a.com/6",
        ]
    );
}

#[test]
fn scrape_urls_extracts_multiple() {
    let text = "See https://one.com and also https://two.com/path for details";
    let urls = scrape_urls(text);
    assert_eq!(urls, vec!["https://one.com", "https://two.com/path"]);
}

#[test]
fn scrape_urls_empty_string() {
    let urls = scrape_urls("");
    assert!(urls.is_empty());
}

#[test]
fn scrape_urls_no_urls_in_text() {
    let urls = scrape_urls("just plain text with no links at all");
    assert!(urls.is_empty());
}
