# Environment Variables

kedge reads the following environment variables. None are required for basic local usage.

## API keys

| Variable | When required | Description |
|----------|--------------|-------------|
| `ANTHROPIC_API_KEY` | `provider = "anthropic"` | API key for Anthropic triage calls. Override the env var name with `api_key_env` in config. |
| `OPENAI_API_KEY` | `provider = "openai"` | API key for OpenAI-compatible triage calls. Override the env var name with `api_key_env` in config. |

### Custom key env var

If your API key is stored in a differently-named variable:

```toml
[triage]
provider = "anthropic"
api_key_env = "MY_CUSTOM_ANTHROPIC_KEY"
```

kedge reads `MY_CUSTOM_ANTHROPIC_KEY` instead of `ANTHROPIC_API_KEY`.

## Repository overrides

| Variable | Default | Description |
|----------|---------|-------------|
| `KEDGE_CODE_REPO_URL` | (auto-detected) | Override the code repo URL used to match anchors. Rarely needed; see below. |
| `KEDGE_DOCS_PATH` | (clone from config) | Local path to the docs directory. When set, kedge skips cloning from `[[repos.docs]]` and reads steering files from this path instead. |

### `KEDGE_CODE_REPO_URL`

kedge uses this to match steering file anchors to the current repository. An anchor is only processed if its `repo` field matches this URL.

kedge resolves the code repo URL in this order:

1. `KEDGE_CODE_REPO_URL` environment variable (if set)
2. `git remote get-url origin` (auto-detected from the repo)
3. `file://<cwd>` (last resort fallback)

`origin` typically points to the same HTTPS or SSH URL used in anchor `repo` fields, so kedge matches anchors without configuration on developer machines. Set `KEDGE_CODE_REPO_URL` when:

- **CI pipelines** — CI runners often set `origin` to a URL that doesn't match your anchors (e.g., `actions/checkout` uses `https://github.com/org/repo` without `.git`, GitLab runners prepend `gitlab-ci-token@`)
- The repo has no `origin` remote
- `origin` uses a different protocol than your anchors (e.g., you cloned via SSH but anchors use HTTPS)

```yaml
# Example: CI override when needed
# GitLab CI
variables:
  KEDGE_CODE_REPO_URL: $CI_PROJECT_URL

# GitHub Actions
env:
  KEDGE_CODE_REPO_URL: ${{ github.server_url }}/${{ github.repository }}
```

### `KEDGE_DOCS_PATH`

kedge reads steering files from this local path instead of cloning from `[[repos.docs]]`. Local changes (including `kedge link` stamps) are visible to `kedge check` immediately.

Use this when:

- **Monorepos** — docs and code live in the same repository. `[[repos.docs]]` reads from a remote-fetched cache and won't see local changes.
- **Local development** — you want to test against local doc edits without pushing first.
- **Pre-cloned CI** — the docs repo is already checked out in your pipeline.

```bash
KEDGE_DOCS_PATH=./docs/steering kedge check
```

## CI detection

kedge detects CI environments by checking for these variables (existence, not value):

| Variable | Environment |
|----------|-------------|
| `CI` | Generic CI |
| `GITHUB_ACTIONS` | GitHub Actions |
| `GITLAB_CI` | GitLab CI |

When any of these is set, `kedge install` defaults to `--workspace` mode (copy) instead of `--link` mode (symlink).

## Agent environment

Extra variables can be passed to the agent process via `agent_env` in `kedge.toml`:

```toml
[remediation]
agent_env = {
  GITLAB_TOKEN = "${GITLAB_TOKEN}",
  DOCS_REPO = "git@github.com:your-org/docs.git"
}
```

Similarly, extra variables for the `command` triage provider:

```toml
[triage]
triage_env = {
  CUSTOM_VAR = "value"
}
```

kedge passes these to the subprocess environment alongside any inherited variables.
