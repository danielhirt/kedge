# Roadmap

Planned work for kedge, roughly in priority order.

## Planned

### AST cache for repeated file parsing
When multiple anchors point at the same source file, kedge parses the full file with tree-sitter once per anchor. A 10K-line file with 20 anchors takes ~73 ms/anchor vs ~3 ms/anchor for single-anchor files. Cache the parsed AST per file path within a single `kedge check` invocation so each file is parsed once regardless of anchor count.

### Parallel detection
The detection loop in `detect_drift` processes docs and anchors serially. For large codebases (500+ anchors), parallelizing fingerprint computation across files with rayon would cut check time roughly proportional to available cores.

### Windows subprocess termination
`invoke_agent` uses SIGKILL for timeout enforcement, which only works on Unix. Windows processes that exceed `agent_timeout` are not killed. Add Windows-compatible process termination via `TerminateProcess`.

### Logging and telemetry
Add structured logging throughout the pipeline (detection, triage, remediation) and optional telemetry for tracking drift trends over time.

### Regex and glob patterns in anchor frontmatter
Allow anchors to target multiple files via glob patterns (e.g., `src/auth/**/*.java`) or regex matches instead of requiring one anchor per file. Reduces frontmatter boilerplate for docs that track an entire module or directory.

## Done (v0.3.0)

- **Skip triage with `provider = "none"`.** Forward all drifted anchors to the remediation agent as `major` severity, letting the agent handle classification.
- **`${VAR}` expansion for `agent_env` and `triage_env`.** Environment variable maps now resolve `${VAR}` patterns from the parent process environment.
- **HTTP timeout on hosted triage providers.** `triage_timeout` now applies to Anthropic and OpenAI API calls, not just the command provider.
- **Default unclassified anchors to `minor`.** Anchors missing from the triage response default to `minor` instead of `no_update`, preventing silent provenance advancement on unclassified docs.
- **Tag ref handling in repo cache.** `get_or_clone` refresh now uses `FETCH_HEAD` instead of `origin/<ref>`, fixing cache refresh for tag-based refs.
- **Removed dead config fields.** `detection.languages`, `detection.fallback`, and `triage.severity_levels` were parsed but unused. Removed from the config struct. Old configs with these fields still parse without error.
- **Migrated docs site to mkdocs-material.** Replaced mdBook with mkdocs-material for better typography, light/dark toggle, and search.
