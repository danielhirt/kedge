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

/// Resolve the docs directory path. Prefers STEER_DOCS_PATH env var,
/// otherwise clones from config's repos.docs via repo_cache.
fn resolve_docs_path(config: &steer::config::Config) -> anyhow::Result<std::path::PathBuf> {
    if let Ok(p) = std::env::var("STEER_DOCS_PATH") {
        return Ok(std::path::PathBuf::from(p));
    }
    let doc_repo = config.repos.docs.first()
        .context("no docs repo configured in steer.toml")?;
    let cached = steer::install::repo_cache::get_or_clone(&doc_repo.url, &doc_repo.git_ref)
        .with_context(|| format!("failed to clone docs repo {}", doc_repo.url))?;
    if doc_repo.path.is_empty() {
        Ok(cached)
    } else {
        Ok(cached.join(&doc_repo.path))
    }
}

/// Compute content-addressed sigs for all anchors in a doc file and update
/// provenance in a single read/write. Returns number of anchors stamped.
fn stamp_anchors(
    code_repo_path: &std::path::Path,
    doc_file: &std::path::Path,
    anchors: &[steer::models::Anchor],
) -> usize {
    let mut updates: Vec<(&str, Option<&str>, String)> = Vec::new();

    for anchor in anchors {
        let code_file = code_repo_path.join(&anchor.path);
        match std::fs::read_to_string(&code_file) {
            Ok(content) => {
                let sig = steer::detection::fingerprint::compute_sig(
                    &content,
                    &anchor.path,
                    anchor.symbol.as_deref(),
                );
                updates.push((&anchor.path, anchor.symbol.as_deref(), sig));
            }
            Err(e) => {
                eprintln!("warning: cannot read {}: {}", anchor.path, e);
            }
        }
    }

    let stamped = updates.len();
    if !updates.is_empty() {
        let batch: Vec<(&str, Option<&str>, &str)> = updates
            .iter()
            .map(|(p, s, sig)| (*p, *s, sig.as_str()))
            .collect();
        if let Err(e) = steer::frontmatter::update_provenance_batch(doc_file, &batch) {
            eprintln!("warning: failed to update provenance in {}: {}", doc_file.display(), e);
            return 0;
        }
    }
    stamped
}

/// Read doc contents from disk for each drifted doc in a drift report.
fn collect_doc_contents(
    docs_path: &std::path::Path,
    report: &steer::models::DriftReport,
) -> std::collections::HashMap<String, String> {
    let mut contents = std::collections::HashMap::new();
    for drifted_doc in &report.drifted {
        let doc_path = docs_path.join(&drifted_doc.doc);
        if let Ok(content) = std::fs::read_to_string(&doc_path) {
            contents.insert(drifted_doc.doc.clone(), content);
        }
    }
    contents
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => {
            if cli.config.exists() {
                eprintln!(
                    "{} already exists — skipping.",
                    cli.config.display()
                );
            } else {
                let template = r#"[detection]
languages = ["rust", "python", "typescript"]
fallback = "content-hash"

[triage]
provider = "anthropic"
model = "claude-opus-4-5"

[remediation]
agent_command = "your-agent-command"
auto_merge_severities = ["no_update"]

[repos]
docs = [
  { url = "https://github.com/your-org/docs.git", path = ".", ref = "main" },
]

[[agents]]
name = "claude"
global_steering = "~/.claude/steering"
workspace_steering = ".claude/steering"
agents_file = "CLAUDE.md"
skill_dir = ""
"#;
                std::fs::write(&cli.config, template)
                    .with_context(|| format!("failed to write {}", cli.config.display()))?;
                println!("Created {} — edit it with your repo URLs and agent config.", cli.config.display());
            }
        }
        Command::Link { files } => {
            let code_repo_path = std::env::current_dir()?;

            let paths_to_process: Vec<std::path::PathBuf> = if files.is_empty() {
                let pattern = format!("{}/**/*.md", code_repo_path.display());
                glob::glob(&pattern)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .collect()
            } else {
                files
            };

            for file_path in &paths_to_process {
                let doc = match steer::frontmatter::parse_doc_file(file_path, "") {
                    Some(d) => d,
                    None => continue,
                };

                let n = doc.frontmatter.anchors.len();
                let stamped = stamp_anchors(&code_repo_path, file_path, &doc.frontmatter.anchors);
                println!("Linked {} ({}/{} anchors stamped)", file_path.display(), stamped, n);
            }
        }
        Command::Check { report } => {
            let code_repo_path = std::env::current_dir()?;
            let repo_name = code_repo_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "unknown".to_string());
            let code_repo_url = std::env::var("STEER_CODE_REPO_URL").unwrap_or_else(|_| {
                format!("file://{}", code_repo_path.display())
            });

            let docs_path = if let Ok(p) = std::env::var("STEER_DOCS_PATH") {
                std::path::PathBuf::from(p)
            } else {
                let config = steer::config::Config::from_file(&cli.config)
                    .context("failed to load steer.toml — run `steer init` first")?;
                resolve_docs_path(&config)?
            };

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

            let config = steer::config::Config::from_file(&cli.config).ok();
            let docs_path = match &config {
                Some(cfg) => resolve_docs_path(cfg)?,
                None => std::env::var("STEER_DOCS_PATH")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| std::path::PathBuf::from("docs")),
            };

            let doc_contents = collect_doc_contents(&docs_path, &drift_report);

            let provider = config.as_ref()
                .map(|c| c.triage.provider.clone())
                .unwrap_or_else(|| "anthropic".to_string());
            let model = config.as_ref()
                .map(|c| c.triage.model.clone())
                .unwrap_or_else(|| "claude-haiku-4-5-20251001".to_string());

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

            let docs_path = resolve_docs_path(&config)?;

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
            let doc_contents = collect_doc_contents(&docs_path, &drift_report);

            let rt = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
            let triaged = rt.block_on(steer::triage::triage_drift_report(
                &drift_report,
                &config.triage.provider,
                &config.triage.model,
                &doc_contents,
            ))?;

            // Partition
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

            let provenance_advanced: Vec<steer::models::ProvenanceSynced> = to_sync
                .iter()
                .map(|doc| {
                    let doc_file = docs_path.join(&doc.doc);
                    // Convert TriagedAnchors to Anchors for stamp_anchors
                    let anchors: Vec<steer::models::Anchor> = doc.anchors.iter().map(|a| {
                        steer::models::Anchor {
                            repo: String::new(),
                            path: a.path.clone(),
                            symbol: a.symbol.clone(),
                            provenance: a.provenance.clone(),
                        }
                    }).collect();
                    let synced = stamp_anchors(&code_repo_path, &doc_file, &anchors);
                    steer::models::ProvenanceSynced {
                        doc: doc.doc.clone(),
                        anchors_synced: synced,
                        reason: "no_update — code changes did not affect documentation accuracy".to_string(),
                    }
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
            let docs_path_str =
                std::env::var("STEER_DOCS_PATH").unwrap_or_else(|_| "docs".to_string());
            let docs_path = std::path::PathBuf::from(&docs_path_str);

            let cwd = std::env::current_dir()?;
            let code_repo_url = std::env::var("STEER_CODE_REPO_URL")
                .unwrap_or_else(|_| format!("file://{}", cwd.display()));

            let docs = steer::frontmatter::scan_docs(&docs_path, &code_repo_url, None);

            if docs.is_empty() {
                println!("No docs with steer frontmatter found in {}", docs_path.display());
            } else {
                for doc in &docs {
                    let group = doc.frontmatter.group.as_deref().unwrap_or("(no group)");
                    println!("{} [group: {}]", doc.path, group);
                    for anchor in &doc.frontmatter.anchors {
                        let symbol_part = anchor
                            .symbol
                            .as_deref()
                            .map(|s| format!("#{}", s))
                            .unwrap_or_default();
                        println!(
                            "  anchor: {}{}  provenance: {}",
                            anchor.path, symbol_part, anchor.provenance
                        );
                    }
                }
            }
        }
        Command::Sync { files } => {
            let code_repo_path = std::env::current_dir()?;

            let paths_to_process: Vec<std::path::PathBuf> = if files.is_empty() {
                let pattern = format!("{}/**/*.md", code_repo_path.display());
                glob::glob(&pattern)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .collect()
            } else {
                files
            };

            let mut total_anchors = 0usize;
            for file_path in &paths_to_process {
                let doc = match steer::frontmatter::parse_doc_file(file_path, "") {
                    Some(d) => d,
                    None => continue,
                };
                total_anchors += stamp_anchors(&code_repo_path, file_path, &doc.frontmatter.anchors);
            }

            println!("Synced {} anchors with content-addressed provenance", total_anchors);
        }
        Command::Install { group, agent, link, workspace, check } => {
            let config = steer::config::Config::from_file(&cli.config)
                .context("failed to load steer.toml — run `steer init` first")?;

            // Determine install mode: explicit flags take priority, then CI env detection
            let is_ci = std::env::var("CI").is_ok()
                || std::env::var("GITHUB_ACTIONS").is_ok()
                || std::env::var("GITLAB_CI").is_ok();
            let use_workspace = workspace || (is_ci && !link);

            // Collect agent platforms to install to
            let platforms: Vec<&steer::config::AgentPlatform> = match &agent {
                Some(name) => {
                    if let Some(p) = config.find_agent(name) {
                        vec![p]
                    } else {
                        anyhow::bail!("unknown agent platform: {}", name);
                    }
                }
                None => config.agents.iter().collect(),
            };

            // Process each configured doc repo
            for doc_repo in &config.repos.docs {
                // --check: skip if already up to date
                if check {
                    match steer::install::repo_cache::is_up_to_date(&doc_repo.url, &doc_repo.git_ref) {
                        Ok(true) => {
                            eprintln!("steering docs are up to date ({})", doc_repo.url);
                            continue;
                        }
                        Ok(false) => {}
                        Err(e) => eprintln!("warning: could not check staleness: {}", e),
                    }
                }

                // Clone or update the cached repo
                let source_dir = steer::install::repo_cache::get_or_clone(&doc_repo.url, &doc_repo.git_ref)
                    .with_context(|| format!("failed to get/clone repo {}", doc_repo.url))?;

                // Install to each agent platform
                for platform in &platforms {
                    let target_dir = if use_workspace {
                        let workspace_steering = shellexpand::tilde(&platform.workspace_steering).into_owned();
                        std::path::PathBuf::from(workspace_steering)
                    } else {
                        let global_steering = shellexpand::tilde(&platform.global_steering).into_owned();
                        std::path::PathBuf::from(global_steering)
                    };

                    let agents_file = if platform.agents_file.is_empty() {
                        None
                    } else {
                        Some(platform.agents_file.as_str())
                    };

                    let skill_dir = if platform.skill_dir.is_empty() {
                        None
                    } else {
                        let expanded = shellexpand::tilde(&platform.skill_dir).into_owned();
                        Some(std::path::PathBuf::from(expanded))
                    };

                    if use_workspace {
                        steer::install::install_to_workspace(
                            &source_dir,
                            &target_dir,
                            group.as_deref(),
                            agents_file,
                            skill_dir.as_deref(),
                        ).with_context(|| format!("install_to_workspace failed for {}", platform.name))?;

                        // Add to .git/info/exclude
                        if let Ok(cwd) = std::env::current_dir() {
                            let rel = target_dir.strip_prefix(&cwd).unwrap_or(&target_dir);
                            let _ = steer::install::add_to_git_exclude(&cwd, &rel.to_string_lossy());
                        }
                    } else {
                        steer::install::install_as_links(
                            &source_dir,
                            &target_dir,
                            group.as_deref(),
                            agents_file,
                            skill_dir.as_deref(),
                        ).with_context(|| format!("install_as_links failed for {}", platform.name))?;
                    }

                    eprintln!(
                        "installed steering to {} ({})",
                        target_dir.display(),
                        platform.name
                    );
                }
            }
        }
    }

    Ok(())
}
