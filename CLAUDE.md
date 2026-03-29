# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # Debug build
cargo test                           # All tests (234 across 17 suites, <3s)
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
- `src/main.rs` — Command routing and orchestration; `DocsSource` struct resolves docs repo URL, scan dir, and repo root for each `[[repos.docs]]` entry
- `src/models.rs` — All data types (`Anchor`, `DriftReport`, `TriagedReport`, `AgentPayload`, `Severity`)
- `src/config.rs` — `kedge.toml` parsing (detection, triage, remediation, repos, agents sections)
- `src/frontmatter.rs` — YAML frontmatter parsing from `.md` files; extracts `kedge:` block; updates provenance in-place
- `src/detection/fingerprint.rs` — AST fingerprinting via tree-sitter (Java, Go, TypeScript/TSX, XML, Python, Rust) with content-hash fallback
- `src/detection/mod.rs` — `detect_drift` accepts `code_repo_url` (for anchor matching), `doc_repo_url` (for agent payloads), and `doc_repo_root` (for relative paths)
- `src/detection/git.rs` — Git operations (read file at rev, diff, HEAD SHA)
- `src/triage/provider.rs` — Anthropic API integration
- `src/remediation/agent.rs` — Agent process spawning
- `src/safety.rs` — Path traversal validation (`validate_path_within`), provenance format validation, URL sanitization
- `src/install/` — Copies/symlinks steering files from docs repo to agent workspace dirs; `repo_cache.rs` manages `~/.cache/kedge/repos/` (keyed by URL + ref)

### Fingerprinting

AST-based fingerprints ignore whitespace and comments. Symbol-scoped hashing (`ClassName#methodName`) tracks specific declarations. Output format: `sig:<16-hex-chars>`. Fallback: SHA-256 content hash for unsupported languages.

### Provenance

Two modes: legacy git SHA (compares fingerprints at two revisions) and content-addressed `sig:` prefix (compares current fingerprint to stored sig directly). `kedge link` and `kedge sync` stamp current signatures.

### Two-Repo Model

Code and docs live in separate repositories. All kedge commands run from the code repo root. The docs repo contains `.md` files with `kedge:` frontmatter anchors pointing to code locations. The code repo contains `kedge.toml`. kedge auto-clones docs from `[[repos.docs]]` in the config; CI pipelines only need the code repo checked out. `kedge install` copies/symlinks doc files from docs repo into agent directories (e.g., `.kiro/steering/`, `docs/`).

Agent payloads contain `target.repo` (docs repo git URL from config) and `target.path` (repo-relative path). Env var `KEDGE_DOCS_REPO_URL` overrides the docs URL when using `KEDGE_DOCS_PATH`.

## Test Structure

All tests in `tests/` (no `#[test]` in src). Uses `tempfile::TempDir` for filesystem tests and real git repos via `git init`. Fixtures in `tests/fixtures/` include sample steering files, Java/Go/TypeScript/XML/Python/Rust source, a full Java project (`java-project/`), and a separate docs repo fixture (`java-project-docs/`).

## Adding a New Language

1. Add variant to `Language` enum in `src/detection/fingerprint.rs`
2. Map file extension in `Language::from_extension()`
3. Add tree-sitter grammar dependency to `Cargo.toml`
4. Implement `tree_sitter_language()` match arm
5. Add `is_comment()` node kinds if different from defaults
6. Add fingerprint tests in `tests/fingerprint_test.rs`
