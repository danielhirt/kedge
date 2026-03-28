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
      "summary": "auth-validation.md: 1 anchor(s) classified",
      "diff": "--- a/src/auth/AuthService.java\n+++ b/src/auth/AuthService.java\n..."
    }
  ],
  "instructions": "Update documentation for commit abc123. Apply the changes described in the drifted anchors and stamp provenance with abc123."
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
  "instructions": "Update documentation for commit abc123..."
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

The `auto_merge_severities` config controls which severity levels get `"auto_merge": true` in the payload. The agent decides what to do with this flag. In most cases, it means the MR can be merged without human review.

```toml
auto_merge_severities = ["no_update", "minor"]
```

In batch mode, `auto_merge` is `true` only if **every** target qualifies individually.

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

# Do your work here...
# (update the doc, commit, push, create MR)

# Print result
echo "{\"mr_url\": \"https://github.com/your-org/docs/pull/99\", \"status\": \"success\"}"
```

The agent's stderr is inherited and shown in the kedge output, so you can use it for progress logging.

## Agent platform configuration

The `[[agents]]` section in `kedge.toml` configures how `kedge install` distributes steering files to agent-specific directories:

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
skill_dir = ""
```

`kedge install` uses this configuration to place doc files where each agent platform can find them. It does not affect how the agent command is invoked. The paths are platform-specific: Kiro uses `.kiro/steering/`, but Claude Code has no built-in "steering" directory, so use any path the agent can read (e.g., `docs/`).
