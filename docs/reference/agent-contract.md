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

The agent's **stderr is inherited**, so anything written to stderr appears in kedge's output. Use this for progress logging.

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
      "current_sig": "sig:a1b2c3d4e5f67890",
      "summary": "auth-validation.md: 1 anchor(s) classified",
      "diff": "--- a/src/auth/AuthService.java\n+++ b/src/auth/AuthService.java\n@@ -42,7 +42,8 @@..."
    }
  ],
  "instructions": "Update the documentation to reflect the code changes described in the drifted anchors. After updating the prose, run `kedge sync` to advance provenance automatically. If kedge is not available in your environment, set each anchor's provenance in the frontmatter to the corresponding current_sig value as a fallback."
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
| `drifted_anchors[].current_sig` | string | The current code fingerprint (`sig:...`). Provided as a fallback — the recommended approach is to run `kedge sync` after updating the doc (see [Provenance management](#provenance-management) below). If kedge is unavailable, the agent can write this value directly as the new `provenance` in the frontmatter. |
| `drifted_anchors[].summary` | string | Human-readable summary of the triage result. |
| `drifted_anchors[].diff` | string | Git diff of the changes (empty for `sig:` provenance). |
| `instructions` | string | Natural-language instructions for the agent. Uses `agent_instructions` from config if set, otherwise a default. |

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
  "instructions": "Update the documentation to reflect the code changes described in the drifted anchors. After updating the prose, run `kedge sync` to advance provenance automatically. If kedge is not available in your environment, set each anchor's provenance in the frontmatter to the corresponding current_sig value as a fallback."
}
```

In batch mode:
- `action` is `"update_docs_batch"`
- `auto_merge` is `true` only if **every** target qualifies individually
- Each element in `targets` has the same structure as the per-doc `target` + `severity` + `drifted_anchors` (including `current_sig` on each anchor)

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

## After the agent returns

kedge captures the MR URL from the agent's output, includes it in the `RemediationSummary`, and exits. kedge does not:

- Monitor or poll MR status
- Trigger CI pipelines on the docs repository
- Merge the MR
- Verify the agent's changes

The `auto_merge` flag in the payload is a signal to the agent, not an action by kedge. It indicates that the drift severity qualifies for automatic merging based on the `auto_merge_severities` config. The agent decides whether to act on it (e.g., enabling auto-merge on the MR, merging immediately, or ignoring it).

### Plain text fallback

If the agent's stdout is not valid JSON, kedge scans the output for URLs:

1. Splits output by whitespace
2. Finds tokens starting with `https://` or `http://`
3. Strips trailing punctuation (`,`, `.`, `;`, `)`, `]`, `"`, `'`)
4. Uses the first URL found as the MR link

An agent can print `Created MR: https://github.com/org/docs/pull/42` and kedge will extract the URL.

## Provenance management

After updating a steering file's prose, the agent must advance the `provenance` values in the file's `kedge:` frontmatter so kedge knows the doc is current.

### Recommended: `kedge sync`

The simplest and most reliable approach is to run `kedge sync` after editing. With no arguments, it finds all `.md` files with kedge frontmatter and recomputes their fingerprints from the current code. The agent only needs to focus on updating prose.

```bash
# 1. Update the doc prose
# 2. Advance provenance for all steering files
kedge sync
# 3. Commit, push, open MR
```

This is the default workflow used by kedge's built-in instructions. It requires `kedge` to be installed in the agent's execution environment and access to the code repository.

### Fallback: manual `current_sig`

If `kedge` is not available in the agent's environment, the agent can write the `current_sig` value from each drifted anchor directly into the frontmatter's `provenance` field. This is more error-prone (the agent must correctly paste hex fingerprint strings into YAML) but works without any kedge dependency.

Organizations can enforce this approach by setting custom `agent_instructions` in `kedge.toml`:

```toml
[remediation]
agent_instructions = "Update the documentation, then set each anchor's provenance to the corresponding current_sig value."
```

## Exit code

The agent process must exit with code `0` on success. kedge treats a non-zero exit code as an error and logs the failure in the `RemediationSummary.errors` array. If any agent invocation fails, `kedge update` itself exits with code `1` after printing the summary.

## `no_update` anchors

kedge does **not** send anchors classified as `no_update` to the agent. By default, kedge advances their provenance by recomputing the fingerprint and writing it to the steering file. Pass `--no-stamp` to `kedge update` to skip the write. Use this in CI when docs live in a separate repo. Run `kedge sync` after agent MRs merge to advance provenance in a dedicated commit.
