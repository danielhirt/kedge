use anyhow::{bail, Context, Result};

use crate::config::TriageConfig;

const DEFAULT_ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_OPENAI_URL: &str = "https://api.openai.com/v1/chat/completions";

fn http_client(timeout_secs: u64) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .context("failed to build HTTP client")
}

pub async fn classify(config: &TriageConfig, prompt: &str) -> Result<String> {
    match config.provider.as_str() {
        "anthropic" => classify_anthropic(config, prompt).await,
        "openai" => classify_openai(config, prompt).await,
        "command" => classify_command(config, prompt),
        other => bail!("unknown triage provider: {}", other),
    }
}

fn resolve_api_key(config: &TriageConfig, default_env: &str) -> Result<String> {
    let env_var = if config.api_key_env.is_empty() {
        default_env
    } else {
        &config.api_key_env
    };
    std::env::var(env_var).with_context(|| format!("{} environment variable is not set", env_var))
}

fn truncate_error(status: reqwest::StatusCode, body: String) -> anyhow::Error {
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        anyhow::anyhow!(
            "API authentication failed ({}). Check your API key.",
            status
        )
    } else {
        let truncated = if body.len() > 500 {
            format!("{}...[truncated]", &body[..500])
        } else {
            body
        };
        anyhow::anyhow!("API error ({}): {}", status, truncated)
    }
}

async fn classify_anthropic(config: &TriageConfig, prompt: &str) -> Result<String> {
    let api_key = resolve_api_key(config, "ANTHROPIC_API_KEY")?;
    let url = if config.api_url.is_empty() {
        DEFAULT_ANTHROPIC_URL
    } else {
        &config.api_url
    };

    let model = if config.model.is_empty() {
        "claude-haiku-4-5-20251001"
    } else {
        &config.model
    };

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "messages": [{"role": "user", "content": prompt}]
    });

    let response = http_client(config.triage_timeout)?
        .post(url)
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
        return Err(truncate_error(status, response_text));
    }

    let json: serde_json::Value = serde_json::from_str(&response_text)
        .context("failed to parse Anthropic API response as JSON")?;

    json["content"][0]["text"]
        .as_str()
        .context("unexpected Anthropic API response shape: missing content[0].text")
        .map(|s| s.to_string())
}

async fn classify_openai(config: &TriageConfig, prompt: &str) -> Result<String> {
    let api_key = resolve_api_key(config, "OPENAI_API_KEY")?;
    let base_url = if config.api_url.is_empty() {
        DEFAULT_OPENAI_URL
    } else {
        &config.api_url
    };

    // Append /chat/completions if the URL looks like a base URL without a path
    let url = if base_url.ends_with("/v1") || base_url.ends_with("/v1/") {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    } else {
        base_url.to_string()
    };

    if config.model.is_empty() {
        bail!("model must be set for openai provider");
    }

    let body = serde_json::json!({
        "model": &config.model,
        "messages": [{"role": "user", "content": prompt}]
    });

    let response = http_client(config.triage_timeout)?
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .with_context(|| format!("failed to send request to {}", url))?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .context("failed to read OpenAI API response")?;

    if !status.is_success() {
        return Err(truncate_error(status, response_text));
    }

    let json: serde_json::Value = serde_json::from_str(&response_text)
        .context("failed to parse OpenAI API response as JSON")?;

    json["choices"][0]["message"]["content"]
        .as_str()
        .context("unexpected OpenAI API response shape: missing choices[0].message.content")
        .map(|s| s.to_string())
}

fn classify_command(config: &TriageConfig, prompt: &str) -> Result<String> {
    if config.triage_command.is_empty() {
        bail!("triage_command must be set when provider = \"command\"");
    }
    crate::remediation::agent::invoke_agent(
        &config.triage_command,
        prompt,
        config.triage_timeout,
        &config.triage_env,
    )
}
