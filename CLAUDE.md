# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # Debug build
cargo test                           # All tests (45 tests, ~100ms)
cargo test --test fingerprint_test   # Single test file
cargo test --test detection_test -- detect_drift  # Single test function
cargo clippy                         # Lint
cargo fmt                            # Format
cargo run -- <command>               # Run CLI (init, check, triage, update, status, sync, link, install)
```

## Architecture

Kedge is a documentation drift detection CLI. It detects when code changes make documentation stale, classifies severity via AI, and invokes agents to fix docs.

### Three-Layer Pipeline

**Detection** (`src/detection/`) — Deterministic, no AI. Compares AST fingerprints of code at provenance vs HEAD to find drift. Outputs `DriftReport` (JSON).

**Triage** (`src/triage/`) — Lightweight AI (Claude Haiku). Classifies each drifted anchor as `no_update`, `minor`, or `major`. Only runs on drifted docs (cost control).

**Remediation** (`src/remediation/`) — Invokes external agent process via stdin/stdout JSON. Agents update docs and open MRs. `no_update` anchors get provenance advanced without agent invocation.

### Key Modules

- `src/cli.rs` — Clap command definitions
- `src/main.rs` — Command routing and orchestration
- `src/models.rs` — All data types (`Anchor`, `DriftReport`, `TriagedReport`, `AgentPayload`, `Severity`)
- `src/config.rs` — `kedge.toml` parsing (detection, triage, remediation, repos, agents sections). `DetectionConfig` includes `exclude_dirs` with defaults. `DocRepo` includes `remote_name` (defaults to `"origin"`).
- `src/frontmatter.rs` — YAML frontmatter parsing from `.md` files; extracts `kedge:` block; updates provenance in-place. `scan_docs` accepts `exclude_dirs` to skip directories during recursive `.md` scanning.
- `src/detection/fingerprint.rs` — AST fingerprinting via tree-sitter (Java, Go, TypeScript/TSX, XML, Python, Rust) with content-hash fallback
- `src/detection/git.rs` — Git operations (read file at rev, diff, HEAD SHA)
- `src/triage/provider.rs` — Anthropic API integration
- `src/remediation/agent.rs` — Agent process spawning
- `src/install/` — Copies/symlinks steering files from docs repo to agent workspace dirs; supports `--recursive` for nested subdirectories. `repo_cache.rs` manages `~/.cache/kedge/repos/` and accepts a configurable `remote_name` for fetch operations.

### Fingerprinting

AST-based fingerprints ignore whitespace and comments. Symbol-scoped hashing (`ClassName#methodName`) tracks specific declarations. Output format: `sig:<16-hex-chars>`. Fallback: SHA-256 content hash for unsupported languages.

### Provenance

Two modes: legacy git SHA (compares fingerprints at two revisions) and content-addressed `sig:` prefix (compares current fingerprint to stored sig directly). `kedge link` and `kedge sync` stamp current signatures.

### Two-Repo Model

Code and docs live in separate repositories. The docs repo contains `.md` files with `kedge:` frontmatter anchors pointing to code locations (any markdown works — standalone docs, `AGENTS.md`, `CLAUDE.md`, etc.). The code repo contains `kedge.toml` config. Multiple `[[repos.docs]]` entries are supported — `check` and `update` scan all repos and merge results. `kedge install` copies/symlinks doc files from docs repo into agent directories (e.g., `.kiro/steering/`, `docs/`); pass `--recursive` to include nested subdirectories.

## Test Structure

All tests in `tests/` (no `#[test]` in src). Uses `tempfile::TempDir` for filesystem tests and real git repos via `git init`. Fixtures in `tests/fixtures/` include sample steering files, Java/Go/TypeScript/XML/Python/Rust source, a full Java project (`java-project/`), and a separate docs repo fixture (`java-project-docs/`).

## Adding a New Language

1. Add variant to `Language` enum in `src/detection/fingerprint.rs`
2. Map file extension in `Language::from_extension()`
3. Add tree-sitter grammar dependency to `Cargo.toml`
4. Implement `tree_sitter_language()` match arm
5. Add `is_comment()` node kinds if different from defaults
6. Add fingerprint tests in `tests/fingerprint_test.rs`
