# Steer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone CLI tool that detects documentation drift via AST fingerprinting, classifies severity with an AI triage step, and invokes an agent to open MRs updating stale docs.

**Architecture:** Three-layer pipeline (detection → triage → remediation) orchestrated by a Rust CLI. Detection uses tree-sitter for language-aware AST fingerprinting against provenance SHAs stored in markdown frontmatter. Triage classifies drift severity via a configurable AI provider. Remediation pipes a structured JSON payload to any agent command via stdin.

**Tech Stack:** Rust, clap (CLI), serde/toml (config/serialization), tree-sitter (AST parsing with java/go/typescript/xml grammars), git2 (libgit2 bindings), reqwest (HTTP for AI APIs), tokio (async runtime), sha2 (hashing), glob (pattern matching)

**Spec:** `docs/specs/2026-03-28-steer-design.md`

---

## File Structure

```
steer/
├── Cargo.toml
├── src/
│   ├── main.rs                       # Entry point, CLI dispatch
│   ├── cli.rs                        # clap command/arg definitions
│   ├── config.rs                     # steer.toml parsing + types
│   ├── models.rs                     # Shared data types (reports, anchors, payloads)
│   ├── frontmatter.rs                # YAML frontmatter extraction from markdown
│   ├── detection/
│   │   ├── mod.rs                    # Detection pipeline orchestration
│   │   ├── fingerprint.rs            # AST fingerprinting + content-hash fallback
│   │   └── git.rs                    # Git operations (file-at-rev, diff generation)
│   ├── triage/
│   │   ├── mod.rs                    # Triage pipeline orchestration
│   │   └── provider.rs              # AI provider abstraction (Anthropic, Bedrock)
│   ├── remediation/
│   │   ├── mod.rs                    # Remediation pipeline orchestration
│   │   └── agent.rs                  # Agent process invocation (stdin pipe)
│   └── install/
│       ├── mod.rs                    # Install pipeline orchestration
│       └── repo_cache.rs            # Doc repo clone/pull/cache management
├── tests/
│   ├── fixtures/
│   │   ├── steering_with_anchors.md  # Sample steering file with steer frontmatter
│   │   ├── steering_no_steer.md      # Steering file without steer block
│   │   ├── AuthService.java          # Sample Java file for fingerprinting
│   │   ├── handler.go                # Sample Go file for fingerprinting
│   │   ├── component.tsx             # Sample TSX file for fingerprinting
│   │   ├── form.xml                  # Sample XML file for fingerprinting
│   │   └── steer.toml                # Sample config
│   ├── config_test.rs
│   ├── frontmatter_test.rs
│   ├── fingerprint_test.rs
│   ├── detection_test.rs
│   ├── triage_test.rs
│   ├── remediation_test.rs
│   └── install_test.rs
└── docs/
```

**Decomposition rationale:**
- `models.rs` is the single source of truth for all data types shared across layers. Each layer module references these types rather than defining its own.
- Detection, triage, and remediation are separate modules because they have different dependencies (tree-sitter vs. HTTP vs. process spawning) and can be tested independently.
- Each command in `cli.rs` delegates to the corresponding layer module — commands are thin wrappers.
- `frontmatter.rs` is standalone because it's used by detection, install, and utility commands.

---

### Task 1: Project Scaffold + CLI Framework

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/cli.rs`

- [ ] **Step 1: Initialize Cargo project**

```bash
cd /home/daniel/Development/steer
cargo init --name steer
```

- [ ] **Step 2: Set up dependencies in Cargo.toml**

```toml
[package]
name = "steer"
version = "0.1.0"
edition = "2021"
description = "Documentation drift detection and remediation CLI"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
toml = "0.8"
tree-sitter = "0.24"
tree-sitter-java = "0.23"
tree-sitter-go = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-xml = "0.7"
git2 = "0.19"
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
sha2 = "0.10"
glob = "0.3"
anyhow = "1"
thiserror = "2"
dirs = "6"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

- [ ] **Step 3: Create CLI structure with all subcommands**

Write `src/cli.rs`:

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "steer", about = "Documentation drift detection and remediation")]
pub struct Cli {
    /// Path to steer.toml config file
    #[arg(long, default_value = "steer.toml")]
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
        /// Specific doc files to link (default: all docs with steer frontmatter)
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
        /// Path to drift report JSON from `steer check`
        #[arg(long)]
        report: Option<PathBuf>,
    },

    /// Full pipeline: check -> triage -> invoke agent -> open MR
    Update {
        /// Save drift report to file
        #[arg(long)]
        report: Option<PathBuf>,
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
    },
}
```

- [ ] **Step 4: Create main.rs entry point**

Write `src/main.rs`:

```rust
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
```

- [ ] **Step 5: Verify it compiles and runs**

Run: `cargo build 2>&1`
Expected: Compiles successfully

Run: `cargo run -- --help`
Expected: Shows help text with all subcommands listed

Run: `cargo run -- check`
Expected: Prints "steer check: not yet implemented (report: None)"

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/
git commit -m "feat: scaffold CLI with all subcommands stubbed"
```

---

### Task 2: Configuration Parsing

**Files:**
- Create: `src/config.rs`
- Create: `tests/fixtures/steer.toml`
- Create: `tests/config_test.rs`

- [ ] **Step 1: Write the test fixture**

Write `tests/fixtures/steer.toml`:

```toml
[detection]
languages = ["java", "go", "typescript", "xml"]
fallback = "content-hash"

[triage]
provider = "anthropic"
model = "claude-haiku-4-5-20251001"
severity_levels = ["no_update", "minor", "major"]

[remediation]
agent_command = "kiro --agent drift-updater"
auto_merge_severities = []

[[repos.docs]]
url = "git@gitlab.example.com:team/docs.git"
path = "steering/"
ref = "main"

[[agents]]
name = "kiro"
global_steering = "~/.kiro/steering/"
workspace_steering = ".kiro/steering/"
agents_file = "AGENTS.md"
skill_dir = ".kiro/skills/"

[[agents]]
name = "claude"
global_steering = "~/.claude/steering/"
workspace_steering = ".claude/steering/"
agents_file = "CLAUDE.md"
skill_dir = ".claude/skills/"
```

- [ ] **Step 2: Write failing tests**

Write `tests/config_test.rs`:

```rust
use steer::config::Config;
use std::path::Path;

#[test]
fn parses_full_config() {
    let config = Config::from_file(Path::new("tests/fixtures/steer.toml")).unwrap();

    assert_eq!(config.detection.languages, vec!["java", "go", "typescript", "xml"]);
    assert_eq!(config.detection.fallback, "content-hash");
    assert_eq!(config.triage.provider, "anthropic");
    assert_eq!(config.triage.model, "claude-haiku-4-5-20251001");
    assert_eq!(config.remediation.agent_command, "kiro --agent drift-updater");
    assert!(config.remediation.auto_merge_severities.is_empty());
}

#[test]
fn parses_doc_repos() {
    let config = Config::from_file(Path::new("tests/fixtures/steer.toml")).unwrap();

    assert_eq!(config.repos.docs.len(), 1);
    assert_eq!(config.repos.docs[0].url, "git@gitlab.example.com:team/docs.git");
    assert_eq!(config.repos.docs[0].path, "steering/");
    assert_eq!(config.repos.docs[0].git_ref, "main");
}

#[test]
fn parses_agent_platforms() {
    let config = Config::from_file(Path::new("tests/fixtures/steer.toml")).unwrap();

    assert_eq!(config.agents.len(), 2);
    assert_eq!(config.agents[0].name, "kiro");
    assert_eq!(config.agents[0].global_steering, "~/.kiro/steering/");
    assert_eq!(config.agents[0].workspace_steering, ".kiro/steering/");
    assert_eq!(config.agents[0].agents_file, "AGENTS.md");
    assert_eq!(config.agents[1].name, "claude");
    assert_eq!(config.agents[1].agents_file, "CLAUDE.md");
}

#[test]
fn returns_error_for_missing_file() {
    let result = Config::from_file(Path::new("nonexistent.toml"));
    assert!(result.is_err());
}

#[test]
fn finds_agent_by_name() {
    let config = Config::from_file(Path::new("tests/fixtures/steer.toml")).unwrap();

    let kiro = config.find_agent("kiro");
    assert!(kiro.is_some());
    assert_eq!(kiro.unwrap().global_steering, "~/.kiro/steering/");

    let missing = config.find_agent("nonexistent");
    assert!(missing.is_none());
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test config_test 2>&1`
Expected: FAIL — module `steer::config` not found

- [ ] **Step 4: Write config module**

Write `src/config.rs`:

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub detection: DetectionConfig,
    pub triage: TriageConfig,
    pub remediation: RemediationConfig,
    pub repos: ReposConfig,
    #[serde(default)]
    pub agents: Vec<AgentPlatform>,
}

#[derive(Debug, Deserialize)]
pub struct DetectionConfig {
    pub languages: Vec<String>,
    #[serde(default = "default_fallback")]
    pub fallback: String,
}

fn default_fallback() -> String {
    "content-hash".to_string()
}

#[derive(Debug, Deserialize)]
pub struct TriageConfig {
    pub provider: String,
    pub model: String,
    #[serde(default = "default_severity_levels")]
    pub severity_levels: Vec<String>,
}

fn default_severity_levels() -> Vec<String> {
    vec!["no_update".into(), "minor".into(), "major".into()]
}

#[derive(Debug, Deserialize)]
pub struct RemediationConfig {
    pub agent_command: String,
    #[serde(default)]
    pub auto_merge_severities: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReposConfig {
    pub docs: Vec<DocRepo>,
}

#[derive(Debug, Deserialize)]
pub struct DocRepo {
    pub url: String,
    pub path: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentPlatform {
    pub name: String,
    pub global_steering: String,
    pub workspace_steering: String,
    pub agents_file: String,
    #[serde(default)]
    pub skill_dir: String,
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config: {}", path.display()))?;
        Ok(config)
    }

    pub fn find_agent(&self, name: &str) -> Option<&AgentPlatform> {
        self.agents.iter().find(|a| a.name == name)
    }
}
```

- [ ] **Step 5: Export module from lib.rs**

Create `src/lib.rs`:

```rust
pub mod config;
```

Update `src/main.rs` to add at the top:

```rust
mod cli;
use steer::config::Config;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test config_test 2>&1`
Expected: All 5 tests pass

- [ ] **Step 7: Commit**

```bash
git add src/config.rs src/lib.rs tests/
git commit -m "feat: add steer.toml config parsing with agent platform support"
```

---

### Task 3: Shared Data Models

**Files:**
- Create: `src/models.rs`

- [ ] **Step 1: Write the data model types**

These types are the shared contract across all layers. Write `src/models.rs`:

```rust
use serde::{Deserialize, Serialize};

/// A single anchor binding a doc to a code location.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Anchor {
    pub repo: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub provenance: String,
}

/// Parsed steer frontmatter from a markdown file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteerFrontmatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub anchors: Vec<Anchor>,
}

/// A doc file with its parsed steer metadata.
#[derive(Debug, Clone)]
pub struct DocFile {
    pub path: String,
    pub doc_repo: String,
    pub frontmatter: SteerFrontmatter,
    pub content: String,
    /// The raw full frontmatter YAML (for Kiro inclusion fields etc.)
    pub raw_frontmatter: String,
}

// --- Detection layer output ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftedAnchor {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub provenance: String,
    pub current_commit: String,
    pub diff_summary: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftedDoc {
    pub doc: String,
    pub doc_repo: String,
    pub anchors: Vec<DriftedAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanDoc {
    pub doc: String,
    pub anchor_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub repo: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub commit: String,
    pub drifted: Vec<DriftedDoc>,
    pub clean: Vec<CleanDoc>,
}

// --- Triage layer output ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    NoUpdate,
    Minor,
    Major,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriagedAnchor {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub severity: Severity,
    pub provenance: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriagedDoc {
    pub doc: String,
    pub doc_repo: String,
    pub severity: Severity,
    pub summary: String,
    pub anchors: Vec<TriagedAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriagedReport {
    pub repo: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub commit: String,
    pub drifted: Vec<TriagedDoc>,
}

// --- Remediation layer ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTarget {
    pub repo: String,
    pub branch_prefix: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnchor {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub severity: Severity,
    pub summary: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPayload {
    pub action: String,
    pub severity: Severity,
    pub auto_merge: bool,
    pub target: AgentTarget,
    pub drifted_anchors: Vec<AgentAnchor>,
    pub instructions: String,
}

// --- Remediation summary ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediatedDoc {
    pub doc: String,
    pub repo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mr_url: Option<String>,
    pub severity: Severity,
    pub auto_merged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceSynced {
    pub doc: String,
    pub anchors_synced: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationSummary {
    pub remediated: Vec<RemediatedDoc>,
    pub provenance_advanced: Vec<ProvenanceSynced>,
    pub errors: Vec<String>,
}

impl Severity {
    /// Returns the max severity from a slice. Panics on empty input.
    pub fn max_of(severities: &[Severity]) -> Severity {
        *severities.iter().max().expect("severities must not be empty")
    }
}
```

- [ ] **Step 2: Export from lib.rs**

Update `src/lib.rs`:

```rust
pub mod config;
pub mod models;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build 2>&1`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/models.rs src/lib.rs
git commit -m "feat: add shared data models for drift reports, triage, and agent payloads"
```

---

### Task 4: Frontmatter Parser

**Files:**
- Create: `src/frontmatter.rs`
- Create: `tests/fixtures/steering_with_anchors.md`
- Create: `tests/fixtures/steering_no_steer.md`
- Create: `tests/frontmatter_test.rs`

- [ ] **Step 1: Write test fixtures**

Write `tests/fixtures/steering_with_anchors.md`:

```markdown
---
inclusion: fileMatch
fileMatchPattern: ["src/auth/**/*.java", "src/api/routes/auth.*"]
steer:
  group: payments-platform
  anchors:
    - repo: git@gitlab.example.com:team/backend.git
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: a1b2c3d4
    - repo: git@gitlab.example.com:team/backend.git
      path: src/auth/AuthService.java
      symbol: AuthService#refreshSession
      provenance: a1b2c3d4
---

# Authentication Steering

This module handles JWT validation and session management.
```

Write `tests/fixtures/steering_no_steer.md`:

```markdown
---
inclusion: always
---

# General Conventions

Use consistent naming patterns across the codebase.
```

- [ ] **Step 2: Write failing tests**

Write `tests/frontmatter_test.rs`:

```rust
use steer::frontmatter::parse_doc_file;
use std::path::Path;

#[test]
fn parses_steer_frontmatter_with_anchors() {
    let doc = parse_doc_file(
        Path::new("tests/fixtures/steering_with_anchors.md"),
        "git@gitlab.example.com:team/docs.git",
    )
    .unwrap();

    let fm = doc.frontmatter;
    assert_eq!(fm.group.as_deref(), Some("payments-platform"));
    assert_eq!(fm.anchors.len(), 2);
    assert_eq!(fm.anchors[0].path, "src/auth/AuthService.java");
    assert_eq!(
        fm.anchors[0].symbol.as_deref(),
        Some("AuthService#validateToken")
    );
    assert_eq!(fm.anchors[0].provenance, "a1b2c3d4");
    assert_eq!(
        fm.anchors[0].repo,
        "git@gitlab.example.com:team/backend.git"
    );
}

#[test]
fn returns_none_for_no_steer_block() {
    let result = parse_doc_file(
        Path::new("tests/fixtures/steering_no_steer.md"),
        "git@gitlab.example.com:team/docs.git",
    );
    assert!(result.is_none() || result.unwrap().frontmatter.anchors.is_empty());
}

#[test]
fn preserves_doc_content_after_frontmatter() {
    let doc = parse_doc_file(
        Path::new("tests/fixtures/steering_with_anchors.md"),
        "git@gitlab.example.com:team/docs.git",
    )
    .unwrap();

    assert!(doc.content.contains("# Authentication Steering"));
    assert!(doc.content.contains("JWT validation"));
}

#[test]
fn preserves_raw_frontmatter_for_kiro_fields() {
    let doc = parse_doc_file(
        Path::new("tests/fixtures/steering_with_anchors.md"),
        "git@gitlab.example.com:team/docs.git",
    )
    .unwrap();

    assert!(doc.raw_frontmatter.contains("inclusion: fileMatch"));
    assert!(doc.raw_frontmatter.contains("fileMatchPattern"));
}

#[test]
fn returns_none_for_file_without_frontmatter() {
    // Create a temp file with no frontmatter
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no_fm.md");
    std::fs::write(&path, "# Just a heading\n\nSome content.").unwrap();

    let result = parse_doc_file(&path, "git@example.com:repo.git");
    assert!(result.is_none());
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test frontmatter_test 2>&1`
Expected: FAIL — module `steer::frontmatter` not found

- [ ] **Step 4: Write frontmatter parser**

Write `src/frontmatter.rs`:

```rust
use crate::models::{DocFile, SteerFrontmatter};
use std::path::Path;

/// Parse a markdown file, extract YAML frontmatter, and return a DocFile
/// if it contains a `steer:` block with anchors. Returns None if no steer
/// frontmatter is found.
pub fn parse_doc_file(path: &Path, doc_repo: &str) -> Option<DocFile> {
    let text = std::fs::read_to_string(path).ok()?;
    parse_doc_string(&text, &path.to_string_lossy(), doc_repo)
}

/// Parse from a string (useful for testing and when reading from git objects).
pub fn parse_doc_string(text: &str, doc_path: &str, doc_repo: &str) -> Option<DocFile> {
    let (raw_frontmatter, body) = extract_frontmatter(text)?;

    // Parse the YAML frontmatter to look for the steer block
    let yaml: serde_yaml::Value = serde_yaml::from_str(&raw_frontmatter).ok()?;
    let steer_value = yaml.get("steer")?;

    let frontmatter: SteerFrontmatter = serde_yaml::from_value(steer_value.clone()).ok()?;

    if frontmatter.anchors.is_empty() {
        return None;
    }

    Some(DocFile {
        path: doc_path.to_string(),
        doc_repo: doc_repo.to_string(),
        frontmatter,
        content: body.to_string(),
        raw_frontmatter: raw_frontmatter.to_string(),
    })
}

/// Split markdown into (frontmatter_yaml, body). Returns None if no
/// frontmatter delimiters found.
fn extract_frontmatter(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start();
    if !text.starts_with("---") {
        return None;
    }

    // Find the closing ---
    let after_open = &text[3..];
    let close_pos = after_open.find("\n---")?;

    let yaml = after_open[..close_pos].trim();
    // Body starts after the closing --- and its newline
    let body_start = 3 + close_pos + 4; // "---" + position + "\n---"
    let body = if body_start < text.len() {
        text[body_start..].trim_start_matches('\n')
    } else {
        ""
    };

    Some((yaml, body))
}

/// Scan a directory for all markdown files with steer frontmatter.
/// Optionally filter by group.
pub fn scan_docs(
    dir: &Path,
    doc_repo: &str,
    group_filter: Option<&str>,
) -> Vec<DocFile> {
    let pattern = format!("{}/**/*.md", dir.display());
    let mut docs = Vec::new();

    for entry in glob::glob(&pattern).into_iter().flatten().flatten() {
        if let Some(doc) = parse_doc_file(&entry, doc_repo) {
            if let Some(filter) = group_filter {
                if doc.frontmatter.group.as_deref() != Some(filter) {
                    continue;
                }
            }
            docs.push(doc);
        }
    }

    docs
}
```

- [ ] **Step 5: Export from lib.rs**

Update `src/lib.rs`:

```rust
pub mod config;
pub mod frontmatter;
pub mod models;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test frontmatter_test 2>&1`
Expected: All 5 tests pass

- [ ] **Step 7: Commit**

```bash
git add src/frontmatter.rs src/lib.rs tests/
git commit -m "feat: add frontmatter parser for steer YAML blocks in markdown"
```

---

### Task 5: Content-Hash Fingerprinting

**Files:**
- Create: `src/detection/mod.rs`
- Create: `src/detection/fingerprint.rs`
- Create: `tests/fingerprint_test.rs`
- Create: `tests/fixtures/AuthService.java`

- [ ] **Step 1: Write test fixtures**

Write `tests/fixtures/AuthService.java`:

```java
package com.example.auth;

import java.util.List;

public class AuthService {

    public boolean validateToken(String token) {
        if (token == null || token.isEmpty()) {
            return false;
        }
        return verifySignature(token);
    }

    public void refreshSession(String sessionId) {
        // Refresh the session expiry
        Session session = sessionStore.get(sessionId);
        if (session != null) {
            session.extend();
        }
    }

    private boolean verifySignature(String token) {
        return true; // simplified
    }
}
```

- [ ] **Step 2: Write failing tests**

Write `tests/fingerprint_test.rs`:

```rust
use steer::detection::fingerprint::{content_hash, ast_fingerprint, Language};

#[test]
fn content_hash_produces_consistent_output() {
    let content = "public class Foo { }";
    let hash1 = content_hash(content);
    let hash2 = content_hash(content);
    assert_eq!(hash1, hash2);
    assert!(!hash1.is_empty());
}

#[test]
fn content_hash_differs_for_different_content() {
    let hash1 = content_hash("public class Foo { }");
    let hash2 = content_hash("public class Bar { }");
    assert_ne!(hash1, hash2);
}

#[test]
fn ast_fingerprint_ignores_whitespace_changes() {
    let compact = "public class Foo { public void bar() { return; } }";
    let spaced = "public class Foo {\n    public void bar() {\n        return;\n    }\n}";

    let hash1 = ast_fingerprint(compact, Language::Java, None).unwrap();
    let hash2 = ast_fingerprint(spaced, Language::Java, None).unwrap();
    assert_eq!(hash1, hash2);
}

#[test]
fn ast_fingerprint_detects_code_changes() {
    let before = "public class Foo { public void bar() { return; } }";
    let after = "public class Foo { public void bar(int x) { return; } }";

    let hash1 = ast_fingerprint(before, Language::Java, None).unwrap();
    let hash2 = ast_fingerprint(after, Language::Java, None).unwrap();
    assert_ne!(hash1, hash2);
}

#[test]
fn ast_fingerprint_with_symbol_filters_to_method() {
    let source = std::fs::read_to_string("tests/fixtures/AuthService.java").unwrap();

    let full_hash = ast_fingerprint(&source, Language::Java, None).unwrap();
    let method_hash =
        ast_fingerprint(&source, Language::Java, Some("AuthService#validateToken")).unwrap();

    // Symbol-scoped hash should differ from full-file hash
    assert_ne!(full_hash, method_hash);
    // Symbol hash should be consistent
    let method_hash2 =
        ast_fingerprint(&source, Language::Java, Some("AuthService#validateToken")).unwrap();
    assert_eq!(method_hash, method_hash2);
}

#[test]
fn ast_fingerprint_ignores_comment_changes() {
    let before = "public class Foo { public void bar() { return; } }";
    let after = "public class Foo { /* added comment */ public void bar() { return; } }";

    let hash1 = ast_fingerprint(before, Language::Java, None).unwrap();
    let hash2 = ast_fingerprint(after, Language::Java, None).unwrap();
    assert_eq!(hash1, hash2);
}

#[test]
fn language_detection_from_extension() {
    assert_eq!(Language::from_extension("java"), Some(Language::Java));
    assert_eq!(Language::from_extension("go"), Some(Language::Go));
    assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
    assert_eq!(Language::from_extension("tsx"), Some(Language::TypeScript));
    assert_eq!(Language::from_extension("xml"), Some(Language::Xml));
    assert_eq!(Language::from_extension("py"), None);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test fingerprint_test 2>&1`
Expected: FAIL — module not found

- [ ] **Step 4: Write fingerprinting module**

Write `src/detection/fingerprint.rs`:

```rust
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Java,
    Go,
    TypeScript,
    Xml,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "java" => Some(Language::Java),
            "go" => Some(Language::Go),
            "ts" | "tsx" | "js" | "jsx" => Some(Language::TypeScript),
            "xml" => Some(Language::Xml),
            _ => None,
        }
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            Language::Java => tree_sitter_java::LANGUAGE.into(),
            Language::Go => tree_sitter_go::LANGUAGE.into(),
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Language::Xml => tree_sitter_xml::LANGUAGE_XML.into(),
        }
    }
}

/// Content-hash fallback: SHA-256 of the raw text with whitespace normalized.
pub fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// AST-based fingerprint: parse with tree-sitter, walk the normalized AST,
/// hash node kinds + token text (excluding whitespace and comments).
/// If `symbol` is provided (e.g., "ClassName#methodName"), only fingerprint
/// that specific declaration.
pub fn ast_fingerprint(
    source: &str,
    lang: Language,
    symbol: Option<&str>,
) -> Result<String> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang.tree_sitter_language())
        .context("Failed to set tree-sitter language")?;

    let tree = parser
        .parse(source, None)
        .context("Failed to parse source")?;

    let root = tree.root_node();

    let target_node = if let Some(sym) = symbol {
        find_symbol_node(root, source, sym, lang)
            .with_context(|| format!("Symbol not found: {sym}"))?
    } else {
        root
    };

    let mut hasher = Sha256::new();
    walk_and_hash(target_node, source, &mut hasher, lang);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Recursively walk the AST, hashing node kinds and token text for
/// non-whitespace, non-comment leaf nodes.
fn walk_and_hash(node: Node, source: &str, hasher: &mut Sha256, lang: Language) {
    let kind = node.kind();

    // Skip comments and whitespace
    if is_comment(kind, lang) {
        return;
    }

    // For named (non-anonymous) nodes, hash the node kind
    if node.is_named() {
        hasher.update(kind.as_bytes());
        hasher.update(b"|");
    }

    if node.child_count() == 0 {
        // Leaf node: hash the actual token text
        let text = &source[node.byte_range()];
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            hasher.update(trimmed.as_bytes());
            hasher.update(b"|");
        }
    } else {
        // Internal node: recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk_and_hash(child, source, hasher, lang);
        }
    }
}

fn is_comment(kind: &str, _lang: Language) -> bool {
    matches!(
        kind,
        "comment" | "line_comment" | "block_comment" | "javadoc_comment"
    )
}

/// Find a node matching a symbol spec like "ClassName#methodName".
/// For "ClassName" alone, returns the class node.
/// For "ClassName#methodName", returns the method node within the class.
fn find_symbol_node<'a>(
    root: Node<'a>,
    source: &str,
    symbol: &str,
    lang: Language,
) -> Result<Node<'a>> {
    let (class_name, method_name) = if let Some((c, m)) = symbol.split_once('#') {
        (Some(c), Some(m))
    } else {
        (Some(symbol), None)
    };

    find_symbol_recursive(root, source, class_name, method_name, lang)
        .with_context(|| format!("Could not find symbol: {symbol}"))
}

fn find_symbol_recursive<'a>(
    node: Node<'a>,
    source: &str,
    class_name: Option<&str>,
    method_name: Option<&str>,
    lang: Language,
) -> Result<Node<'a>> {
    let kind = node.kind();

    // Check if this node is the target class/type
    let is_class_node = matches!(
        kind,
        "class_declaration"
            | "interface_declaration"
            | "type_declaration"
            | "type_spec"
            | "class_definition"
    );

    if is_class_node {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = &source[name_node.byte_range()];
            if class_name == Some(name) {
                if let Some(method) = method_name {
                    // Look for the method inside this class
                    return find_method_in_class(node, source, method, lang);
                }
                return Ok(node);
            }
        }
    }

    // Check if this is a top-level function (Go, TypeScript)
    let is_function_node = matches!(
        kind,
        "function_declaration" | "method_declaration" | "function"
    );
    if is_function_node && class_name.is_some() && method_name.is_none() {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = &source[name_node.byte_range()];
            if class_name == Some(name) {
                return Ok(node);
            }
        }
    }

    // Recurse
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(found) =
            find_symbol_recursive(child, source, class_name, method_name, lang)
        {
            return Ok(found);
        }
    }

    anyhow::bail!("Symbol not found in subtree")
}

fn find_method_in_class<'a>(
    class_node: Node<'a>,
    source: &str,
    method_name: &str,
    _lang: Language,
) -> Result<Node<'a>> {
    let mut cursor = class_node.walk();
    for child in class_node.children(&mut cursor) {
        let kind = child.kind();
        if matches!(
            kind,
            "method_declaration"
                | "function_declaration"
                | "method_definition"
                | "function"
                | "constructor_declaration"
        ) {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = &source[name_node.byte_range()];
                if name == method_name {
                    return Ok(child);
                }
            }
        }
        // Check inside class_body for Java
        if kind == "class_body" {
            if let Ok(found) = find_method_in_class(child, source, method_name, _lang) {
                return Ok(found);
            }
        }
    }
    anyhow::bail!("Method {method_name} not found in class")
}
```

Write `src/detection/mod.rs`:

```rust
pub mod fingerprint;
pub mod git;
```

Note: `git.rs` will be created in the next task. Create a placeholder:

```rust
// src/detection/git.rs — placeholder, implemented in Task 6
```

- [ ] **Step 5: Export from lib.rs**

Update `src/lib.rs`:

```rust
pub mod config;
pub mod detection;
pub mod frontmatter;
pub mod models;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test fingerprint_test 2>&1`
Expected: All 7 tests pass

Note: If tree-sitter crate versions have API differences from what's shown, adjust the `tree_sitter_language()` method. The tree-sitter crates may use `tree_sitter_java::language()` (function) vs `tree_sitter_java::LANGUAGE` (constant) depending on version. Check `cargo doc` output if compilation fails and adjust accordingly.

- [ ] **Step 7: Commit**

```bash
git add src/detection/ tests/
git commit -m "feat: add AST fingerprinting with tree-sitter and content-hash fallback"
```

---

### Task 6: Git Operations

**Files:**
- Create: `src/detection/git.rs`
- Create: `tests/detection_test.rs`

- [ ] **Step 1: Write failing tests**

These tests require a real git repo, so they create temporary repos with committed files.

Write `tests/detection_test.rs`:

```rust
use std::process::Command;
use tempfile::TempDir;

/// Helper: create a git repo with an initial commit containing a file.
fn setup_git_repo(file_name: &str, content: &str) -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let repo_path = dir.path();

    Command::new("git").args(["init"]).current_dir(repo_path).output().unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    std::fs::write(repo_path.join(file_name), content).unwrap();
    Command::new("git")
        .args(["add", file_name])
        .current_dir(repo_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    let sha = String::from_utf8(output.stdout).unwrap().trim().to_string();

    (dir, sha)
}

#[test]
fn reads_file_at_provenance_commit() {
    let (dir, sha) = setup_git_repo("hello.txt", "original content");

    // Modify the file after commit
    std::fs::write(dir.path().join("hello.txt"), "modified content").unwrap();

    let content =
        steer::detection::git::read_file_at_rev(dir.path(), &sha, "hello.txt").unwrap();
    assert_eq!(content, "original content");
}

#[test]
fn generates_diff_between_commits() {
    let (dir, sha1) = setup_git_repo("code.java", "class Foo {}");

    // Make a second commit with changes
    std::fs::write(dir.path().join("code.java"), "class Foo { int x; }").unwrap();
    Command::new("git")
        .args(["add", "code.java"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add field"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let diff =
        steer::detection::git::diff_since(dir.path(), &sha1, "code.java").unwrap();
    assert!(diff.contains("class Foo"));
    assert!(diff.contains("int x"));
}

#[test]
fn gets_head_sha() {
    let (dir, sha) = setup_git_repo("f.txt", "content");
    let head = steer::detection::git::head_sha(dir.path()).unwrap();
    assert_eq!(head, sha);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test detection_test 2>&1`
Expected: FAIL — functions not found

- [ ] **Step 3: Implement git operations**

Write `src/detection/git.rs`:

```rust
use anyhow::{Context, Result};
use git2::Repository;

/// Read a file's content at a specific git revision.
pub fn read_file_at_rev(
    repo_path: &std::path::Path,
    rev: &str,
    file_path: &str,
) -> Result<String> {
    let repo = Repository::open(repo_path)
        .with_context(|| format!("Not a git repo: {}", repo_path.display()))?;
    let obj = repo
        .revparse_single(rev)
        .with_context(|| format!("Could not resolve rev: {rev}"))?;
    let commit = obj
        .peel_to_commit()
        .with_context(|| format!("Rev is not a commit: {rev}"))?;
    let tree = commit.tree()?;
    let entry = tree
        .get_path(std::path::Path::new(file_path))
        .with_context(|| format!("File not found at {rev}: {file_path}"))?;
    let blob = repo.find_blob(entry.id())?;
    let content = std::str::from_utf8(blob.content())
        .with_context(|| format!("File is not UTF-8: {file_path}"))?;
    Ok(content.to_string())
}

/// Generate a unified diff for a file between a revision and the current working tree.
pub fn diff_since(
    repo_path: &std::path::Path,
    from_rev: &str,
    file_path: &str,
) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["diff", from_rev, "--", file_path])
        .current_dir(repo_path)
        .output()
        .context("Failed to run git diff")?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Get the SHA of HEAD in the given repo.
pub fn head_sha(repo_path: &std::path::Path) -> Result<String> {
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    Ok(commit.id().to_string())
}

/// Generate a short summary of what changed in a diff (first 200 chars of diff stat).
pub fn diff_summary(
    repo_path: &std::path::Path,
    from_rev: &str,
    file_path: &str,
) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--stat", from_rev, "--", file_path])
        .current_dir(repo_path)
        .output()
        .context("Failed to run git diff --stat")?;

    let stat = String::from_utf8_lossy(&output.stdout);
    Ok(stat.chars().take(200).collect())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test detection_test 2>&1`
Expected: All 3 tests pass

- [ ] **Step 5: Commit**

```bash
git add src/detection/git.rs tests/detection_test.rs
git commit -m "feat: add git operations for provenance lookups and diff generation"
```

---

### Task 7: Detection Pipeline + `steer check`

**Files:**
- Modify: `src/detection/mod.rs`
- Modify: `src/main.rs`
- Modify: `src/cli.rs` (if needed)

This task wires the frontmatter parser, fingerprinting, and git operations into the `steer check` command.

- [ ] **Step 1: Write detection pipeline integration test**

Add to `tests/detection_test.rs`:

```rust
use steer::config::Config;
use steer::detection;
use steer::models::DriftReport;

#[test]
fn detection_pipeline_finds_drift_when_code_changed() {
    // Setup: git repo with a Java file
    let (dir, sha) = setup_git_repo(
        "src/auth/AuthService.java",
        "public class AuthService { public boolean validate(String t) { return true; } }",
    );

    // Create a steering doc that anchors to this file at the initial commit
    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        r#"---
inclusion: fileMatch
fileMatchPattern: ["src/auth/**/*.java"]
steer:
  group: test
  anchors:
    - repo: "file://{repo}"
      path: src/auth/AuthService.java
      symbol: AuthService#validate
      provenance: {sha}
---

# Auth docs
"#,
        repo = dir.path().display(),
        sha = sha,
    );
    std::fs::create_dir_all(docs_dir.path().join("steering/test")).unwrap();
    std::fs::write(
        docs_dir.path().join("steering/test/auth.md"),
        &steering,
    )
    .unwrap();

    // Now modify the Java file (change the method signature)
    std::fs::create_dir_all(dir.path().join("src/auth")).unwrap();
    std::fs::write(
        dir.path().join("src/auth/AuthService.java"),
        "public class AuthService { public boolean validate(String t, List<String> scopes) { return true; } }",
    ).unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add scopes param"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Run detection
    let report = detection::detect_drift(
        dir.path(),
        docs_dir.path().join("steering/test"),
        &format!("file://{}", dir.path().display()),
        "test-repo",
    )
    .unwrap();

    assert_eq!(report.drifted.len(), 1);
    assert_eq!(report.drifted[0].anchors.len(), 1);
    assert_eq!(
        report.drifted[0].anchors[0].path,
        "src/auth/AuthService.java"
    );
}

#[test]
fn detection_pipeline_reports_clean_when_no_changes() {
    let (dir, sha) = setup_git_repo(
        "src/auth/AuthService.java",
        "public class AuthService { public boolean validate(String t) { return true; } }",
    );

    let docs_dir = TempDir::new().unwrap();
    let steering = format!(
        r#"---
steer:
  group: test
  anchors:
    - repo: "file://{repo}"
      path: src/auth/AuthService.java
      provenance: {sha}
---

# Auth docs
"#,
        repo = dir.path().display(),
        sha = sha,
    );
    std::fs::create_dir_all(docs_dir.path().join("steering/test")).unwrap();
    std::fs::write(
        docs_dir.path().join("steering/test/auth.md"),
        &steering,
    )
    .unwrap();

    let report = detection::detect_drift(
        dir.path(),
        docs_dir.path().join("steering/test"),
        &format!("file://{}", dir.path().display()),
        "test-repo",
    )
    .unwrap();

    assert!(report.drifted.is_empty());
    assert_eq!(report.clean.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test detection_test detection_pipeline 2>&1`
Expected: FAIL — `detection::detect_drift` not found

- [ ] **Step 3: Implement detection pipeline**

Update `src/detection/mod.rs`:

```rust
pub mod fingerprint;
pub mod git;

use crate::frontmatter;
use crate::models::{CleanDoc, DriftReport, DriftedAnchor, DriftedDoc};
use anyhow::{Context, Result};
use fingerprint::Language;
use std::path::Path;

/// Run drift detection: compare anchored code at HEAD vs provenance for all
/// doc files in the given directory that reference the given code repo.
pub fn detect_drift(
    code_repo_path: &Path,
    docs_dir: impl AsRef<Path>,
    code_repo_url: &str,
    repo_name: &str,
) -> Result<DriftReport> {
    let head = git::head_sha(code_repo_path)?;
    let docs = frontmatter::scan_docs(docs_dir.as_ref(), code_repo_url, None);

    let mut drifted = Vec::new();
    let mut clean = Vec::new();

    for doc in &docs {
        let mut drifted_anchors = Vec::new();

        for anchor in &doc.frontmatter.anchors {
            // Only check anchors that reference this code repo
            if !anchor.repo.contains(code_repo_url)
                && !code_repo_url.contains(&anchor.repo)
            {
                // Check for file:// path match as well
                let repo_canonical = anchor.repo.trim_start_matches("file://");
                let code_canonical =
                    code_repo_path.to_string_lossy();
                if repo_canonical != code_canonical.as_ref() {
                    continue;
                }
            }

            match check_anchor(code_repo_path, anchor, &head) {
                Ok(Some(drifted_anchor)) => drifted_anchors.push(drifted_anchor),
                Ok(None) => {} // clean
                Err(e) => {
                    eprintln!(
                        "Warning: failed to check anchor {}#{}: {}",
                        anchor.path,
                        anchor.symbol.as_deref().unwrap_or("*"),
                        e
                    );
                }
            }
        }

        if drifted_anchors.is_empty() {
            clean.push(CleanDoc {
                doc: doc.path.clone(),
                anchor_count: doc.frontmatter.anchors.len(),
            });
        } else {
            drifted.push(DriftedDoc {
                doc: doc.path.clone(),
                doc_repo: doc.doc_repo.clone(),
                anchors: drifted_anchors,
            });
        }
    }

    Ok(DriftReport {
        repo: repo_name.to_string(),
        git_ref: "main".to_string(),
        commit: head,
        drifted,
        clean,
    })
}

fn check_anchor(
    repo_path: &Path,
    anchor: &crate::models::Anchor,
    head_sha: &str,
) -> Result<Option<DriftedAnchor>> {
    // Read file at provenance and at HEAD
    let provenance_content =
        git::read_file_at_rev(repo_path, &anchor.provenance, &anchor.path)
            .with_context(|| {
                format!(
                    "Could not read {} at provenance {}",
                    anchor.path, anchor.provenance
                )
            })?;

    let current_content =
        git::read_file_at_rev(repo_path, head_sha, &anchor.path)
            .or_else(|_| {
                // File might be in working tree but not yet committed
                std::fs::read_to_string(repo_path.join(&anchor.path))
                    .context("File not found")
            })?;

    // Determine language from file extension
    let ext = Path::new(&anchor.path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let (prov_fingerprint, curr_fingerprint) = match Language::from_extension(ext) {
        Some(lang) => {
            let prov = fingerprint::ast_fingerprint(
                &provenance_content,
                lang,
                anchor.symbol.as_deref(),
            )?;
            let curr = fingerprint::ast_fingerprint(
                &current_content,
                lang,
                anchor.symbol.as_deref(),
            )?;
            (prov, curr)
        }
        None => {
            let prov = fingerprint::content_hash(&provenance_content);
            let curr = fingerprint::content_hash(&current_content);
            (prov, curr)
        }
    };

    if prov_fingerprint == curr_fingerprint {
        return Ok(None); // No drift
    }

    let diff = git::diff_since(repo_path, &anchor.provenance, &anchor.path)
        .unwrap_or_default();
    let summary = git::diff_summary(repo_path, &anchor.provenance, &anchor.path)
        .unwrap_or_default();

    Ok(Some(DriftedAnchor {
        path: anchor.path.clone(),
        symbol: anchor.symbol.clone(),
        provenance: anchor.provenance.clone(),
        current_commit: head_sha.to_string(),
        diff_summary: summary,
        diff,
    }))
}
```

- [ ] **Step 4: Wire `steer check` into main.rs**

Update the `Command::Check` arm in `src/main.rs`:

```rust
Command::Check { report } => {
    let config = Config::from_file(&cli.config)?;
    let code_repo_path = std::env::current_dir()?;
    let repo_name = code_repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // For each configured doc repo, clone/use cache and run detection
    // For now, support a local docs path passed via env var for testing
    let docs_path = std::env::var("STEER_DOCS_PATH")
        .unwrap_or_else(|_| ".".to_string());

    let drift_report = steer::detection::detect_drift(
        &code_repo_path,
        &docs_path,
        &config.repos.docs[0].url,
        &repo_name,
    )?;

    let json = serde_json::to_string_pretty(&drift_report)?;

    if let Some(report_path) = report {
        std::fs::write(&report_path, &json)?;
        eprintln!("Drift report written to {}", report_path.display());
    } else {
        println!("{json}");
    }

    if !drift_report.drifted.is_empty() {
        std::process::exit(1);
    }
}
```

Add the import at top of main.rs: `use steer::config::Config;`

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test detection_test 2>&1`
Expected: All 5 tests pass

- [ ] **Step 6: Commit**

```bash
git add src/detection/mod.rs src/main.rs tests/detection_test.rs
git commit -m "feat: implement detection pipeline and steer check command"
```

---

### Task 8: Triage Layer

**Files:**
- Create: `src/triage/mod.rs`
- Create: `src/triage/provider.rs`
- Create: `tests/triage_test.rs`

- [ ] **Step 1: Write failing tests**

Write `tests/triage_test.rs`:

```rust
use steer::models::*;
use steer::triage;

#[test]
fn builds_triage_prompt_from_drift_report() {
    let drifted_doc = DriftedDoc {
        doc: "auth.md".to_string(),
        doc_repo: "git@example.com:docs.git".to_string(),
        anchors: vec![DriftedAnchor {
            path: "src/Auth.java".to_string(),
            symbol: Some("Auth#validate".to_string()),
            provenance: "abc123".to_string(),
            current_commit: "def456".to_string(),
            diff_summary: "Added scopes param".to_string(),
            diff: "+public boolean validate(String t, List<String> scopes)".to_string(),
        }],
    };

    let prompt = triage::build_triage_prompt(&drifted_doc, "# Auth docs\nValidates tokens.");
    assert!(prompt.contains("# Auth docs"));
    assert!(prompt.contains("Auth#validate"));
    assert!(prompt.contains("scopes"));
    assert!(prompt.contains("no_update"));
    assert!(prompt.contains("minor"));
    assert!(prompt.contains("major"));
}

#[test]
fn parses_triage_response_json() {
    let response = r#"[
        {"path": "src/Auth.java", "symbol": "Auth#validate", "severity": "major"}
    ]"#;

    let classifications = triage::parse_triage_response(response).unwrap();
    assert_eq!(classifications.len(), 1);
    assert_eq!(classifications[0].severity, Severity::Major);
}

#[test]
fn applies_triage_to_drift_report() {
    let drift_report = DriftReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc123".to_string(),
        drifted: vec![DriftedDoc {
            doc: "auth.md".to_string(),
            doc_repo: "git@example.com:docs.git".to_string(),
            anchors: vec![
                DriftedAnchor {
                    path: "src/Auth.java".to_string(),
                    symbol: Some("Auth#validate".to_string()),
                    provenance: "old1".to_string(),
                    current_commit: "abc123".to_string(),
                    diff_summary: "Added param".to_string(),
                    diff: "+param".to_string(),
                },
                DriftedAnchor {
                    path: "src/Auth.java".to_string(),
                    symbol: Some("Auth#refresh".to_string()),
                    provenance: "old2".to_string(),
                    current_commit: "abc123".to_string(),
                    diff_summary: "Whitespace".to_string(),
                    diff: " ".to_string(),
                },
            ],
        }],
        clean: vec![],
    };

    let classifications = vec![
        triage::AnchorClassification {
            path: "src/Auth.java".to_string(),
            symbol: Some("Auth#validate".to_string()),
            severity: Severity::Major,
        },
        triage::AnchorClassification {
            path: "src/Auth.java".to_string(),
            symbol: Some("Auth#refresh".to_string()),
            severity: Severity::NoUpdate,
        },
    ];

    let triaged = triage::apply_classifications(&drift_report, &classifications, "Auth was updated with scopes");
    assert_eq!(triaged.drifted.len(), 1);
    assert_eq!(triaged.drifted[0].severity, Severity::Major); // max of anchors
    assert_eq!(triaged.drifted[0].anchors.len(), 2); // both anchors preserved
    assert_eq!(triaged.drifted[0].anchors[0].severity, Severity::Major);
    assert_eq!(triaged.drifted[0].anchors[1].severity, Severity::NoUpdate);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test triage_test 2>&1`
Expected: FAIL — module not found

- [ ] **Step 3: Implement AI provider abstraction**

Write `src/triage/provider.rs`:

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
}

/// Call an AI provider to classify drift severity.
/// Currently supports "anthropic". Returns the raw text response.
pub async fn classify(
    provider: &str,
    model: &str,
    prompt: &str,
) -> Result<String> {
    match provider {
        "anthropic" => call_anthropic(model, prompt).await,
        "bedrock" => call_bedrock(model, prompt).await,
        other => anyhow::bail!("Unsupported triage provider: {other}"),
    }
}

async fn call_anthropic(model: &str, prompt: &str) -> Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY not set")?;

    let client = reqwest::Client::new();
    let request = AnthropicRequest {
        model: model.to_string(),
        max_tokens: 4096,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await
        .context("Failed to call Anthropic API")?;

    let body: AnthropicResponse = response
        .json()
        .await
        .context("Failed to parse Anthropic response")?;

    body.content
        .first()
        .map(|c| c.text.clone())
        .context("Empty response from Anthropic")
}

async fn call_bedrock(model: &str, prompt: &str) -> Result<String> {
    // Bedrock uses AWS SDK — stub for now, same JSON contract
    anyhow::bail!("Bedrock provider not yet implemented. Use ANTHROPIC_API_KEY with provider = \"anthropic\" for now.")
}
```

- [ ] **Step 4: Implement triage pipeline**

Write `src/triage/mod.rs`:

```rust
pub mod provider;

use crate::models::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorClassification {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub severity: Severity,
}

/// Build the triage prompt for a single drifted doc.
pub fn build_triage_prompt(drifted_doc: &DriftedDoc, doc_content: &str) -> String {
    let mut anchors_section = String::new();
    for anchor in &drifted_doc.anchors {
        anchors_section.push_str(&format!(
            "\n### {} {}\n```diff\n{}\n```\n",
            anchor.path,
            anchor
                .symbol
                .as_deref()
                .map(|s| format!("({})", s))
                .unwrap_or_default(),
            anchor.diff,
        ));
    }

    format!(
        r#"You are classifying documentation drift severity.

## Current documentation
{doc_content}

## Code changes since last doc review
{anchors_section}

For each anchor, classify as:
- no_update: code changed but the documentation is still accurate
- minor: documentation needs a mechanical/obvious update (rename, move, signature change)
- major: documentation needs substantive revision (behavior, architecture, or intent changed)

Respond with a JSON array only. Each element must have "path", "symbol" (or null), and "severity" fields. Example:
[{{"path": "src/Foo.java", "symbol": "Foo#bar", "severity": "minor"}}]"#
    )
}

/// Parse the AI model's JSON response into classifications.
pub fn parse_triage_response(response: &str) -> Result<Vec<AnchorClassification>> {
    // The model might wrap JSON in markdown code fences — strip them
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str(cleaned).context("Failed to parse triage response as JSON")
}

/// Apply classifications to a drift report, producing a triaged report.
pub fn apply_classifications(
    drift_report: &DriftReport,
    classifications: &[AnchorClassification],
    summary: &str,
) -> TriagedReport {
    let mut triaged_docs = Vec::new();

    for drifted_doc in &drift_report.drifted {
        let mut triaged_anchors = Vec::new();

        for anchor in &drifted_doc.anchors {
            let severity = classifications
                .iter()
                .find(|c| {
                    c.path == anchor.path && c.symbol == anchor.symbol
                })
                .map(|c| c.severity)
                .unwrap_or(Severity::Major); // default to major if not classified

            triaged_anchors.push(TriagedAnchor {
                path: anchor.path.clone(),
                symbol: anchor.symbol.clone(),
                severity,
                provenance: anchor.provenance.clone(),
                diff: anchor.diff.clone(),
            });
        }

        let doc_severity = Severity::max_of(
            &triaged_anchors.iter().map(|a| a.severity).collect::<Vec<_>>(),
        );

        triaged_docs.push(TriagedDoc {
            doc: drifted_doc.doc.clone(),
            doc_repo: drifted_doc.doc_repo.clone(),
            severity: doc_severity,
            summary: summary.to_string(),
            anchors: triaged_anchors,
        });
    }

    TriagedReport {
        repo: drift_report.repo.clone(),
        git_ref: drift_report.git_ref.clone(),
        commit: drift_report.commit.clone(),
        drifted: triaged_docs,
    }
}

/// Full triage pipeline: build prompts, call AI, parse responses, apply.
pub async fn triage_drift_report(
    drift_report: &DriftReport,
    provider_name: &str,
    model: &str,
    doc_contents: &std::collections::HashMap<String, String>,
) -> Result<TriagedReport> {
    let mut all_classifications = Vec::new();
    let mut summaries = std::collections::HashMap::new();

    for drifted_doc in &drift_report.drifted {
        let doc_content = doc_contents
            .get(&drifted_doc.doc)
            .map(|s| s.as_str())
            .unwrap_or("[doc content not available]");

        let prompt = build_triage_prompt(drifted_doc, doc_content);
        let response = provider::classify(provider_name, model, &prompt).await?;
        let classifications = parse_triage_response(&response)?;

        // Extract summary from a second lightweight call or use the response
        summaries.insert(drifted_doc.doc.clone(), response.clone());
        all_classifications.extend(classifications);
    }

    // Build a combined summary per doc
    let mut triaged_docs = Vec::new();
    for drifted_doc in &drift_report.drifted {
        let doc_classifications: Vec<_> = all_classifications
            .iter()
            .filter(|c| {
                drifted_doc
                    .anchors
                    .iter()
                    .any(|a| a.path == c.path && a.symbol == c.symbol)
            })
            .cloned()
            .collect();

        let summary = summaries
            .get(&drifted_doc.doc)
            .cloned()
            .unwrap_or_default();

        let partial = apply_classifications(
            &DriftReport {
                repo: drift_report.repo.clone(),
                git_ref: drift_report.git_ref.clone(),
                commit: drift_report.commit.clone(),
                drifted: vec![drifted_doc.clone()],
                clean: vec![],
            },
            &doc_classifications,
            &summary,
        );

        triaged_docs.extend(partial.drifted);
    }

    Ok(TriagedReport {
        repo: drift_report.repo.clone(),
        git_ref: drift_report.git_ref.clone(),
        commit: drift_report.commit.clone(),
        drifted: triaged_docs,
    })
}
```

- [ ] **Step 5: Export from lib.rs**

Update `src/lib.rs`:

```rust
pub mod config;
pub mod detection;
pub mod frontmatter;
pub mod models;
pub mod triage;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test triage_test 2>&1`
Expected: All 3 tests pass

- [ ] **Step 7: Wire standalone `steer triage` command**

Update the `Command::Triage` arm in `src/main.rs`:

```rust
Command::Triage { report } => {
    let config = Config::from_file(&cli.config)?;

    // Read drift report from file or stdin
    let drift_json = if let Some(report_path) = report {
        std::fs::read_to_string(&report_path)?
    } else {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    };

    let drift_report: steer::models::DriftReport = serde_json::from_str(&drift_json)?;

    // Read doc contents for triage prompts
    let doc_contents: std::collections::HashMap<String, String> = drift_report
        .drifted
        .iter()
        .filter_map(|d| {
            std::fs::read_to_string(&d.doc)
                .ok()
                .map(|c| (d.doc.clone(), c))
        })
        .collect();

    let rt = tokio::runtime::Runtime::new()?;
    let triaged = rt.block_on(steer::triage::triage_drift_report(
        &drift_report,
        &config.triage.provider,
        &config.triage.model,
        &doc_contents,
    ))?;

    println!("{}", serde_json::to_string_pretty(&triaged)?);
}
```

This makes the pipeline composable: `steer check --report drift.json && steer triage --report drift.json`

- [ ] **Step 8: Commit**

```bash
git add src/triage/ src/lib.rs src/main.rs tests/triage_test.rs
git commit -m "feat: add triage layer with AI provider abstraction and severity classification"
```

---

### Task 9: Remediation Layer + Agent Invocation

**Files:**
- Create: `src/remediation/mod.rs`
- Create: `src/remediation/agent.rs`
- Create: `tests/remediation_test.rs`

- [ ] **Step 1: Write failing tests**

Write `tests/remediation_test.rs`:

```rust
use steer::models::*;
use steer::remediation;

#[test]
fn builds_agent_payload_from_triaged_doc() {
    let triaged_doc = TriagedDoc {
        doc: "steering/auth.md".to_string(),
        doc_repo: "git@example.com:docs.git".to_string(),
        severity: Severity::Major,
        summary: "Auth model changed to scope-based".to_string(),
        anchors: vec![TriagedAnchor {
            path: "src/Auth.java".to_string(),
            symbol: Some("Auth#validate".to_string()),
            severity: Severity::Major,
            provenance: "abc123".to_string(),
            diff: "+scopes param".to_string(),
        }],
    };

    let payload =
        remediation::build_agent_payload(&triaged_doc, "def456", false);

    assert_eq!(payload.action, "update_docs");
    assert_eq!(payload.severity, Severity::Major);
    assert!(!payload.auto_merge);
    assert_eq!(payload.target.repo, "git@example.com:docs.git");
    assert_eq!(payload.target.path, "steering/auth.md");
    assert!(payload.target.branch_prefix.starts_with("steer/auto-update"));
    assert_eq!(payload.drifted_anchors.len(), 1);
    assert!(payload.instructions.contains("def456"));
}

#[test]
fn filters_no_update_from_remediation() {
    let triaged = TriagedReport {
        repo: "test".to_string(),
        git_ref: "main".to_string(),
        commit: "abc".to_string(),
        drifted: vec![TriagedDoc {
            doc: "auth.md".to_string(),
            doc_repo: "git@example.com:docs.git".to_string(),
            severity: Severity::NoUpdate,
            summary: "No real changes".to_string(),
            anchors: vec![TriagedAnchor {
                path: "src/Auth.java".to_string(),
                symbol: None,
                severity: Severity::NoUpdate,
                provenance: "old".to_string(),
                diff: " ".to_string(),
            }],
        }],
    };

    let (to_remediate, to_sync) = remediation::partition_by_action(&triaged);
    assert!(to_remediate.is_empty());
    assert_eq!(to_sync.len(), 1);
}

#[test]
fn auto_merge_flag_set_for_configured_severities() {
    let auto_merge_severities = vec!["minor".to_string()];

    assert!(remediation::should_auto_merge(
        Severity::Minor,
        &auto_merge_severities
    ));
    assert!(!remediation::should_auto_merge(
        Severity::Major,
        &auto_merge_severities
    ));
    assert!(!remediation::should_auto_merge(
        Severity::NoUpdate,
        &auto_merge_severities
    ));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test remediation_test 2>&1`
Expected: FAIL — module not found

- [ ] **Step 3: Implement remediation module**

Write `src/remediation/agent.rs`:

```rust
use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Invoke an agent command, piping JSON payload via stdin.
/// Returns the agent's stdout output.
pub fn invoke_agent(command: &str, payload_json: &str) -> Result<String> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        anyhow::bail!("Empty agent command");
    }

    let mut child = Command::new(parts[0])
        .args(&parts[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to start agent command: {command}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload_json.as_bytes())
            .context("Failed to write to agent stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Agent command failed")?;

    if !output.status.success() {
        anyhow::bail!(
            "Agent command exited with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

Write `src/remediation/mod.rs`:

```rust
pub mod agent;

use crate::models::*;

/// Build an agent payload from a triaged doc.
pub fn build_agent_payload(
    doc: &TriagedDoc,
    current_commit: &str,
    auto_merge: bool,
) -> AgentPayload {
    AgentPayload {
        action: "update_docs".to_string(),
        severity: doc.severity,
        auto_merge,
        target: AgentTarget {
            repo: doc.doc_repo.clone(),
            branch_prefix: "steer/auto-update".to_string(),
            path: doc.doc.clone(),
        },
        drifted_anchors: doc
            .anchors
            .iter()
            .filter(|a| a.severity != Severity::NoUpdate)
            .map(|a| AgentAnchor {
                path: a.path.clone(),
                symbol: a.symbol.clone(),
                severity: a.severity,
                summary: doc.summary.clone(),
                diff: a.diff.clone(),
            })
            .collect(),
        instructions: format!(
            "Update the target document to reflect the code changes. \
             Stamp all modified anchors with provenance commit {current_commit}."
        ),
    }
}

/// Split triaged docs into those needing remediation vs. provenance-only sync.
/// Docs where ALL anchors are no_update go to sync. Others go to remediation.
pub fn partition_by_action(report: &TriagedReport) -> (Vec<&TriagedDoc>, Vec<&TriagedDoc>) {
    let mut to_remediate = Vec::new();
    let mut to_sync = Vec::new();

    for doc in &report.drifted {
        if doc.anchors.iter().all(|a| a.severity == Severity::NoUpdate) {
            to_sync.push(doc);
        } else {
            to_remediate.push(doc);
        }
    }

    (to_remediate, to_sync)
}

/// Check if a severity level should auto-merge based on config.
pub fn should_auto_merge(severity: Severity, auto_merge_severities: &[String]) -> bool {
    let severity_str = match severity {
        Severity::NoUpdate => "no_update",
        Severity::Minor => "minor",
        Severity::Major => "major",
    };
    auto_merge_severities.iter().any(|s| s == severity_str)
}
```

- [ ] **Step 4: Export from lib.rs**

Update `src/lib.rs`:

```rust
pub mod config;
pub mod detection;
pub mod frontmatter;
pub mod models;
pub mod remediation;
pub mod triage;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test remediation_test 2>&1`
Expected: All 3 tests pass

- [ ] **Step 6: Wire `steer update` into main.rs**

Update the `Command::Update` arm in `src/main.rs`:

```rust
Command::Update { report } => {
    let config = Config::from_file(&cli.config)?;
    let code_repo_path = std::env::current_dir()?;
    let repo_name = code_repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let docs_path = std::env::var("STEER_DOCS_PATH")
        .unwrap_or_else(|_| ".".to_string());

    // Step 1: Detection
    let drift_report = steer::detection::detect_drift(
        &code_repo_path,
        &docs_path,
        &config.repos.docs[0].url,
        &repo_name,
    )?;

    if drift_report.drifted.is_empty() {
        println!("No drift detected.");
        return Ok(());
    }

    if let Some(report_path) = &report {
        let json = serde_json::to_string_pretty(&drift_report)?;
        std::fs::write(report_path, &json)?;
    }

    // Step 2: Triage
    let rt = tokio::runtime::Runtime::new()?;
    let doc_contents: std::collections::HashMap<String, String> = drift_report
        .drifted
        .iter()
        .filter_map(|d| {
            std::fs::read_to_string(&d.doc)
                .ok()
                .map(|c| (d.doc.clone(), c))
        })
        .collect();

    let triaged = rt.block_on(steer::triage::triage_drift_report(
        &drift_report,
        &config.triage.provider,
        &config.triage.model,
        &doc_contents,
    ))?;

    // Step 3: Remediation
    let (to_remediate, to_sync) = steer::remediation::partition_by_action(&triaged);

    let mut summary = steer::models::RemediationSummary {
        remediated: Vec::new(),
        provenance_advanced: Vec::new(),
        errors: Vec::new(),
    };

    for doc in to_remediate {
        let auto_merge = steer::remediation::should_auto_merge(
            doc.severity,
            &config.remediation.auto_merge_severities,
        );
        let payload =
            steer::remediation::build_agent_payload(doc, &triaged.commit, auto_merge);
        let payload_json = serde_json::to_string_pretty(&payload)?;

        match steer::remediation::agent::invoke_agent(
            &config.remediation.agent_command,
            &payload_json,
        ) {
            Ok(output) => {
                summary.remediated.push(RemediatedDoc {
                    doc: doc.doc.clone(),
                    repo: doc.doc_repo.clone(),
                    mr_url: extract_mr_url(&output),
                    severity: doc.severity,
                    auto_merged: auto_merge,
                });
            }
            Err(e) => {
                summary.errors.push(format!("{}: {e}", doc.doc));
            }
        }
    }

    for doc in to_sync {
        summary.provenance_advanced.push(ProvenanceSynced {
            doc: doc.doc.clone(),
            anchors_synced: doc.anchors.len(),
            reason: "no_update — code changes did not affect documentation accuracy"
                .to_string(),
        });
    }

    println!("{}", serde_json::to_string_pretty(&summary)?);
}
```

Add this helper function in main.rs:

```rust
fn extract_mr_url(output: &str) -> Option<String> {
    output
        .lines()
        .find(|l| l.contains("merge_request") || l.contains("pull/") || l.starts_with("http"))
        .map(|l| l.trim().to_string())
}
```

- [ ] **Step 7: Commit**

```bash
git add src/remediation/ src/lib.rs src/main.rs tests/remediation_test.rs
git commit -m "feat: add remediation layer with agent invocation and steer update pipeline"
```

---

### Task 10: Doc Repo Cache + `steer install`

**Files:**
- Create: `src/install/mod.rs`
- Create: `src/install/repo_cache.rs`
- Create: `tests/install_test.rs`

- [ ] **Step 1: Write failing tests**

Write `tests/install_test.rs`:

```rust
use steer::install;
use tempfile::TempDir;

#[test]
fn copies_steering_files_in_workspace_mode() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();

    // Create source structure: group + shared + _steer
    let group_dir = source_dir.path().join("payments-platform");
    let shared_dir = source_dir.path().join("shared");
    let meta_dir = source_dir.path().join("_steer");
    std::fs::create_dir_all(&group_dir).unwrap();
    std::fs::create_dir_all(&shared_dir).unwrap();
    std::fs::create_dir_all(&meta_dir).unwrap();

    std::fs::write(group_dir.join("auth.md"), "# Auth").unwrap();
    std::fs::write(shared_dir.join("conventions.md"), "# Conventions").unwrap();
    std::fs::write(meta_dir.join("AGENTS.md"), "# Steer instructions").unwrap();
    std::fs::write(meta_dir.join("skill.md"), "# Skill").unwrap();

    let target = workspace_dir.path().join(".kiro/steering");

    install::install_to_workspace(
        source_dir.path(),
        &target,
        Some("payments-platform"),
        Some("AGENTS.md"),
        None,
    )
    .unwrap();

    // Group files copied
    assert!(target.join("auth.md").exists());
    // Shared files copied
    assert!(target.join("conventions.md").exists());
    // AGENTS.md copied
    assert!(target.join("AGENTS.md").exists());
}

#[test]
fn filters_by_group() {
    let source_dir = TempDir::new().unwrap();
    let workspace_dir = TempDir::new().unwrap();

    let group_a = source_dir.path().join("team-a");
    let group_b = source_dir.path().join("team-b");
    std::fs::create_dir_all(&group_a).unwrap();
    std::fs::create_dir_all(&group_b).unwrap();

    std::fs::write(group_a.join("a.md"), "# A").unwrap();
    std::fs::write(group_b.join("b.md"), "# B").unwrap();

    let target = workspace_dir.path().join(".kiro/steering");

    install::install_to_workspace(
        source_dir.path(),
        &target,
        Some("team-a"),
        None,
        None,
    )
    .unwrap();

    assert!(target.join("a.md").exists());
    assert!(!target.join("b.md").exists());
}

#[test]
fn creates_symlinks_in_link_mode() {
    let source_dir = TempDir::new().unwrap();
    let link_dir = TempDir::new().unwrap();

    let group = source_dir.path().join("my-group");
    std::fs::create_dir_all(&group).unwrap();
    std::fs::write(group.join("doc.md"), "# Doc").unwrap();

    let target = link_dir.path().join("steering");

    install::install_as_links(
        source_dir.path(),
        &target,
        Some("my-group"),
        None,
        None,
    )
    .unwrap();

    let link = target.join("doc.md");
    assert!(link.exists());
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test install_test 2>&1`
Expected: FAIL — module not found

- [ ] **Step 3: Implement repo cache**

Write `src/install/repo_cache.rs`:

```rust
use anyhow::{Context, Result};
use sha2::Digest;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get or create a cached clone of a git repo.
/// Cache lives in ~/.cache/steer/repos/<hash>/
pub fn get_or_clone(repo_url: &str, git_ref: &str) -> Result<PathBuf> {
    let cache_dir = cache_dir_for(repo_url)?;

    if cache_dir.join(".git").exists() {
        // Pull latest
        Command::new("git")
            .args(["fetch", "origin", git_ref])
            .current_dir(&cache_dir)
            .output()
            .context("Failed to fetch")?;
        Command::new("git")
            .args(["checkout", &format!("origin/{git_ref}")])
            .current_dir(&cache_dir)
            .output()
            .context("Failed to checkout")?;
    } else {
        std::fs::create_dir_all(&cache_dir)?;
        Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--branch",
                git_ref,
                repo_url,
                &cache_dir.to_string_lossy(),
            ])
            .output()
            .with_context(|| format!("Failed to clone {repo_url}"))?;
    }

    Ok(cache_dir)
}

/// Check if the cached repo is up to date with remote HEAD.
pub fn is_up_to_date(repo_url: &str, git_ref: &str) -> Result<bool> {
    let cache_dir = cache_dir_for(repo_url)?;
    if !cache_dir.join(".git").exists() {
        return Ok(false);
    }

    let local = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&cache_dir)
        .output()?;
    let local_sha = String::from_utf8_lossy(&local.stdout).trim().to_string();

    let remote = Command::new("git")
        .args(["ls-remote", repo_url, git_ref])
        .output()?;
    let remote_line = String::from_utf8_lossy(&remote.stdout);
    let remote_sha = remote_line
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();

    Ok(local_sha == remote_sha)
}

fn cache_dir_for(repo_url: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let hash = format!("{:x}", sha2::Sha256::digest(repo_url.as_bytes()));
    let short_hash = &hash[..12];
    Ok(home.join(".cache/steer/repos").join(short_hash))
}
```

- [ ] **Step 4: Implement install module**

Write `src/install/mod.rs`:

```rust
pub mod repo_cache;

use anyhow::{Context, Result};
use std::path::Path;

/// Copy steering files to a workspace directory (CI/ephemeral mode).
pub fn install_to_workspace(
    source_dir: &Path,
    target_dir: &Path,
    group: Option<&str>,
    agents_file: Option<&str>,
    skill_dir: Option<&str>,
) -> Result<()> {
    std::fs::create_dir_all(target_dir)
        .with_context(|| format!("Failed to create {}", target_dir.display()))?;

    // Copy group-specific files
    if let Some(group_name) = group {
        let group_dir = source_dir.join(group_name);
        if group_dir.is_dir() {
            copy_md_files(&group_dir, target_dir)?;
        }
    }

    // Always copy shared/
    let shared_dir = source_dir.join("shared");
    if shared_dir.is_dir() {
        copy_md_files(&shared_dir, target_dir)?;
    }

    // Copy _steer meta files
    let meta_dir = source_dir.join("_steer");
    if meta_dir.is_dir() {
        if let Some(af) = agents_file {
            let src = meta_dir.join("AGENTS.md");
            if src.exists() {
                std::fs::copy(&src, target_dir.join(af))?;
            }
        }
        if let Some(sd) = skill_dir {
            let skill_target = Path::new(sd);
            std::fs::create_dir_all(skill_target).ok();
            let src = meta_dir.join("skill.md");
            if src.exists() {
                std::fs::copy(&src, skill_target.join("steer-check.md")).ok();
            }
        }
    }

    Ok(())
}

/// Symlink steering files to a global directory (developer machine mode).
pub fn install_as_links(
    source_dir: &Path,
    target_dir: &Path,
    group: Option<&str>,
    agents_file: Option<&str>,
    skill_dir: Option<&str>,
) -> Result<()> {
    std::fs::create_dir_all(target_dir)
        .with_context(|| format!("Failed to create {}", target_dir.display()))?;

    // Symlink group-specific files
    if let Some(group_name) = group {
        let group_dir = source_dir.join(group_name);
        if group_dir.is_dir() {
            symlink_md_files(&group_dir, target_dir)?;
        }
    }

    // Always symlink shared/
    let shared_dir = source_dir.join("shared");
    if shared_dir.is_dir() {
        symlink_md_files(&shared_dir, target_dir)?;
    }

    // Symlink _steer meta files
    let meta_dir = source_dir.join("_steer");
    if meta_dir.is_dir() {
        if let Some(af) = agents_file {
            let src = meta_dir.join("AGENTS.md");
            if src.exists() {
                let dest = target_dir.join(af);
                if dest.exists() {
                    std::fs::remove_file(&dest).ok();
                }
                std::os::unix::fs::symlink(&src, &dest)?;
            }
        }
    }

    Ok(())
}

/// Add a directory to .git/info/exclude so it doesn't show as untracked.
pub fn add_to_git_exclude(workspace_root: &Path, dir_to_exclude: &str) -> Result<()> {
    let exclude_file = workspace_root.join(".git/info/exclude");
    if let Ok(content) = std::fs::read_to_string(&exclude_file) {
        if content.contains(dir_to_exclude) {
            return Ok(()); // already excluded
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&exclude_file)?;
    use std::io::Write;
    writeln!(file, "{dir_to_exclude}")?;
    Ok(())
}

fn copy_md_files(from: &Path, to: &Path) -> Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let dest = to.join(path.file_name().unwrap());
            std::fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}

fn symlink_md_files(from: &Path, to: &Path) -> Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let dest = to.join(path.file_name().unwrap());
            if dest.exists() {
                std::fs::remove_file(&dest).ok();
            }
            std::os::unix::fs::symlink(
                std::fs::canonicalize(&path)?,
                &dest,
            )?;
        }
    }
    Ok(())
}
```

- [ ] **Step 5: Wire `steer install` into main.rs**

Update the `Command::Install` arm in `src/main.rs`:

```rust
Command::Install {
    group,
    agent,
    link,
    workspace,
    check,
} => {
    let config = Config::from_file(&cli.config)?;

    // Determine install mode
    let use_workspace = workspace
        || (!link
            && (std::env::var("CI").is_ok()
                || std::env::var("GITLAB_CI").is_ok()
                || std::env::var("GITHUB_ACTIONS").is_ok()));

    // Clone/cache the docs repo
    let doc_repo = &config.repos.docs[0];

    if check {
        if steer::install::repo_cache::is_up_to_date(&doc_repo.url, &doc_repo.git_ref)? {
            eprintln!("Steering files are up to date.");
            return Ok(());
        }
    }

    let cached_repo =
        steer::install::repo_cache::get_or_clone(&doc_repo.url, &doc_repo.git_ref)?;
    let source_dir = cached_repo.join(&doc_repo.path);

    // Select agent platforms
    let platforms: Vec<_> = if let Some(agent_name) = &agent {
        config
            .agents
            .iter()
            .filter(|a| a.name == *agent_name)
            .collect()
    } else {
        config.agents.iter().collect()
    };

    if platforms.is_empty() {
        anyhow::bail!("No matching agent platform found");
    }

    for platform in &platforms {
        let group_ref = group.as_deref();

        if use_workspace {
            let target = std::path::PathBuf::from(&platform.workspace_steering);
            steer::install::install_to_workspace(
                &source_dir,
                &target,
                group_ref,
                Some(&platform.agents_file),
                if platform.skill_dir.is_empty() {
                    None
                } else {
                    Some(&platform.skill_dir)
                },
            )?;
            // Add to git exclude
            let cwd = std::env::current_dir()?;
            steer::install::add_to_git_exclude(&cwd, &platform.workspace_steering)?;
            eprintln!(
                "Installed steering to {} (workspace mode, platform: {})",
                platform.workspace_steering, platform.name
            );
        } else {
            let target = shellexpand::tilde(&platform.global_steering);
            let target_path = std::path::PathBuf::from(target.as_ref());
            steer::install::install_as_links(
                &source_dir,
                &target_path,
                group_ref,
                Some(&platform.agents_file),
                if platform.skill_dir.is_empty() {
                    None
                } else {
                    Some(&platform.skill_dir)
                },
            )?;
            eprintln!(
                "Installed steering to {} (link mode, platform: {})",
                platform.global_steering, platform.name
            );
        }
    }
}
```

Add `shellexpand` to `Cargo.toml` dependencies:

```toml
shellexpand = "3"
```

- [ ] **Step 6: Export from lib.rs**

Update `src/lib.rs`:

```rust
pub mod config;
pub mod detection;
pub mod frontmatter;
pub mod install;
pub mod models;
pub mod remediation;
pub mod triage;
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test --test install_test 2>&1`
Expected: All 3 tests pass

- [ ] **Step 8: Commit**

```bash
git add src/install/ src/lib.rs src/main.rs Cargo.toml tests/install_test.rs
git commit -m "feat: add steer install with workspace/link modes and doc repo caching"
```

---

### Task 11: Utility Commands — `steer init`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement `steer init`**

This command creates a `steer.toml` template and scans for existing doc files.

Update the `Command::Init` arm in `src/main.rs`:

```rust
Command::Init => {
    let config_path = std::path::Path::new("steer.toml");
    if config_path.exists() {
        eprintln!("steer.toml already exists. Skipping.");
        return Ok(());
    }

    let template = r#"[detection]
languages = ["java", "go", "typescript", "xml"]
fallback = "content-hash"

[triage]
provider = "anthropic"
model = "claude-haiku-4-5-20251001"
severity_levels = ["no_update", "minor", "major"]

[remediation]
agent_command = "echo 'Configure your agent command here'"
auto_merge_severities = []

[[repos.docs]]
url = "git@your-host:your-org/docs.git"
path = "steering/"
ref = "main"

[[agents]]
name = "kiro"
global_steering = "~/.kiro/steering/"
workspace_steering = ".kiro/steering/"
agents_file = "AGENTS.md"
skill_dir = ".kiro/skills/"
"#;

    std::fs::write(config_path, template)?;
    println!("Created steer.toml — edit it with your repo URLs and agent config.");
}
```

- [ ] **Step 2: Verify it works**

Run: `cd /tmp && mkdir steer-test && cd steer-test && cargo run --manifest-path /home/daniel/Development/steer/Cargo.toml -- init`
Expected: Creates `steer.toml` with template content

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement steer init command"
```

---

### Task 12: Utility Commands — `steer link`, `steer status`, `steer sync`

**Files:**
- Modify: `src/main.rs`
- Modify: `src/frontmatter.rs`

- [ ] **Step 1: Add frontmatter update helper**

Add to `src/frontmatter.rs`:

```rust
/// Update the provenance SHA for a specific anchor in a markdown file.
/// Rewrites the file in place, preserving all non-steer frontmatter.
pub fn update_provenance(
    path: &Path,
    anchor_path: &str,
    anchor_symbol: Option<&str>,
    new_provenance: &str,
) -> Result<(), anyhow::Error> {
    let text = std::fs::read_to_string(path)?;
    let (raw_fm, body) = extract_frontmatter(&text)
        .ok_or_else(|| anyhow::anyhow!("No frontmatter in {}", path.display()))?;

    let mut yaml: serde_yaml::Value = serde_yaml::from_str(raw_fm)?;

    if let Some(steer) = yaml.get_mut("steer") {
        if let Some(anchors) = steer.get_mut("anchors") {
            if let Some(anchors_seq) = anchors.as_sequence_mut() {
                for anchor in anchors_seq.iter_mut() {
                    let p = anchor.get("path").and_then(|v| v.as_str());
                    let s = anchor.get("symbol").and_then(|v| v.as_str());
                    if p == Some(anchor_path) && s == anchor_symbol {
                        anchor["provenance"] =
                            serde_yaml::Value::String(new_provenance.to_string());
                    }
                }
            }
        }
    }

    let new_fm = serde_yaml::to_string(&yaml)?;
    let new_content = format!("---\n{}---\n\n{body}", new_fm);
    std::fs::write(path, new_content)?;

    Ok(())
}
```

Note: you'll need to add `use anyhow::Result;` at the top if not already imported.

- [ ] **Step 2: Implement remaining commands in main.rs**

Update `Command::Link`:

```rust
Command::Link { files } => {
    let code_repo_path = std::env::current_dir()?;
    let head = steer::detection::git::head_sha(&code_repo_path)?;

    let targets: Vec<_> = if files.is_empty() {
        // Find all md files with steer frontmatter in current dir
        glob::glob("**/*.md")?
            .filter_map(|e| e.ok())
            .collect()
    } else {
        files
    };

    for file in &targets {
        if let Some(doc) = steer::frontmatter::parse_doc_file(file, "") {
            for anchor in &doc.frontmatter.anchors {
                steer::frontmatter::update_provenance(
                    file,
                    &anchor.path,
                    anchor.symbol.as_deref(),
                    &head,
                )?;
            }
            println!("Linked {} ({} anchors stamped at {})",
                file.display(), doc.frontmatter.anchors.len(), &head[..8]);
        }
    }
}
```

Update `Command::Status`:

```rust
Command::Status => {
    let config = Config::from_file(&cli.config)?;
    let code_repo_path = std::env::current_dir()?;

    let docs_path = std::env::var("STEER_DOCS_PATH")
        .unwrap_or_else(|_| ".".to_string());

    let docs = steer::frontmatter::scan_docs(
        std::path::Path::new(&docs_path),
        &config.repos.docs[0].url,
        None,
    );

    if docs.is_empty() {
        println!("No steering files with steer anchors found.");
        return Ok(());
    }

    for doc in &docs {
        println!("\n{}", doc.path);
        if let Some(group) = &doc.frontmatter.group {
            println!("  group: {group}");
        }
        for anchor in &doc.frontmatter.anchors {
            let status = match steer::detection::git::read_file_at_rev(
                &code_repo_path,
                &anchor.provenance,
                &anchor.path,
            ) {
                Ok(_) => "ok",
                Err(_) => "provenance not found",
            };
            println!(
                "  {} {}  @{}  [{}]",
                anchor.path,
                anchor.symbol.as_deref().unwrap_or("(whole file)"),
                &anchor.provenance[..std::cmp::min(8, anchor.provenance.len())],
                status
            );
        }
    }
}
```

Update `Command::Sync`:

```rust
Command::Sync { files } => {
    let code_repo_path = std::env::current_dir()?;
    let head = steer::detection::git::head_sha(&code_repo_path)?;

    let targets: Vec<_> = if files.is_empty() {
        glob::glob("**/*.md")?
            .filter_map(|e| e.ok())
            .collect()
    } else {
        files
    };

    let mut synced = 0;
    for file in &targets {
        if let Some(doc) = steer::frontmatter::parse_doc_file(file, "") {
            for anchor in &doc.frontmatter.anchors {
                steer::frontmatter::update_provenance(
                    file,
                    &anchor.path,
                    anchor.symbol.as_deref(),
                    &head,
                )?;
                synced += 1;
            }
        }
    }

    println!("Synced {synced} anchors to provenance {}", &head[..8]);
}
```

- [ ] **Step 3: Verify commands compile and basic usage works**

Run: `cargo build 2>&1`
Expected: Compiles successfully

Run: `cargo run -- --help`
Expected: All commands listed

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/frontmatter.rs
git commit -m "feat: implement steer link, status, and sync commands"
```

---

### Task 13: Integration Test — Full Pipeline

**Files:**
- Create: `tests/integration_test.rs`

- [ ] **Step 1: Write end-to-end integration test**

Write `tests/integration_test.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::process;
use tempfile::TempDir;

/// Full pipeline test: set up a code repo and docs, change code, run steer check.
#[test]
fn steer_check_detects_drift_end_to_end() {
    // 1. Create a "code repo" with a Java file
    let code_dir = TempDir::new().unwrap();
    let code_path = code_dir.path();

    process::Command::new("git")
        .args(["init"])
        .current_dir(code_path)
        .output()
        .unwrap();
    process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(code_path)
        .output()
        .unwrap();
    process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(code_path)
        .output()
        .unwrap();

    std::fs::create_dir_all(code_path.join("src/auth")).unwrap();
    std::fs::write(
        code_path.join("src/auth/Auth.java"),
        "public class Auth { public boolean check(String t) { return true; } }",
    )
    .unwrap();
    process::Command::new("git")
        .args(["add", "."])
        .current_dir(code_path)
        .output()
        .unwrap();
    process::Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(code_path)
        .output()
        .unwrap();

    let sha_output = process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(code_path)
        .output()
        .unwrap();
    let sha = String::from_utf8(sha_output.stdout)
        .unwrap()
        .trim()
        .to_string();

    // 2. Create a "docs" directory with steering file
    let docs_dir = TempDir::new().unwrap();
    std::fs::create_dir_all(docs_dir.path().join("test-group")).unwrap();
    let steering_content = format!(
        r#"---
steer:
  group: test-group
  anchors:
    - repo: "file://{code}"
      path: src/auth/Auth.java
      provenance: {sha}
---

# Auth Architecture
Token validation returns boolean.
"#,
        code = code_path.display(),
        sha = sha,
    );
    std::fs::write(
        docs_dir.path().join("test-group/auth.md"),
        &steering_content,
    )
    .unwrap();

    // 3. Create steer.toml in code repo
    let config = format!(
        r#"[detection]
languages = ["java"]
fallback = "content-hash"

[triage]
provider = "anthropic"
model = "claude-haiku-4-5-20251001"
severity_levels = ["no_update", "minor", "major"]

[remediation]
agent_command = "echo done"
auto_merge_severities = []

[[repos.docs]]
url = "file://{code}"
path = ""
ref = "main"
"#,
        code = code_path.display()
    );
    std::fs::write(code_path.join("steer.toml"), &config).unwrap();

    // 4. Check with no changes — should exit 0
    Command::cargo_bin("steer")
        .unwrap()
        .args(["check", "--config", "steer.toml"])
        .current_dir(code_path)
        .env("STEER_DOCS_PATH", docs_dir.path().join("test-group"))
        .assert()
        .success()
        .stdout(predicate::str::contains("\"drifted\": []"));

    // 5. Now change the code
    std::fs::write(
        code_path.join("src/auth/Auth.java"),
        "public class Auth { public boolean check(String t, List<String> scopes) { return true; } }",
    )
    .unwrap();
    process::Command::new("git")
        .args(["add", "."])
        .current_dir(code_path)
        .output()
        .unwrap();
    process::Command::new("git")
        .args(["commit", "-m", "add scopes"])
        .current_dir(code_path)
        .output()
        .unwrap();

    // 6. Check again — should exit 1 (drift found)
    Command::cargo_bin("steer")
        .unwrap()
        .args(["check", "--config", "steer.toml"])
        .current_dir(code_path)
        .env("STEER_DOCS_PATH", docs_dir.path().join("test-group"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("src/auth/Auth.java"));
}
```

- [ ] **Step 2: Run integration test**

Run: `cargo test --test integration_test 2>&1`
Expected: Test passes

- [ ] **Step 3: Commit**

```bash
git add tests/integration_test.rs
git commit -m "test: add end-to-end integration test for steer check pipeline"
```

---

## Summary

| Task | Delivers | Key Files |
|------|----------|-----------|
| 1 | CLI scaffold with all subcommands | `Cargo.toml`, `src/main.rs`, `src/cli.rs` |
| 2 | Config parsing | `src/config.rs` |
| 3 | Shared data models | `src/models.rs` |
| 4 | Frontmatter parser | `src/frontmatter.rs` |
| 5 | AST fingerprinting + content hash | `src/detection/fingerprint.rs` |
| 6 | Git provenance lookups | `src/detection/git.rs` |
| 7 | Detection pipeline + `steer check` | `src/detection/mod.rs` |
| 8 | Triage layer + AI provider | `src/triage/` |
| 9 | Remediation + agent invocation + `steer update` | `src/remediation/` |
| 10 | Doc repo cache + `steer install` | `src/install/` |
| 11 | `steer init` | `src/main.rs` |
| 12 | `steer link`, `steer status`, `steer sync` | `src/main.rs`, `src/frontmatter.rs` |
| 13 | End-to-end integration test | `tests/integration_test.rs` |

After all tasks: the CLI can detect drift, classify severity, invoke an agent for remediation, and install steering files to any configured agent platform.
