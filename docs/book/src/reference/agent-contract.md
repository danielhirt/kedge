# Agent Interface Contract

kedge invokes an external agent process via the `agent_command` configured in `kedge.toml`. The agent receives a JSON payload on stdin and prints its result to stdout.

## Invocation

kedge runs the agent command as a subprocess:

1. Parses `agent_command` as a shell command (supports quoting and arguments)
2. Spawns the process with stdin piped and stdout captured
3. Writes the JSON payload to stdin and closes it
4. Reads stdout on a background thread
5. Waits up to `agent_timeout` seconds for the process to exit
6. If the timeout is exceeded, sends SIGKILL

The agent's **stderr is inherited** -- anything written to stderr appears in kedge's output. Use this for progress logging.

Extra environment variables from `agent_env` are passed to the process.

## Per-doc payload

When `batch = false` (the default), kedge invokes the agent once per drifted doc.

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
      "diff": "--- a/src/auth/AuthService.java\n+++ b/src/auth/AuthService.java\n@@ -42,7 +42,8 @@..."
    }
  ],
  "instructions": "Update documentation for commit abc123def. Apply the changes described in the drifted anchors and stamp provenance with abc123def."
}
```

### Field reference

| Field | Type | Description |
|-------|------|-------------|
| `action` | string | Always `"update_docs"` for per-doc mode. |
| `severity` | string | Overall severity for this doc: `"no_update"`, `"minor"`, or `"major"`. |
| `auto_merge` | boolean | Whether the MR qualifies for auto-merge based on `auto_merge_severities` config. |
| `target.repo` | string | Git URL of the docs repository containing the steering file. |
| `target.branch_prefix` | string | Suggested branch name prefix (`"kedge/auto-update"`). |
| `target.path` | string | Path of the steering file within the docs repo. |
| `drifted_anchors` | array | Anchors that changed. Only includes anchors with severity above `no_update`. |
| `drifted_anchors[].path` | string | Code file path. |
| `drifted_anchors[].symbol` | string or null | Symbol within the file, if scoped. |
| `drifted_anchors[].severity` | string | Anchor-level severity. |
| `drifted_anchors[].summary` | string | Human-readable summary of the triage result. |
| `drifted_anchors[].diff` | string | Git diff of the changes (empty for `sig:` provenance). |
| `instructions` | string | Natural-language instructions for the agent. |

## Batch payload

When `batch = true`, kedge sends all drifted docs in a single invocation.

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
  "instructions": "Update documentation for commit abc123def..."
}
```

In batch mode:
- `action` is `"update_docs_batch"`
- `auto_merge` is `true` only if **every** target qualifies individually
- Each element in `targets` has the same structure as the per-doc `target` + `severity` + `drifted_anchors`

## Response format

### Structured JSON (preferred)

**Per-doc mode:**

```json
{
  "mr_url": "https://github.com/your-org/docs/pull/42",
  "status": "success"
}
```

**Batch mode:**

```json
{
  "mr_urls": [
    "https://github.com/your-org/docs/pull/42",
    "https://github.com/your-org/docs/pull/43"
  ],
  "status": "success"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `mr_url` | string (optional) | URL of the created merge request. |
| `mr_urls` | string array (optional) | URLs of created merge requests (batch mode). |
| `status` | string | Status indicator (informational, not parsed by kedge). |

### Plain text fallback

If the agent's stdout is not valid JSON, kedge scans the output for URLs:

1. Splits output by whitespace
2. Finds tokens starting with `https://` or `http://`
3. Strips trailing punctuation (`,`, `.`, `;`, `)`, `]`, `"`, `'`)
4. Uses the first URL found as the MR link

This makes it easy to get started -- an agent can simply print `Created MR: https://github.com/org/docs/pull/42` and kedge will extract the URL.

## Exit code

The agent process must exit with code `0` on success. A non-zero exit code is treated as an error, and kedge logs the failure in the `RemediationSummary.errors` array.

## `no_update` anchors

Anchors classified as `no_update` are **not** sent to the agent. Instead, kedge automatically advances their provenance by recomputing the fingerprint and writing it to the steering file. This avoids unnecessary agent invocations for cosmetic code changes.
