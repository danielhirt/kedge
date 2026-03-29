<p align="center">
  <img src="docs/assets/logo.png" alt="kedge" width="500">
</p>

<p align="center">
  <a href="https://github.com/danielhirt/kedge/actions/workflows/ci.yml"><img src="https://github.com/danielhirt/kedge/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/danielhirt/kedge/releases/latest"><img src="https://img.shields.io/github/v/release/danielhirt/kedge" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/danielhirt/kedge" alt="License"></a>
</p>

CLI that detects when code changes make docs stale, classifies drift severity via AI, and invokes agents to update docs and open merge requests.

## How It Works

Three-layer pipeline:

1. **Detection.** Compare AST fingerprints of code at provenance vs HEAD. Deterministic, no AI. Outputs a drift report.
2. **Triage.** Classify each drifted anchor as `no_update`, `minor`, or `major` via a lightweight LLM call.
3. **Remediation.** Invoke an external agent to update the docs and open an MR. `no_update` anchors get their provenance advanced without doc changes (pass `--no-stamp` in CI to defer this to `kedge sync`). kedge's pipeline ends when the agent returns. Your review workflows handle MR approval, CI, and merging.

Any markdown file with `kedge:` frontmatter becomes a tracked doc: standalone files, `AGENTS.md`, `CLAUDE.md`, or anything else. kedge calls these "steering files" (a term from Kiro), but the tool is agent-agnostic.

## Installation

### Shell installer (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/danielhirt/kedge/main/install.sh | sh
```

The script detects your platform and downloads the latest release binary to `/usr/local/bin`. Override the install directory with `KEDGE_INSTALL_DIR`:

```bash
curl -fsSL https://raw.githubusercontent.com/danielhirt/kedge/main/install.sh | KEDGE_INSTALL_DIR=~/.local/bin sh
```

### Homebrew (macOS/Linux)

```bash
brew install danielhirt/tap/kedge
```

### Docker

```bash
docker run --rm -v "$PWD:/repo" -w /repo danielhirt/kedge check
```

Or build from source:

```bash
docker build -t kedge .
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/danielhirt/kedge/releases). Binaries cover Linux, macOS, and Windows (x86_64 and aarch64). Each archive includes a `.sha256` checksum file.

### From source

Requires Rust 1.70+ and `git` on PATH.

```bash
cargo install --path .
```

### CI runner setup

For air-gapped runners, pre-build the binary and add it to your runner image:

```dockerfile
COPY kedge /usr/local/bin/kedge
```

On developer machines, `kedge install --link` symlinks steering files. In CI, `kedge install --workspace` copies them.

## Quick Start

```bash
# In your code repository:
kedge init                  # Creates kedge.toml with defaults
# Edit kedge.toml with your repos, agent command, and triage provider
kedge link                  # Stamp initial provenance on all doc anchors
kedge check                 # Detect drift (exit 0 = clean, exit 1 = drift)
kedge update                # Full pipeline: detect -> triage -> agent -> MR
```

## Configuration

`kedge.toml` lives in your code repository root.

```toml
[detection]
# exclude_dirs = [".git", "node_modules", "target", ".venv", "__pycache__", ".tox", "vendor"]

[triage]
provider = "anthropic"                # "anthropic", "openai", "command", or "none"
model = "claude-haiku-4-5-20251001"   # required for anthropic/openai providers
# api_url = ""                        # custom API endpoint (enterprise proxy/gateway)
# api_key_env = ""                    # env var name for API key (default: ANTHROPIC_API_KEY or OPENAI_API_KEY)
# triage_timeout = 120                # seconds per doc (default: 120)
# triage_env = { }                    # extra env vars for command provider
# severity_levels = ["no_update", "minor", "major"]

[remediation]
agent_command = "your-agent-command"  # receives JSON on stdin, prints result to stdout
auto_merge_severities = ["no_update"]
# batch = true                        # single agent invocation for all drifted docs
# agent_timeout = 300                 # seconds, kills agent process if exceeded (default: 300)
# agent_env = { }                     # extra env vars passed to agent process
# agent_instructions = ""             # replaces default agent instructions when set

[repos]
# git_timeout = 300                   # seconds for clone/fetch operations (default: 300)

[[repos.docs]]
url = "git@gitlab.example.com:platform/docs.git"
path = "steering/"
ref = "main"
# remote_name = "origin"              # git remote name for fetch (default: "origin")

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

### Configuration Reference

| Section | Field | Default | Description |
|---------|-------|---------|-------------|
| `[detection]` | `exclude_dirs` | `.git`, `node_modules`, ... | Directories to skip when scanning for docs |
| `[triage]` | `provider` | `"command"` | AI provider: `anthropic`, `openai`, `command`, or `none` |
| | `model` | | Model ID (required for `anthropic`/`openai`) |
| | `api_url` | provider default | Custom API endpoint for enterprise proxies |
| | `api_key_env` | `ANTHROPIC_API_KEY` or `OPENAI_API_KEY` | Env var name holding the API key |
| | `triage_command` | | Shell command for `command` provider |
| | `triage_timeout` | `120` | Seconds per triage call |
| | `triage_env` | `{}` | Extra env vars for `command` provider. Values support `${VAR}` expansion. |
| `[remediation]` | `agent_command` | | Shell command to invoke the agent |
| | `auto_merge_severities` | `[]` | Severities where auto-merge flag is set |
| | `batch` | `false` | Bundle all drifted docs into one agent call |
| | `agent_timeout` | `300` | Seconds before agent process is killed |
| | `agent_env` | `{}` | Extra env vars passed to agent. Values support `${VAR}` expansion. |
| | `agent_instructions` | `""` | Replaces the default `instructions` in the agent payload |
| `[repos]` | `git_timeout` | `300` | Seconds for clone/fetch/ls-remote operations |
| `[[repos.docs]]` | `url` | | Git URL of the documentation repository |
| | `path` | | Subdirectory within docs repo for steering files |
| | `ref` | | Git branch or tag to track |
| | `remote_name` | `"origin"` | Git remote name for fetch operations |
| `[[agents]]` | `name` | | Platform identifier (used with `--agent` flag) |
| | `global_steering` | | Path for symlinked steering files (dev machines) |
| | `workspace_steering` | | Path for copied steering files (CI) |
| | `agents_file` | | Platform-specific instructions file (e.g., `AGENTS.md`) |
| | `skill_dir` | `""` | Path for agent skill files. If empty, skill installation is skipped. |

### Timeout Budget

Configure timeouts in `kedge.toml`. Defaults fit within a 1-hour CI pipeline:

| Setting | Default | Worst case (5 docs) |
|---------|---------|---------------------|
| `triage_timeout` | 120s | ~10 min (serial) |
| `agent_timeout` | 300s | ~25 min per-doc, ~5 min batch |
| `git_timeout` | 300s | ~5 min (clone + fetch) |

Total worst case: ~40 min per-doc mode, ~20 min batch mode.

## Enterprise CI Integration

### GitLab CI

**MR pipeline** (gate on drift):

```yaml
kedge-check:
  stage: test
  script:
    - kedge check
  variables:
    KEDGE_DOCS_PATH: /path/to/docs     # or let kedge clone from [repos.docs]
    KEDGE_CODE_REPO_URL: $CI_PROJECT_URL
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

**Scheduled pipeline** (full detect -> triage -> remediate -> MR):

```yaml
kedge-update:
  stage: docs
  script:
    - kedge install --workspace --group $KEDGE_GROUP
    - kedge update --no-stamp
  variables:
    KEDGE_CODE_REPO_URL: $CI_PROJECT_URL
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
      when: always
```

`--no-stamp` skips provenance writes for `no_update` anchors. Run `kedge sync` in the docs repo after agent MRs merge to advance provenance in a single commit.

### GitHub Actions

```yaml
name: Documentation Drift
on:
  push:
    branches: [main]

jobs:
  kedge:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install kedge
        run: cargo install --path .
      - name: Run drift detection and remediation
        run: kedge update --no-stamp
        env:
          KEDGE_CODE_REPO_URL: ${{ github.server_url }}/${{ github.repository }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | If provider = `anthropic` | API key for Anthropic triage calls |
| `OPENAI_API_KEY` | If provider = `openai` | API key for OpenAI-compatible triage calls |
| `KEDGE_CODE_REPO_URL` | No | Override code repo URL (default: `file://<cwd>`) |
| `KEDGE_DOCS_PATH` | No | Override docs path (skips clone from `[[repos.docs]]`) |

Pass extra env vars to the agent process via `agent_env` in `kedge.toml`:

```toml
[remediation]
agent_command = "your-agent-command"
agent_env = { GITLAB_TOKEN = "${GITLAB_TOKEN}", DOCS_REPO = "git@gitlab.example.com:platform/docs.git" }
```

### Triage Providers

**`anthropic`** Direct Anthropic API. Set `api_url` to route through an enterprise proxy or API gateway.

**`openai`** Any OpenAI-compatible endpoint (Azure OpenAI, vLLM, etc.). Requires `model`. If `api_url` ends with `/v1`, kedge appends `/chat/completions`.

**`command`** Pipes the triage prompt to an external command via stdin. Set `triage_command` and, if needed, `triage_env`. The command must print a JSON array to stdout:

```json
[
  {"path": "src/Auth.java", "symbol": "Auth#validate", "severity": "minor"},
  {"path": "src/Baz.java", "symbol": null, "severity": "no_update"}
]
```

Each element needs `path`, `symbol` (string or null), and `severity` (`"no_update"`, `"minor"`, or `"major"`).

**`none`** Skips classification. All drifted anchors are set to `major` and forwarded to the remediation agent. Use this when your agent handles severity decisions based on org-specific rules.

## Steering File Format

Markdown files with `kedge:` frontmatter, stored in the docs repository:

```markdown
---
kedge:
  group: payments
  anchors:
    - repo: "git@gitlab.example.com:platform/services.git"
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: "sig:a1b2c3d4e5f67890"
---

# Authentication Token Validation

This document describes how token validation works...
```

| Field | Description |
|-------|-------------|
| `group` | Business unit grouping (used with `--group` flag to scope operations) |
| `anchors[].repo` | Git URL of the code repository this anchor points to |
| `anchors[].path` | File path within the code repository |
| `anchors[].symbol` | Optional. Specific symbol to track (e.g., `ClassName#methodName`) |
| `anchors[].provenance` | Content fingerprint (`sig:...`) or git SHA of last-known-good state |

### Provenance

Content-addressed provenance (`sig:` prefix) is the default. The fingerprint captures the AST structure at the anchored location:

- **Whitespace/comment immune.** Formatting changes don't trigger drift.
- **Rebase/amend/squash safe.** kedge computes fingerprints from code structure, not git history.
- **Symbol-scoped.** Tracks specific declarations (e.g., `AuthService#validateToken`), not entire files.

`kedge link` stamps initial provenance. `kedge sync` advances provenance without changing doc content.

Legacy SHA-based provenance (plain git commit hashes) requires git history traversal but kedge still supports it.

### Agent Response Contract

The agent receives a JSON payload on stdin and prints output to stdout. Two response formats:

**Structured JSON (preferred):**

```json
{"mr_url": "https://gitlab.example.com/platform/docs/-/merge_requests/42", "status": "success"}
```

For batch mode (`batch = true`):

```json
{"mr_urls": ["https://gitlab.example.com/.../merge_requests/42", "https://gitlab.example.com/.../merge_requests/43"], "status": "success"}
```

**Plain text fallback:**

kedge scans stdout for URLs starting with `https://` or `http://` and uses the first match as the MR link.

## Commands

kedge has two independent workflows:

**Drift pipeline.** Run in your code repo to detect stale docs, triage severity, and invoke agents to open MRs.

| Command | Description |
|---------|-------------|
| `kedge init` | Create a default `kedge.toml` in the current directory |
| `kedge check [--report <file>]` | Detect drift and output a report (exit 1 if drift found) |
| `kedge triage [--report <file>]` | Classify drift severity via AI (reads from stdin or file) |
| `kedge update [--report <file>] [--no-stamp]` | Full pipeline: detect, triage, invoke agent, open MR |
| `kedge status` | Show all anchors and their current state |
| `kedge link [files...]` | Stamp content-addressed provenance on doc anchors |
| `kedge sync [files...]` | Advance provenance without changing doc content |

**Steering distribution.** Copies or symlinks doc files from the docs repo into directories where agents read them. Independent of the drift pipeline. Use it to set up agent workspaces on dev machines or in CI before the agent runs.

| Command | Description |
|---------|-------------|
| `kedge install` | Pull steering files from docs repo to agent directories |

`--config <path>` overrides the default config file (`kedge.toml`) on all commands.

### Install Flags

| Flag | Description |
|------|-------------|
| `--group <name>` | Only install steering files for this business unit |
| `--agent <name>` | Target a specific agent platform (default: all configured) |
| `--link` | Symlink to global steering directory (dev machines) |
| `--workspace` | Copy to workspace steering directory (CI) |
| `--check` | Skip if already up to date (compare local vs remote HEAD) |
| `--recursive` | Include files from subdirectories within group/shared folders |

CI environments (`CI`, `GITHUB_ACTIONS`, or `GITLAB_CI` set) default to `--workspace` mode unless you pass `--link`.

## Supported Languages

AST fingerprinting (whitespace/comment immune, symbol-scoped):

| Language | Extensions | Symbol syntax |
|----------|------------|---------------|
| Java | `.java` | `ClassName#methodName` |
| Go | `.go` | `FunctionName` |
| TypeScript | `.ts`, `.tsx`, `.js`, `.jsx` | `ClassName#methodName` or `functionName` |
| Python | `.py` | `ClassName#method_name` or `function_name` |
| Rust | `.rs` | `StructName#method_name` or `function_name` |
| XML | `.xml` | (file-level only) |

Other file types fall back to SHA-256 content hashing. The fallback hashes raw content, so whitespace and comment changes register as drift.

## Security

- Git CLI calls use `--` end-of-options separator
- kedge validates anchor paths against path traversal
- kedge validates provenance values against git ref injection
- kedge validates repo URLs against injection and strips credentials from error output.
- kedge sets cache directories to `0o700` permissions
- `kedge install` skips symlinks when traversing source directories
- HTTP via rustls (no OpenSSL dependency)

## Development

```bash
cargo build                          # Debug build
cargo test                           # All tests
cargo test --test fingerprint_test   # Single test file
cargo clippy                         # Lint
cargo fmt                            # Format
```
