# Architecture

kedge is a Rust CLI built around a three-layer pipeline: detection, triage, and remediation. Each layer has clear inputs and outputs, and the layers compose into the full `kedge update` pipeline.

## Module map

```
src/
├── main.rs              Command routing and orchestration
├── lib.rs               Library exports (re-exports all modules)
├── cli.rs               Clap command definitions
├── config.rs            kedge.toml parsing (Config, DetectionConfig, TriageConfig, etc.)
├── models.rs            All data types (Anchor, DriftReport, TriagedReport, AgentPayload, etc.)
├── frontmatter.rs       YAML frontmatter parsing and provenance updates
├── output.rs            Agent output parsing (JSON and URL scraping)
├── safety.rs            Input validation (provenance, paths, URLs, git refs)
├── detection/
│   ├── mod.rs           detect_drift() — orchestrates anchor scanning and comparison
│   ├── fingerprint.rs   AST fingerprinting via tree-sitter; content-hash fallback
│   └── git.rs           Git CLI operations (read_file_at_rev, diff_with_summary, head_sha)
├── triage/
│   ├── mod.rs           Prompt building, response parsing, classification application
│   └── provider.rs      API integration (Anthropic, OpenAI, command)
├── remediation/
│   ├── mod.rs           Payload construction, partition_by_action, auto-merge logic
│   └── agent.rs         Agent process spawning with timeout and stdin/stdout handling
└── install/
    ├── mod.rs           Steering file installation (copy/symlink), gitignore management
    └── repo_cache.rs    Doc repo cloning and caching (~/.cache/kedge/repos/)
```

## Data flow

```
                    kedge update
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
    ┌─────────┐    ┌──────────┐    ┌─────────────┐
    │Detection│───►│  Triage  │───►│Remediation  │
    └─────────┘    └──────────┘    └─────────────┘
         │               │               │
    DriftReport    TriagedReport    RemediationSummary
```

### Detection → DriftReport

**Input:** Code repo path, docs directory, repo URL.

**Process:**
1. `frontmatter::scan_docs()` discovers steering files
2. For each anchor, `fingerprint::compute_sig()` computes the current fingerprint
3. Compare against stored provenance (direct comparison for `sig:`, git history for legacy SHA)
4. Partition docs into `drifted` and `clean`

**Output:** `DriftReport` with `repo`, `ref`, `commit`, `drifted[]`, `clean[]`.

### Triage → TriagedReport

**Input:** `DriftReport`, `TriageConfig`, doc contents.

**Process:**
1. For each drifted doc, `build_triage_prompt()` constructs the AI prompt
2. `provider::classify()` dispatches to the configured backend (Anthropic/OpenAI/command)
3. `parse_triage_response()` extracts `{ path, symbol, severity }` classifications from the response
4. `apply_classifications()` maps classifications onto the drift report anchors

**Output:** `TriagedReport` with classified anchors and doc-level severity (max of anchor severities).

### Remediation → RemediationSummary

**Input:** `TriagedReport`, `RemediationConfig`.

**Process:**
1. `partition_by_action()` splits docs: those needing agent remediation vs those needing only provenance sync
2. For remediable docs: build `AgentPayload` or `BatchAgentPayload`, invoke agent via `agent::invoke_agent()`
3. For sync-only docs: recompute fingerprints and update provenance in steering files
4. Parse agent output for MR URLs

**Output:** `RemediationSummary` with `remediated[]`, `provenance_advanced[]`, `errors[]`.

## Key types

| Type | Module | Purpose |
|------|--------|---------|
| `Anchor` | models | A code location reference in a steering file |
| `DocFile` | models | Parsed steering file with frontmatter and content |
| `DriftReport` | models | Detection output: drifted and clean docs |
| `DriftedAnchor` | models | An anchor whose code has changed |
| `Severity` | models | Enum: `NoUpdate`, `Minor`, `Major` |
| `TriagedReport` | models | Triage output: classified anchors |
| `AgentPayload` | models | JSON sent to agent (per-doc mode) |
| `BatchAgentPayload` | models | JSON sent to agent (batch mode) |
| `RemediationSummary` | models | Final pipeline output |
| `Config` | config | Parsed kedge.toml |

## Design decisions

### No AI in detection

Detection is fully deterministic. AST fingerprinting uses tree-sitter (compiled C grammars) with no AI inference. This keeps detection fast, free, and auditable.

### Tree-sitter for AST parsing

Tree-sitter grammars are fast (native C), incremental, and available for most languages. Each grammar is a Cargo dependency, so adding a language is just adding a crate.

### Agent agnostic

Remediation delegates to an external process via stdin/stdout. kedge doesn't know or care what the agent is -- it could be Kiro, Claude Code, a shell script, or any other tool. This makes kedge composable with any AI coding agent.

### Git CLI over libgit2

kedge shells out to `git` for operations like `show`, `diff`, and `rev-parse`. This avoids the complexity of libgit2 bindings, works with any git version the user has installed, and handles authentication (SSH keys, credential helpers) without kedge needing to know about them.

### Single binary, no runtime dependencies

The binary is statically linked with rustls (no OpenSSL). The only runtime requirement is `git` on PATH.
