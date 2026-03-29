# Agent Integration

kedge's remediation layer invokes an external agent process to update documentation and open merge requests. The agent receives a JSON payload on stdin and prints its result to stdout.

## Supported agents

kedge is agent-agnostic. Any process that reads JSON from stdin and writes output to stdout works. Common choices:

- **Kiro**, Amazon's AI coding agent
- **Claude Code**, Anthropic's CLI agent
- **Custom scripts** that wrap any tool or LLM behind a shell command

## Configuration

Set the agent command in `kedge.toml`:

```toml
[remediation]
agent_command = "your-agent-command"
auto_merge_severities = ["no_update"]
# batch = false        # true = one call for all docs; false = one call per doc
# agent_timeout = 300  # seconds before the process is killed
# agent_env = { }      # extra env vars passed to the agent process
```

The `agent_command` is parsed as a shell command (using `shell-words`), so quoting and arguments work as expected:

```toml
agent_command = "node scripts/update-docs.js --verbose"
```

## Payload format

### Per-doc mode (default)

kedge invokes the agent once per drifted doc. The JSON payload sent on stdin:

```json
{
  "action": "update_docs",
  "severity": "minor",
  "auto_merge": false,
  "target": {
    "repo": "git@github.com:your-org/docs.git",
    "branch_prefix": "kedge/auto-update",
    "path": "steering/auth-validation.md"
  },
  "drifted_anchors": [
    {
      "path": "src/auth/AuthService.java",
      "symbol": "AuthService#validateToken",
      "severity": "minor",
      "current_sig": "sig:a1b2c3d4e5f67890",
      "summary": "auth-validation.md: 1 anchor(s) classified",
      "diff": "--- a/src/auth/AuthService.java\n+++ b/src/auth/AuthService.java\n..."
    }
  ],
  "instructions": "Update the documentation to reflect the code changes described in the drifted anchors. After updating the prose, run `kedge sync` to advance provenance automatically. If kedge is not available in your environment, set each anchor's provenance in the frontmatter to the corresponding current_sig value as a fallback."
}
```

### Batch mode

When `batch = true` in config, kedge sends all drifted docs in a single invocation:

```json
{
  "action": "update_docs_batch",
  "auto_merge": false,
  "targets": [
    {
      "target": {
        "repo": "git@github.com:your-org/docs.git",
        "branch_prefix": "kedge/auto-update",
        "path": "steering/auth-validation.md"
      },
      "severity": "minor",
      "drifted_anchors": [...]
    },
    {
      "target": {
        "repo": "git@github.com:your-org/docs.git",
        "branch_prefix": "kedge/auto-update",
        "path": "steering/payment-flow.md"
      },
      "severity": "major",
      "drifted_anchors": [...]
    }
  ],
  "instructions": "Update the documentation to reflect the code changes described in the drifted anchors. After updating the prose, run `kedge sync` to advance provenance automatically. If kedge is not available in your environment, set each anchor's provenance in the frontmatter to the corresponding current_sig value as a fallback."
}
```

## Response format

### Structured JSON (preferred)

Per-doc mode:

```json
{"mr_url": "https://github.com/your-org/docs/pull/42", "status": "success"}
```

Batch mode:

```json
{
  "mr_urls": [
    "https://github.com/your-org/docs/pull/42",
    "https://github.com/your-org/docs/pull/43"
  ],
  "status": "success"
}
```

### Plain text fallback

If the agent does not output valid JSON, kedge scans stdout for URLs starting with `https://` or `http://` and uses the first match as the MR link.

## Auto-merge

The `auto_merge_severities` config controls which severity levels get `"auto_merge": true` in the payload.

```toml
auto_merge_severities = ["no_update", "minor"]
```

In batch mode, `auto_merge` is `true` only if **every** target qualifies individually.

kedge passes the flag; the agent decides what to do with it (enable auto-merge on the MR, merge immediately, or ignore it). kedge exits after receiving the agent's response and does not monitor, merge, or verify MRs. MR lifecycle management — approval, CI, merging — belongs to your existing review workflows.

## Timeouts

`agent_timeout` (default: 300 seconds) sets how long kedge waits for the agent process. If exceeded, kedge sends SIGKILL and reports an error.

For batch mode with many docs, increase the timeout:

```toml
agent_timeout = 600
```

## Environment variables

Pass extra env vars to the agent via `agent_env`:

```toml
[remediation]
agent_command = "scripts/update-docs.sh"
agent_env = {
  GITLAB_TOKEN = "${GITLAB_TOKEN}",
  DOCS_REPO = "git@github.com:your-org/docs.git"
}
```

Values support `${VAR}` expansion from the current environment.

## Writing a custom agent

A minimal agent script that reads the payload and prints a result:

```bash
#!/bin/bash
# Read JSON payload from stdin
payload=$(cat)

# Extract fields
doc_path=$(echo "$payload" | jq -r '.target.path')
severity=$(echo "$payload" | jq -r '.severity')

# 1. Update the doc prose (your logic here)
# ...

# 2. Advance provenance via kedge sync (recommended)
kedge sync

# 3. Commit, push, create MR
# ...

# Print result
echo "{\"mr_url\": \"https://github.com/your-org/docs/pull/99\", \"status\": \"success\"}"
```

The agent's stderr is inherited and shown in the kedge output, so you can use it for progress logging.

## Provenance management

After updating a steering file, the agent must advance the `provenance` values in its `kedge:` frontmatter so kedge treats the doc as current.

**Recommended: `kedge sync`** — Run `kedge sync` after editing. With no arguments, it finds all `.md` files with kedge frontmatter and recomputes their fingerprints from the current code. The agent focuses on prose; kedge handles the bookkeeping. Requires `kedge` in the agent's environment and access to the code repo.

**Fallback: manual `current_sig`** — If `kedge` is unavailable, the agent can write each anchor's `current_sig` from the payload directly into the frontmatter `provenance` field. This works without a kedge dependency but is more error-prone. To use this approach, set custom instructions in `kedge.toml`:

```toml
[remediation]
agent_instructions = "Update the documentation, then set each anchor's provenance to the corresponding current_sig value."
```

## Agent platform configuration (`kedge install`)

The `[[agents]]` section in `kedge.toml` configures `kedge install`, which distributes steering files to agent-specific directories. This is a separate workflow from the drift pipeline — `kedge install` sets up agent workspaces, while `kedge check`/`kedge update` handle drift detection and remediation.

```toml
[[agents]]
name = "kiro"
global_steering = "~/.kiro/steering/"
workspace_steering = ".kiro/steering/"
agents_file = "AGENTS.md"
skill_dir = ".kiro/skills/"

[[agents]]
name = "claude"
global_steering = "~/.claude/docs/"
workspace_steering = "docs/"
agents_file = "CLAUDE.md"
skill_dir = ".claude/skills/"
```

`kedge install` places doc files where each agent platform reads them. The `[[agents]]` config does not affect how the agent command is invoked during remediation — that's controlled by `[remediation].agent_command`.

Typical usage:
- **Dev machines**: `kedge install --link` symlinks steering files to global agent directories
- **CI**: `kedge install --workspace` copies them to workspace directories before the agent runs
