# Configuration (kedge.toml)

`kedge.toml` lives in the root of your code repository. Run `kedge init` to generate a template.

## Full example

```toml
[detection]
# exclude_dirs = [".git", "node_modules", "target", ".venv", "__pycache__", ".tox", "vendor"]

[triage]
provider = "anthropic"
model = "claude-haiku-4-5-20251001"
# api_url = ""                        # custom API endpoint (enterprise proxy)
# api_key_env = ""                    # env var name for API key
# triage_command = ""                 # required when provider = "command"
# triage_timeout = 120                # seconds per doc
# triage_env = { }                    # extra env vars for command provider

[remediation]
agent_command = "your-agent-command"
auto_merge_severities = ["no_update"]
# batch = false                       # single agent call for all drifted docs
# agent_timeout = 300                 # seconds before agent process is killed
# agent_env = { }                     # extra env vars passed to agent

[repos]
# git_timeout = 300                   # seconds for clone/fetch operations
docs = [
  { url = "git@github.com:your-org/docs.git", path = "steering/", ref = "main" },
]

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

## `[detection]`

Controls how kedge scans for steering files. Language detection is automatic based on file extensions.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `exclude_dirs` | string array | see below | Directory names to skip when scanning for steering files. |

### `exclude_dirs`

When scanning a docs directory for `.md` files, kedge skips any path containing a directory segment matching one of these names. The default list:

```toml
exclude_dirs = [".git", "node_modules", "target", ".venv", "__pycache__", ".tox", "vendor"]
```

Override to match your project layout:

```toml
[detection]
exclude_dirs = [".git", "node_modules", "build", ".next"]
```

Setting `exclude_dirs = []` disables all exclusions.

## `[triage]`

Controls AI-based severity classification.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | string | `"command"` | AI provider: `"anthropic"`, `"openai"`, or `"command"`. |
| `model` | string | `""` | Model ID. Required for `anthropic` and `openai` providers. Example: `"claude-haiku-4-5-20251001"`. |
| `api_url` | string | provider default | Custom API endpoint URL. Use for enterprise proxies or API gateways. |
| `api_key_env` | string | `"ANTHROPIC_API_KEY"` or `"OPENAI_API_KEY"` | Name of the environment variable holding the API key. |
| `triage_command` | string | `""` | Shell command for the `command` provider. Receives prompt on stdin. |
| `triage_timeout` | integer | `120` | Seconds to wait per triage call before timing out. |
| `triage_env` | table | `{}` | Extra environment variables passed to the `command` provider process. Values support `${VAR}` expansion. |

### Provider details

**`anthropic`**: Direct Anthropic API. Default endpoint: `https://api.anthropic.com/v1/messages`. Set `api_url` to route through an enterprise proxy. Default API key env var: `ANTHROPIC_API_KEY`.

**`openai`**: Any OpenAI-compatible endpoint (Azure OpenAI, vLLM, local models). Default endpoint: `https://api.openai.com/v1/chat/completions`. If `api_url` ends with `/v1`, kedge appends `/chat/completions`. `model` is required. Default API key env var: `OPENAI_API_KEY`.

**`command`**: Pipes the triage prompt to an external command via stdin. Set `triage_command` to the shell command. Use `triage_env` for extra environment variables. The command must print a JSON array to stdout classifying each anchor:

```json
[
  {"path": "src/Auth.java", "symbol": "Auth#validate", "severity": "minor"},
  {"path": "src/Baz.java", "symbol": null, "severity": "no_update"}
]
```

Each element needs `path` (string), `symbol` (string or null), and `severity` (`"no_update"`, `"minor"`, or `"major"`). The response may optionally be wrapped in `` ```json `` fences — kedge strips them before parsing.

## `[remediation]`

Controls the agent invocation layer.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `agent_command` | string | (required) | Shell command to invoke the agent. Receives JSON on stdin. |
| `auto_merge_severities` | string array | `[]` | Severity levels for which `auto_merge` is set to `true` in the agent payload. |
| `batch` | boolean | `false` | When `true`, send all drifted docs in a single agent invocation instead of one per doc. |
| `agent_timeout` | integer | `300` | Seconds before the agent process is killed (SIGKILL). |
| `agent_env` | table | `{}` | Extra environment variables passed to the agent process. Values support `${VAR}` expansion. |
| `agent_instructions` | string | `""` | Custom text that replaces the default `instructions` in the agent JSON payload. |

### `agent_instructions`

The default instruction tells the agent to update documentation based on the drifted anchors. Set `agent_instructions` to replace it with org-specific guidance:

```toml
[remediation]
agent_command = "your-agent-command"
agent_instructions = """
Update the documentation to reflect the code changes.
Follow our style guide at https://wiki.example.com/docs-style.
Include the JIRA ticket ID in the MR title.
"""
```

When set, this value becomes the `instructions` field in the JSON payload sent to the agent. When empty (default), kedge uses a generic instruction.

### Per-doc vs batch mode

**Per-doc mode** (default): kedge invokes the agent once per drifted doc. The payload action is `"update_docs"`. Best when each doc should get its own MR.

**Batch mode** (`batch = true`): kedge sends all drifted docs in a single invocation. The payload action is `"update_docs_batch"`. Best when you want one MR covering all drifted docs, or when agent startup is expensive.

## `[repos]`

Controls git operations for doc repositories.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `git_timeout` | integer | `300` | Seconds for clone, fetch, and ls-remote operations. |

### `[[repos.docs]]`

One or more documentation repositories. kedge clones each repo and scans all of them during `check` and `update`.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | string | (required) | Git URL of the docs repository (SSH or HTTPS). |
| `path` | string | (required) | Subdirectory within the repo where steering files live. Use `"."` for the repo root. |
| `ref` | string | (required) | Git branch or tag to track. |
| `remote_name` | string | `"origin"` | Git remote name used for fetch operations. |

Docs repos are cloned to `~/.cache/kedge/repos/` and fetched on each run. The cache directory uses `0o700` permissions.

#### `remote_name`

kedge fetches from the `origin` remote by default. Set `remote_name` to pull from a different remote, useful for fork-based workflows:

```toml
[[repos.docs]]
url = "git@github.com:your-org/docs.git"
path = "steering/"
ref = "main"
remote_name = "upstream"
```

After the initial clone, kedge renames the remote from `origin` to the specified name. Subsequent fetches use that remote.

#### Multiple repos

Both `kedge check` and `kedge update` scan steering files from all configured repos and merge the results into a single drift report.

```toml
[[repos.docs]]
url = "git@github.com:your-org/platform-docs.git"
path = "steering/"
ref = "main"

[[repos.docs]]
url = "git@github.com:your-org/api-docs.git"
path = "."
ref = "main"
```

## `[[agents]]`

Agent platform configurations used by `kedge install` to distribute doc files. The paths are platform-specific. Kiro uses `.kiro/steering/`, while other agents (Claude Code, custom tools) use whatever directory they read docs from.

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Platform identifier. Used with `kedge install --agent <name>`. |
| `global_steering` | string | Path for symlinked steering files on dev machines (`kedge install --link`). Supports `~` expansion. |
| `workspace_steering` | string | Path for copied steering files in CI (`kedge install --workspace`). |
| `agents_file` | string | Platform-specific instructions file name (e.g., `"AGENTS.md"`, `"CLAUDE.md"`). Installed from `_kedge/AGENTS.md` in the docs repo. |
| `skill_dir` | string | Path for agent skill files. If empty, skill installation is skipped. |

## Timeout budget

All timeouts are configurable. Default values fit within a 1-hour CI pipeline:

| Setting | Default | Worst case (5 docs) |
|---------|---------|---------------------|
| `triage_timeout` | 120s | ~10 min (serial) |
| `agent_timeout` | 300s | ~25 min per-doc, ~5 min batch |
| `git_timeout` | 300s | ~5 min (clone + fetch) |

Total worst case: ~40 min per-doc mode, ~20 min batch mode.
