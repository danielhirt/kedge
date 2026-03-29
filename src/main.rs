mod cli;

use anyhow::Context;
use clap::Parser;
use cli::{Cli, Command};

fn code_repo_url(cwd: &std::path::Path) -> String {
    std::env::var("KEDGE_CODE_REPO_URL").unwrap_or_else(|_| format!("file://{}", cwd.display()))
}

fn repo_name(cwd: &std::path::Path) -> String {
    cwd.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string())
}

fn resolve_md_files(
    files: Vec<std::path::PathBuf>,
    base: &std::path::Path,
) -> Vec<std::path::PathBuf> {
    if files.is_empty() {
        let pattern = format!("{}/**/*.md", base.display());
        glob::glob(&pattern)
            .into_iter()
            .flatten()
            .flatten()
            .collect()
    } else {
        files
    }
}

/// Resolved docs source with enough info to pass to detect_drift and resolve paths.
struct DocsSource {
    /// Directory to scan for .md files (may be a subdirectory of repo_root).
    scan_dir: std::path::PathBuf,
    /// Git URL of the docs repository (used in agent payloads).
    repo_url: String,
    /// Root of the cloned docs repo (for computing repo-relative paths).
    repo_root: std::path::PathBuf,
}

fn resolve_docs_source(config: &kedge::config::Config) -> anyhow::Result<DocsSource> {
    if let Ok(p) = std::env::var("KEDGE_DOCS_PATH") {
        let scan_dir = std::path::PathBuf::from(&p);
        return Ok(DocsSource {
            repo_url: std::env::var("KEDGE_DOCS_REPO_URL")
                .unwrap_or_else(|_| code_repo_url(&std::env::current_dir().unwrap_or_default())),
            repo_root: scan_dir.clone(),
            scan_dir,
        });
    }
    let doc_repo = config
        .repos
        .docs
        .first()
        .context("no docs repo configured in kedge.toml")?;
    let cached = kedge::install::repo_cache::get_or_clone(
        &doc_repo.url,
        &doc_repo.git_ref,
        config.repos.git_timeout,
        &doc_repo.remote_name,
    )
    .with_context(|| {
        format!(
            "failed to clone docs repo {}",
            kedge::safety::sanitize_url(&doc_repo.url)
        )
    })?;
    let scan_dir = if doc_repo.path.is_empty() {
        cached.clone()
    } else {
        let resolved = cached.join(&doc_repo.path);
        kedge::safety::validate_path_within(&cached, &resolved)
            .context("docs repo path escapes cache directory")?;
        resolved
    };
    Ok(DocsSource {
        repo_url: doc_repo.url.clone(),
        repo_root: cached,
        scan_dir,
    })
}

fn resolve_all_docs_sources(config: &kedge::config::Config) -> anyhow::Result<Vec<DocsSource>> {
    if let Ok(p) = std::env::var("KEDGE_DOCS_PATH") {
        let scan_dir = std::path::PathBuf::from(&p);
        return Ok(vec![DocsSource {
            repo_url: std::env::var("KEDGE_DOCS_REPO_URL")
                .unwrap_or_else(|_| code_repo_url(&std::env::current_dir().unwrap_or_default())),
            repo_root: scan_dir.clone(),
            scan_dir,
        }]);
    }

    if config.repos.docs.is_empty() {
        anyhow::bail!("no docs repos configured in kedge.toml");
    }

    let mut sources = Vec::new();
    for doc_repo in &config.repos.docs {
        let cached = kedge::install::repo_cache::get_or_clone(
            &doc_repo.url,
            &doc_repo.git_ref,
            config.repos.git_timeout,
            &doc_repo.remote_name,
        )
        .with_context(|| {
            format!(
                "failed to clone docs repo {}",
                kedge::safety::sanitize_url(&doc_repo.url)
            )
        })?;

        let scan_dir = if doc_repo.path.is_empty() {
            cached.clone()
        } else {
            let p = cached.join(&doc_repo.path);
            kedge::safety::validate_path_within(&cached, &p)
                .context("docs repo path escapes cache directory")?;
            p
        };
        sources.push(DocsSource {
            repo_url: doc_repo.url.clone(),
            repo_root: cached,
            scan_dir,
        });
    }
    Ok(sources)
}

fn stamp_anchors(
    code_repo_path: &std::path::Path,
    doc_file: &std::path::Path,
    anchors: &[kedge::models::Anchor],
) -> usize {
    let canon_repo = code_repo_path
        .canonicalize()
        .unwrap_or_else(|_| code_repo_path.to_path_buf());
    let mut updates: Vec<(&str, Option<&str>, String)> = Vec::new();

    for anchor in anchors {
        let code_file = code_repo_path.join(&anchor.path);
        if kedge::safety::validate_path_within_canon(&canon_repo, &code_file).is_err() {
            eprintln!(
                "warning: skipping anchor with path outside repo: {}",
                anchor.path
            );
            continue;
        }
        match std::fs::read_to_string(&code_file) {
            Ok(content) => {
                let sig = kedge::detection::fingerprint::compute_sig(
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
        if let Err(e) = kedge::frontmatter::update_provenance_batch(doc_file, &batch) {
            eprintln!(
                "warning: failed to update provenance in {}: {}",
                doc_file.display(),
                e
            );
            return 0;
        }
    }
    stamped
}

fn collect_doc_contents(
    repo_roots: &[&std::path::Path],
    report: &kedge::models::DriftReport,
) -> std::collections::HashMap<String, String> {
    let mut contents = std::collections::HashMap::new();
    for drifted_doc in &report.drifted {
        for root in repo_roots {
            let doc_path = root.join(&drifted_doc.doc);
            if kedge::safety::validate_path_within(root, &doc_path).is_err() {
                eprintln!(
                    "warning: skipping doc with path outside repo root: {}",
                    drifted_doc.doc
                );
                break;
            }
            if let Ok(content) = std::fs::read_to_string(&doc_path) {
                contents.insert(drifted_doc.doc.clone(), content);
                break;
            }
        }
    }
    contents
}

/// Resolve a repo-relative doc path back to an absolute filesystem path.
fn resolve_doc_file(relative: &str, repo_roots: &[&std::path::Path]) -> Option<std::path::PathBuf> {
    repo_roots.iter().find_map(|root| {
        let full = root.join(relative);
        if kedge::safety::validate_path_within(root, &full).is_err() {
            return None;
        }
        full.exists().then_some(full)
    })
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => {
            if cli.config.exists() {
                eprintln!("{} already exists — skipping.", cli.config.display());
            } else {
                let template = r#"[detection]
# exclude_dirs = [".git", "node_modules", "target", ".venv", "__pycache__", ".tox", "vendor"]

[triage]
provider = "command"                   # "anthropic", "openai", or "command"
triage_command = "your-triage-command"  # required when provider = "command"
# model = "claude-haiku-4-5-20251001"  # required for anthropic/openai providers
# api_url = ""                         # custom API endpoint (default: provider's public API)
# api_key_env = ""                     # env var name for API key (default: ANTHROPIC_API_KEY or OPENAI_API_KEY)
# triage_timeout = 120                 # seconds (default: 120)
# triage_env = { }                     # extra env vars for command provider

[remediation]
agent_command = "your-agent-command"
auto_merge_severities = ["no_update"]
# batch = true        # single agent invocation for all drifted docs
# agent_timeout = 300  # seconds, kills agent process if exceeded
# agent_env = { }      # extra env vars passed to agent process
# agent_instructions = ""  # replaces default agent instructions when set

[repos]
# git_timeout = 300  # seconds for clone/fetch operations (default: 300)
docs = [
  { url = "https://github.com/your-org/docs.git", path = ".", ref = "main" },
]
# remote_name = "origin"  # git remote name to fetch from (default: "origin")

[[agents]]
name = "claude"
global_steering = "~/.claude/docs"
workspace_steering = "docs"
agents_file = "CLAUDE.md"
skill_dir = ".claude/skills/"
"#;
                std::fs::write(&cli.config, template)
                    .with_context(|| format!("failed to write {}", cli.config.display()))?;
                println!(
                    "Created {} — edit it with your repo URLs and agent config.",
                    cli.config.display()
                );
            }
        }
        Command::Link { files } => {
            let code_repo_path = std::env::current_dir()?;
            let paths_to_process = resolve_md_files(files, &code_repo_path);

            for file_path in &paths_to_process {
                let doc = match kedge::frontmatter::parse_doc_file(file_path, "") {
                    Some(d) => d,
                    None => continue,
                };

                let n = doc.frontmatter.anchors.len();
                let stamped = stamp_anchors(&code_repo_path, file_path, &doc.frontmatter.anchors);
                println!(
                    "Linked {} ({}/{} anchors stamped)",
                    file_path.display(),
                    stamped,
                    n
                );
            }
        }
        Command::Check { report } => {
            let code_repo_path = std::env::current_dir()?;
            let rn = repo_name(&code_repo_path);
            let cru = code_repo_url(&code_repo_path);

            let config = kedge::config::Config::from_file(&cli.config)
                .context("failed to load kedge.toml — run `kedge init` first");
            let (docs_sources, exclude_dirs) = if let Ok(p) = std::env::var("KEDGE_DOCS_PATH") {
                let exclude = config
                    .as_ref()
                    .map(|c| c.detection.exclude_dirs.clone())
                    .unwrap_or_else(|_| kedge::config::DetectionConfig::default().exclude_dirs);
                let scan_dir = std::path::PathBuf::from(&p);
                let source = DocsSource {
                    repo_url: std::env::var("KEDGE_DOCS_REPO_URL").unwrap_or_else(|_| cru.clone()),
                    repo_root: scan_dir.clone(),
                    scan_dir,
                };
                (vec![source], exclude)
            } else {
                let cfg = config?;
                let exclude = cfg.detection.exclude_dirs.clone();
                (resolve_all_docs_sources(&cfg)?, exclude)
            };

            let mut merged_report: Option<kedge::models::DriftReport> = None;
            for source in &docs_sources {
                let report = kedge::detection::detect_drift(
                    &code_repo_path,
                    &source.scan_dir,
                    &cru,
                    &source.repo_url,
                    &source.repo_root,
                    &rn,
                    &exclude_dirs,
                )?;
                match &mut merged_report {
                    Some(existing) => {
                        existing.drifted.extend(report.drifted);
                        existing.clean.extend(report.clean);
                    }
                    None => merged_report = Some(report),
                }
            }

            let drift_report = merged_report.context("no drift report generated")?;
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

            let drift_report: kedge::models::DriftReport =
                serde_json::from_str(&json_input).context("failed to parse drift report JSON")?;

            let config = kedge::config::Config::from_file(&cli.config)
                .context("failed to load kedge.toml — triage requires a [triage] config")?;
            let docs_source = resolve_docs_source(&config)?;

            let doc_contents =
                collect_doc_contents(&[docs_source.repo_root.as_path()], &drift_report);

            let rt = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
            let triaged = rt.block_on(kedge::triage::triage_drift_report(
                &drift_report,
                &config.triage,
                &doc_contents,
            ))?;

            println!("{}", serde_json::to_string_pretty(&triaged)?);
        }
        Command::Update { report, no_stamp } => {
            let config = kedge::config::Config::from_file(&cli.config)
                .context("failed to load kedge.toml — run `kedge init` first")?;

            let code_repo_path = std::env::current_dir()?;
            let rn = repo_name(&code_repo_path);
            let cru = code_repo_url(&code_repo_path);

            let docs_sources = resolve_all_docs_sources(&config)?;

            let mut merged_report: Option<kedge::models::DriftReport> = None;
            for source in &docs_sources {
                let sub_report = kedge::detection::detect_drift(
                    &code_repo_path,
                    &source.scan_dir,
                    &cru,
                    &source.repo_url,
                    &source.repo_root,
                    &rn,
                    &config.detection.exclude_dirs,
                )?;
                match &mut merged_report {
                    Some(existing) => {
                        existing.drifted.extend(sub_report.drifted);
                        existing.clean.extend(sub_report.clean);
                    }
                    None => merged_report = Some(sub_report),
                }
            }

            let drift_report = merged_report.context("no drift report generated")?;

            if let Some(path) = &report {
                let json = serde_json::to_string_pretty(&drift_report)?;
                std::fs::write(path, &json)?;
                eprintln!("Drift report written to {}", path.display());
            }

            if drift_report.drifted.is_empty() {
                println!("No drift detected.");
                return Ok(());
            }

            let current_commit = drift_report.commit.clone();
            let repo_roots: Vec<&std::path::Path> =
                docs_sources.iter().map(|s| s.repo_root.as_path()).collect();
            let doc_contents = collect_doc_contents(&repo_roots, &drift_report);

            let anchor_count: usize = drift_report.drifted.iter().map(|d| d.anchors.len()).sum();
            if config.triage.provider == "none" {
                eprintln!(
                    "Skipping triage — forwarding {} drifted anchor(s) across {} doc(s) to agent...",
                    anchor_count,
                    drift_report.drifted.len(),
                );
            } else {
                eprintln!(
                    "Sending {} drifted anchor(s) across {} doc(s) to {} for triage...",
                    anchor_count,
                    drift_report.drifted.len(),
                    config.triage.provider,
                );
            }

            let rt = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
            let triaged = rt.block_on(kedge::triage::triage_drift_report(
                &drift_report,
                &config.triage,
                &doc_contents,
            ))?;

            let (to_remediate, to_sync) = kedge::remediation::partition_by_action(&triaged);

            let mut remediated: Vec<kedge::models::RemediatedDoc> = Vec::new();
            let mut errors: Vec<String> = Vec::new();

            if config.remediation.batch && to_remediate.len() > 1 {
                let batch_payload = kedge::remediation::build_batch_agent_payload(
                    &to_remediate,
                    &current_commit,
                    &config.remediation.auto_merge_severities,
                    &config.remediation.agent_instructions,
                );
                let batch_auto_merge = batch_payload.auto_merge;
                let payload_json = serde_json::to_string(&batch_payload)
                    .context("failed to serialize batch agent payload")?;

                match kedge::remediation::agent::invoke_agent(
                    &config.remediation.agent_command,
                    &payload_json,
                    config.remediation.agent_timeout,
                    &config.remediation.agent_env,
                ) {
                    Ok(output) => {
                        let (_single, mut urls) = kedge::output::parse_agent_output(&output);
                        for doc in &to_remediate {
                            // Best-effort heuristic: match URL containing the doc's file stem
                            let stem = std::path::Path::new(&doc.doc)
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or(&doc.doc);
                            let mr_url = urls
                                .iter()
                                .position(|u| u.contains(stem))
                                .map(|i| urls.remove(i))
                                .or_else(|| {
                                    if !urls.is_empty() {
                                        Some(urls.remove(0))
                                    } else {
                                        None
                                    }
                                });

                            remediated.push(kedge::models::RemediatedDoc {
                                doc: doc.doc.clone(),
                                repo: doc.doc_repo.clone(),
                                mr_url,
                                severity: doc.severity,
                                auto_merged: batch_auto_merge,
                            });
                        }
                    }
                    Err(e) => {
                        for doc in &to_remediate {
                            errors.push(format!("{}: {}", doc.doc, e));
                        }
                    }
                }
            } else {
                // Per-doc mode (default): one agent invocation per drifted doc
                for doc in &to_remediate {
                    let auto_merge = kedge::remediation::should_auto_merge(
                        doc.severity,
                        &config.remediation.auto_merge_severities,
                    );
                    let payload = kedge::remediation::build_agent_payload(
                        doc,
                        &current_commit,
                        auto_merge,
                        &config.remediation.agent_instructions,
                    );
                    let payload_json = serde_json::to_string(&payload)
                        .context("failed to serialize agent payload")?;

                    match kedge::remediation::agent::invoke_agent(
                        &config.remediation.agent_command,
                        &payload_json,
                        config.remediation.agent_timeout,
                        &config.remediation.agent_env,
                    ) {
                        Ok(output) => {
                            let (mr_url, _all) = kedge::output::parse_agent_output(&output);
                            remediated.push(kedge::models::RemediatedDoc {
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
            }

            let provenance_advanced: Vec<kedge::models::ProvenanceSynced> = to_sync
                .iter()
                .map(|doc| {
                    if no_stamp {
                        return kedge::models::ProvenanceSynced {
                            doc: doc.doc.clone(),
                            anchors_synced: 0,
                            reason:
                                "no_update — provenance not stamped (use kedge sync to advance)"
                                    .to_string(),
                        };
                    }
                    let doc_file = match resolve_doc_file(&doc.doc, &repo_roots) {
                        Some(p) => p,
                        None => {
                            eprintln!(
                                "warning: cannot resolve doc path for provenance sync: {}",
                                doc.doc
                            );
                            return kedge::models::ProvenanceSynced {
                                doc: doc.doc.clone(),
                                anchors_synced: 0,
                                reason: "could not resolve doc path".to_string(),
                            };
                        }
                    };
                    let anchors: Vec<kedge::models::Anchor> = doc
                        .anchors
                        .iter()
                        .map(|a| kedge::models::Anchor {
                            repo: String::new(),
                            path: a.path.clone(),
                            symbol: a.symbol.clone(),
                            provenance: a.provenance.clone(),
                        })
                        .collect();
                    let synced = stamp_anchors(&code_repo_path, &doc_file, &anchors);
                    kedge::models::ProvenanceSynced {
                        doc: doc.doc.clone(),
                        anchors_synced: synced,
                        reason: "no_update — code changes did not affect documentation accuracy"
                            .to_string(),
                    }
                })
                .collect();

            let has_errors = !errors.is_empty();
            let summary = kedge::models::RemediationSummary {
                remediated,
                provenance_advanced,
                errors,
            };
            println!("{}", serde_json::to_string_pretty(&summary)?);

            if has_errors {
                std::process::exit(1);
            }
        }
        Command::Status => {
            let docs_path_str =
                std::env::var("KEDGE_DOCS_PATH").unwrap_or_else(|_| "docs".to_string());
            let docs_path = std::path::PathBuf::from(&docs_path_str);

            let cwd = std::env::current_dir()?;
            let cru = code_repo_url(&cwd);

            let exclude_dirs = kedge::config::DetectionConfig::default().exclude_dirs;
            let docs = kedge::frontmatter::scan_docs(&docs_path, &cru, None, &exclude_dirs);

            if docs.is_empty() {
                println!(
                    "No docs with kedge frontmatter found in {}",
                    docs_path.display()
                );
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
            let paths_to_process = resolve_md_files(files, &code_repo_path);

            let mut total_anchors = 0usize;
            for file_path in &paths_to_process {
                let doc = match kedge::frontmatter::parse_doc_file(file_path, "") {
                    Some(d) => d,
                    None => continue,
                };
                total_anchors +=
                    stamp_anchors(&code_repo_path, file_path, &doc.frontmatter.anchors);
            }

            println!(
                "Synced {} anchors with content-addressed provenance",
                total_anchors
            );
        }
        Command::Install {
            group,
            agent,
            link,
            workspace,
            check,
            recursive,
        } => {
            let config = kedge::config::Config::from_file(&cli.config)
                .context("failed to load kedge.toml — run `kedge init` first")?;

            let is_ci = std::env::var("CI").is_ok()
                || std::env::var("GITHUB_ACTIONS").is_ok()
                || std::env::var("GITLAB_CI").is_ok();
            let use_workspace = workspace || (is_ci && !link);

            let platforms: Vec<&kedge::config::AgentPlatform> = match &agent {
                Some(name) => {
                    if let Some(p) = config.find_agent(name) {
                        vec![p]
                    } else {
                        anyhow::bail!("unknown agent platform: {}", name);
                    }
                }
                None => config.agents.iter().collect(),
            };

            for doc_repo in &config.repos.docs {
                if check {
                    match kedge::install::repo_cache::is_up_to_date(
                        &doc_repo.url,
                        &doc_repo.git_ref,
                        config.repos.git_timeout,
                        &doc_repo.remote_name,
                    ) {
                        Ok(true) => {
                            eprintln!(
                                "steering docs are up to date ({})",
                                kedge::safety::sanitize_url(&doc_repo.url)
                            );
                            continue;
                        }
                        Ok(false) => {}
                        Err(e) => eprintln!("warning: could not check staleness: {}", e),
                    }
                }

                let cached = kedge::install::repo_cache::get_or_clone(
                    &doc_repo.url,
                    &doc_repo.git_ref,
                    config.repos.git_timeout,
                    &doc_repo.remote_name,
                )
                .with_context(|| {
                    format!(
                        "failed to get/clone repo {}",
                        kedge::safety::sanitize_url(&doc_repo.url)
                    )
                })?;

                let source_dir = if doc_repo.path.is_empty() {
                    cached
                } else {
                    let p = cached.join(&doc_repo.path);
                    kedge::safety::validate_path_within(&cached, &p)
                        .context("docs repo path escapes cache directory")?;
                    p
                };

                for platform in &platforms {
                    let target_dir = if use_workspace {
                        let workspace_steering =
                            shellexpand::tilde(&platform.workspace_steering).into_owned();
                        std::path::PathBuf::from(workspace_steering)
                    } else {
                        let global_steering =
                            shellexpand::tilde(&platform.global_steering).into_owned();
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
                        kedge::install::install_to_workspace(
                            &source_dir,
                            &target_dir,
                            group.as_deref(),
                            agents_file,
                            skill_dir.as_deref(),
                            recursive,
                        )
                        .with_context(|| {
                            format!("install_to_workspace failed for {}", platform.name)
                        })?;

                        if let Ok(cwd) = std::env::current_dir() {
                            let rel = target_dir.strip_prefix(&cwd).unwrap_or(&target_dir);
                            let _ =
                                kedge::install::add_to_git_exclude(&cwd, &rel.to_string_lossy());
                        }
                    } else {
                        #[cfg(unix)]
                        kedge::install::install_as_links(
                            &source_dir,
                            &target_dir,
                            group.as_deref(),
                            agents_file,
                            skill_dir.as_deref(),
                            recursive,
                        )
                        .with_context(|| {
                            format!("install_as_links failed for {}", platform.name)
                        })?;
                        #[cfg(not(unix))]
                        anyhow::bail!(
                            "--link mode requires a unix system (use --workspace instead)"
                        );
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
