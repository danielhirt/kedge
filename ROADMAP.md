# Roadmap

Planned work for kedge, roughly in priority order.

## Planned

### Built-in agent configuration and steering templates
`kedge init` generates `kedge.toml` but leaves the agent command, steering files, and agent instructions as blank placeholders. New users must figure out agent integration from scratch. Ship a default agent configuration that works with Claude Code (and optionally Kiro), including a starter `agent_command`, `agent_instructions`, and a template steering file with example anchors. A `kedge init --full` flag could scaffold the complete setup: config, a sample steering doc, and platform-specific agent files (`CLAUDE.md`, `AGENTS.md`).

### AST cache for repeated file parsing
When multiple anchors point at the same source file, kedge parses the full file with tree-sitter once per anchor. A 10K-line file with 20 anchors takes ~73 ms/anchor vs ~3 ms/anchor for single-anchor files. Cache the parsed AST per file path within a single `kedge check` invocation so each file is parsed once regardless of anchor count.

### Parallel detection
The detection loop in `detect_drift` processes docs and anchors serially. For large codebases (500+ anchors), parallelizing fingerprint computation across files with rayon would cut check time roughly proportional to available cores.

### Doc-scoped triage classifications
`triage_drift_report` classifies each doc separately, but `apply_classifications` flattens results into a single `HashMap<(path, symbol), Severity>`. If two docs anchor to the same code symbol, the last classification wins for both. Apply each doc's classifications to that doc immediately instead of merging into a shared map.

### Multi-repo doc identity in lookups
`collect_doc_contents` and `resolve_doc_file` key by repo-relative path only. Two docs repos with a file at the same relative path (e.g., both have `auth.md`) collide. Include `doc_repo` in the lookup key.

### Windows subprocess termination
`invoke_agent` uses SIGKILL for timeout enforcement, which only works on Unix. Windows processes that exceed `agent_timeout` are not killed. Add Windows-compatible process termination via `TerminateProcess`.

### Logging and telemetry
Add structured logging throughout the pipeline (detection, triage, remediation) and optional telemetry for tracking drift trends over time.

### Regex and glob patterns in anchor frontmatter
Allow anchors to target multiple files via glob patterns (e.g., `src/auth/**/*.java`) or regex matches instead of requiring one anchor per file. Reduces frontmatter boilerplate for docs that track an entire module or directory.

## Done (v0.3.1)

- **Fix agent payload targeting.** `target.repo` now contains the docs repo URL (from `[[repos.docs]]` config) instead of the code repo URL. `target.path` is repo-relative instead of an absolute cache path. Added `KEDGE_DOCS_REPO_URL` env var for `KEDGE_DOCS_PATH` users.
- **Exit non-zero on agent failure.** `kedge update` now exits 1 when any agent invocation fails. Previously always exited 0.
- **Repo cache keyed by URL and ref.** Two `[[repos.docs]]` entries for the same repo at different refs no longer share a checkout directory.
- **`kedge install` respects `[[repos.docs]].path`.** The configured subdirectory is now joined to the clone root before scanning for steering files.
- **Path traversal guard.** `collect_doc_contents` and `resolve_doc_file` validate that resolved paths stay within the repo root.

## Done (v0.3.0)

- **Skip triage with `provider = "none"`.** Forward all drifted anchors to the remediation agent as `major` severity, letting the agent handle classification.
- **`${VAR}` expansion for `agent_env` and `triage_env`.** Environment variable maps now resolve `${VAR}` patterns from the parent process environment.
- **HTTP timeout on hosted triage providers.** `triage_timeout` now applies to Anthropic and OpenAI API calls, not just the command provider.
- **Default unclassified anchors to `minor`.** Anchors missing from the triage response default to `minor` instead of `no_update`, preventing silent provenance advancement on unclassified docs.
- **Tag ref handling in repo cache.** `get_or_clone` refresh now uses `FETCH_HEAD` instead of `origin/<ref>`, fixing cache refresh for tag-based refs.
- **Removed dead config fields.** `detection.languages`, `detection.fallback`, and `triage.severity_levels` were parsed but unused. Removed from the config struct. Old configs with these fields still parse without error.
- **Migrated docs site to mkdocs-material.** Replaced mdBook with mkdocs-material for better typography, light/dark toggle, and search.
