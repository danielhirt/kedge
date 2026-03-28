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
            println!("steer check: not yet implemented (report: {:?})", report);
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
