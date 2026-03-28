mod cli;

use clap::Parser;
use cli::{Cli, Command};

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
            println!("steer triage: not yet implemented (report: {:?})", report);
        }
        Command::Update { report } => {
            println!("steer update: not yet implemented (report: {:?})", report);
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
