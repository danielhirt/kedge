use anyhow::{bail, Context, Result};

/// Call an AI provider to classify drift severity.
///
/// Supported providers:
/// - `"anthropic"` — Anthropic Messages API (requires ANTHROPIC_API_KEY)
/// - `"bedrock"`   — stub, not yet implemented
pub async fn classify(provider: &str, model: &str, prompt: &str) -> Result<String> {
    match provider {
        "anthropic" => classify_anthropic(model, prompt).await,
        "bedrock" => bail!("bedrock provider: not yet implemented"),
        other => bail!("unknown provider: {}", other),
    }
}

async fn classify_anthropic(model: &str, prompt: &str) -> Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY environment variable is not set")?;

    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "messages": [
            {"role": "user", "content": prompt}
        ]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("failed to send request to Anthropic API")?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .context("failed to read Anthropic API response")?;

    if !status.is_success() {
        bail!("Anthropic API error ({}): {}", status, response_text);
    }

    let json: serde_json::Value = serde_json::from_str(&response_text)
        .context("failed to parse Anthropic API response as JSON")?;

    let text = json["content"][0]["text"]
        .as_str()
        .context("unexpected Anthropic API response shape: missing content[0].text")?
        .to_string();

    Ok(text)
}
