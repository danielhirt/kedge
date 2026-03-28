mod cli;

use anyhow::Context;
use clap::Parser;
use cli::{Cli, Command};

/// Scan agent stdout for a URL (https://...) and return the first one found.
fn extract_mr_url(output: &str) -> Option<String> {
    for word in output.split_whitespace() {
        if word.starts_with("https://") || word.starts_with("http://") {
            return Some(word.trim_end_matches([',', '.', ';', ')', ']', '"', '\'']).to_string());
        }
    }
    None
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => {
            println!("steer init: not yet implemented");
        }
        Command::Link { files } => {
            println!("steer link: not yet implemented ({} files)", files.len());
        }
        Command::Check { report } => {
            // Read config to get the repo name (best-effort; fall back to cwd name).
            let config = steer::config::Config::from_file(&cli.config).ok();
            let repo_name = config
                .as_ref()
                .and_then(|_| {
                    std::env::current_dir()
                        .ok()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                })
                .unwrap_or_else(|| "unknown".to_string());

            // Resolve code repo path (current working directory).
            let code_repo_path = std::env::current_dir()?;

            // Docs location comes from the STEER_DOCS_PATH env var.
            let docs_path_str = std::env::var("STEER_DOCS_PATH").unwrap_or_else(|_| "docs".to_string());
            let docs_path = std::path::PathBuf::from(&docs_path_str);

            // Derive the repo URL: prefer file:// for local paths, otherwise use
            // the STEER_CODE_REPO_URL env var if set.
            let code_repo_url = std::env::var("STEER_CODE_REPO_URL").unwrap_or_else(|_| {
                format!("file://{}", code_repo_path.display())
            });

            let drift_report = steer::detection::detect_drift(
                &code_repo_path,
                &docs_path,
                &code_repo_url,
                &repo_name,
            )?;

            let json = serde_json::to_string_pretty(&drift_report)?;

            match &report {
                Some(path) => {
                    std::fs::write(path, &json)?;
                    eprintln!("Report written to {}", path.display());
                }
                None => println!("{}", json),
            }

            if !drift_report.drifted.is_empty() {
                std::process::exit(1);
            }
        }
        Command::Triage { report } => {
            // Read drift report from --report file or stdin.
            let json_input = match &report {
                Some(path) => std::fs::read_to_string(path)
                    .with_context(|| format!("failed to read report file {}", path.display()))?,
                None => {
                    use std::io::Read;
                    let mut buf = String::new();
                    std::io::stdin()
                        .read_to_string(&mut buf)
                        .context("failed to read drift report from stdin")?;
                    buf
                }
            };

            let drift_report: steer::models::DriftReport = serde_json::from_str(&json_input)
                .context("failed to parse drift report JSON")?;

            // Read doc contents for each drifted doc.
            let docs_path_str =
                std::env::var("STEER_DOCS_PATH").unwrap_or_else(|_| "docs".to_string());
            let docs_path = std::path::PathBuf::from(&docs_path_str);

            let mut doc_contents: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for drifted_doc in &drift_report.drifted {
                let doc_path = docs_path.join(&drifted_doc.doc);
                match std::fs::read_to_string(&doc_path) {
                    Ok(content) => {
                        doc_contents.insert(drifted_doc.doc.clone(), content);
                    }
                    Err(e) => {
                        eprintln!(
                            "warning: could not read doc {}: {}",
                            doc_path.display(),
                            e
                        );
                    }
                }
            }

            let provider =
                std::env::var("STEER_AI_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
            let model = std::env::var("STEER_AI_MODEL")
                .unwrap_or_else(|_| "claude-opus-4-5".to_string());

            let rt = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
            let triaged = rt.block_on(steer::triage::triage_drift_report(
                &drift_report,
                &provider,
                &model,
                &doc_contents,
            ))?;

            println!("{}", serde_json::to_string_pretty(&triaged)?);
        }
        Command::Update { report } => {
            let config = steer::config::Config::from_file(&cli.config)
                .context("failed to load steer.toml — run `steer init` first")?;

            let code_repo_path = std::env::current_dir()?;
            let repo_name = code_repo_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "unknown".to_string());
            let code_repo_url = std::env::var("STEER_CODE_REPO_URL").unwrap_or_else(|_| {
                format!("file://{}", code_repo_path.display())
            });
            let docs_path_str =
                std::env::var("STEER_DOCS_PATH").unwrap_or_else(|_| "docs".to_string());
            let docs_path = std::path::PathBuf::from(&docs_path_str);

            // 1. Detection
            let drift_report = steer::detection::detect_drift(
                &code_repo_path,
                &docs_path,
                &code_repo_url,
                &repo_name,
            )?;

            if let Some(path) = &report {
                let json = serde_json::to_string_pretty(&drift_report)?;
                std::fs::write(path, &json)?;
                eprintln!("Drift report written to {}", path.display());
            }

            // 2. Early exit if clean
            if drift_report.drifted.is_empty() {
                println!("No drift detected.");
                return Ok(());
            }

            let current_commit = drift_report.commit.clone();

            // 3. Triage
            let mut doc_contents: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for drifted_doc in &drift_report.drifted {
                let doc_path = docs_path.join(&drifted_doc.doc);
                if let Ok(content) = std::fs::read_to_string(&doc_path) {
                    doc_contents.insert(drifted_doc.doc.clone(), content);
                }
            }

            let provider = config.triage.provider.clone();
            let model = config.triage.model.clone();

            let rt = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
            let triaged = rt.block_on(steer::triage::triage_drift_report(
                &drift_report,
                &provider,
                &model,
                &doc_contents,
            ))?;

            // 4. Partition
            let (to_remediate, to_sync) =
                steer::remediation::partition_by_action(&triaged);

            // 5. Remediate each doc that needs agent invocation
            let mut remediated: Vec<steer::models::RemediatedDoc> = Vec::new();
            let mut errors: Vec<String> = Vec::new();

            for doc in &to_remediate {
                let auto_merge = steer::remediation::should_auto_merge(
                    doc.severity,
                    &config.remediation.auto_merge_severities,
                );
                let payload =
                    steer::remediation::build_agent_payload(doc, &current_commit, auto_merge);
                let payload_json = serde_json::to_string(&payload)
                    .context("failed to serialize agent payload")?;

                match steer::remediation::agent::invoke_agent(
                    &config.remediation.agent_command,
                    &payload_json,
                ) {
                    Ok(output) => {
                        let mr_url = extract_mr_url(&output);
                        remediated.push(steer::models::RemediatedDoc {
                            doc: doc.doc.clone(),
                            repo: doc.doc_repo.clone(),
                            mr_url,
                            severity: doc.severity,
                            auto_merged: auto_merge,
                        });
                    }
                    Err(e) => {
                        errors.push(format!("{}: {}", doc.doc, e));
                    }
                }
            }

            // 6. Provenance-sync entries
            let provenance_advanced: Vec<steer::models::ProvenanceSynced> = to_sync
                .iter()
                .map(|doc| steer::models::ProvenanceSynced {
                    doc: doc.doc.clone(),
                    anchors_synced: doc.anchors.len(),
                    reason: "all anchors no_update".to_string(),
                })
                .collect();

            // 7. Output summary
            let summary = steer::models::RemediationSummary {
                remediated,
                provenance_advanced,
                errors,
            };
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Status => {
            println!("steer status: not yet implemented");
        }
        Command::Sync { files } => {
            println!("steer sync: not yet implemented ({} files)", files.len());
        }
        Command::Install { group, agent, .. } => {
            println!(
                "steer install: not yet implemented (group: {:?}, agent: {:?})",
                group, agent
            );
        }
    }

    Ok(())
}
