# Quick Start

Get kedge running in your code repository in five minutes.

## Prerequisites

- kedge [installed](installation.md)
- A git repository with code
- A docs repository with markdown steering files (or you'll create one below)

## 1. Initialize

In your code repository root:

```bash
kedge init
```

kedge creates a `kedge.toml` with sensible defaults. Open it and configure:

- **`[detection].languages`**: the languages in your codebase (e.g., `["java", "typescript", "python"]`)
- **`[triage]`**: your AI provider (`anthropic`, `openai`, or `command`)
- **`[remediation].agent_command`**: the command that updates docs (e.g., a Kiro or Claude Code agent)
- **`[[repos.docs]]`**: the git URL and path of your documentation repository

```toml
[detection]
languages = ["java", "typescript"]
fallback = "content-hash"

[triage]
provider = "anthropic"
model = "claude-haiku-4-5-20251001"

[remediation]
agent_command = "your-agent-command"
auto_merge_severities = ["no_update"]

[repos]
docs = [
  { url = "git@github.com:your-org/docs.git", path = "steering/", ref = "main" },
]

[[agents]]
name = "claude"
global_steering = "~/.claude/docs"
workspace_steering = "docs"
agents_file = "CLAUDE.md"
skill_dir = ""
```

## 2. Stamp provenance

Once your steering files have `kedge:` frontmatter with anchors pointing to code locations:

```bash
kedge link
```

kedge computes an AST fingerprint for each anchored code location and writes it into the steering file's `provenance` field. Future drift detection compares against this baseline.

## 3. Detect drift

```bash
kedge check
```

kedge outputs a JSON drift report to stdout. Exit code `0` means all docs are up to date. Exit code `1` means drift was found. At least one anchor's code has changed since provenance was last stamped.

Save the report to a file:

```bash
kedge check --report drift.json
```

## 4. Run the full pipeline

```bash
kedge update
```

kedge runs the complete three-layer pipeline:

1. **Detection**: compares AST fingerprints at provenance vs HEAD
2. **Triage**: classifies each drifted anchor as `no_update`, `minor`, or `major` via AI
3. **Remediation**: invokes your agent to update docs and open merge requests, and advances provenance for `no_update` anchors

kedge outputs a `RemediationSummary` JSON showing what was remediated, what had provenance advanced, and any errors.

## 5. Check status

```bash
kedge status
```

kedge shows all steering files with their anchors, groups, and current provenance values.

## Optional: distribute steering files to agents

If your agents read docs from specific local directories (e.g., `.kiro/steering/`), use `kedge install` to copy or symlink steering files there. This is independent of the drift pipeline above.

```bash
kedge install --link                  # dev machine: symlink to global agent dirs
kedge install --workspace             # CI: copy to workspace dirs
```

See [Agent Integration](../guides/agent-integration.md) for details on configuring `[[agents]]` platforms.

## Next steps

- [Your First Steering File](first-steering-file.md): create a steering file from scratch
- [Configuration](../reference/configuration.md): full `kedge.toml` reference
- [GitLab CI](../guides/ci-gitlab.md) or [GitHub Actions](../guides/ci-github-actions.md): set up kedge in CI
