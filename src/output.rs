use crate::models::AgentResponse;

pub fn parse_agent_output(output: &str) -> (Option<String>, Vec<String>) {
    if let Ok(resp) = serde_json::from_str::<AgentResponse>(output.trim()) {
        let mut urls: Vec<String> = resp.mr_urls.unwrap_or_default();
        if let Some(url) = &resp.mr_url {
            if !urls.contains(url) {
                urls.insert(0, url.clone());
            }
        }
        (resp.mr_url, urls)
    } else {
        let urls = scrape_urls(output);
        let first = urls.first().cloned();
        (first, urls)
    }
}

pub fn scrape_urls(output: &str) -> Vec<String> {
    output
        .split_whitespace()
        .filter(|w| w.starts_with("https://") || w.starts_with("http://"))
        .map(|w| {
            w.trim_end_matches([',', '.', ';', ')', ']', '"', '\''])
                .to_string()
        })
        .collect()
}
