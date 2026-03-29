use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "kedge",
    about = "Documentation drift detection and remediation"
)]
pub struct Cli {
    /// Path to kedge.toml config file
    #[arg(long, default_value = "kedge.toml")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a repo — creates config, scans for existing docs
    Init,

    /// Stamp/update provenance anchors in doc frontmatter
    Link {
        /// Specific doc files to link (default: all docs with kedge frontmatter)
        #[arg()]
        files: Vec<PathBuf>,
    },

    /// Detect drift, output report (exit 0 = clean, exit 1 = drift found)
    Check {
        /// Output report to file instead of stdout
        #[arg(long)]
        report: Option<PathBuf>,
    },

    /// Run semantic triage on detected drift (classify severity)
    Triage {
        /// Path to drift report JSON from `kedge check`
        #[arg(long)]
        report: Option<PathBuf>,
    },

    /// Full pipeline: check -> triage -> invoke agent -> open MR
    Update {
        /// Save drift report to file
        #[arg(long)]
        report: Option<PathBuf>,

        /// Skip provenance stamping for no_update anchors (use `kedge sync` later)
        #[arg(long)]
        no_stamp: bool,
    },

    /// Show all anchors and their current drift state
    Status,

    /// Advance provenance markers without doc content changes
    Sync {
        /// Specific doc files to sync
        #[arg()]
        files: Vec<PathBuf>,
    },

    /// Pull steering files from doc repo to local/workspace agent directories
    Install {
        /// Business unit group to install
        #[arg(long)]
        group: Option<String>,

        /// Target a specific agent platform (default: all configured)
        #[arg(long)]
        agent: Option<String>,

        /// Symlink to global steering directory
        #[arg(long, conflicts_with = "workspace")]
        link: bool,

        /// Copy to workspace steering directory
        #[arg(long, conflicts_with = "link")]
        workspace: bool,

        /// Only sync if stale (compares against doc repo HEAD)
        #[arg(long)]
        check: bool,

        /// Recursively include files from subdirectories within group/shared folders
        #[arg(long)]
        recursive: bool,
    },
}
