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

kedge will read `MY_CUSTOM_ANTHROPIC_KEY` instead of `ANTHROPIC_API_KEY`.

## Repository overrides

| Variable | Default | Description |
|----------|---------|-------------|
| `KEDGE_CODE_REPO_URL` | `file://<cwd>` | URL of the code repository. Set this in CI to match the repo URL in your steering file anchors. |
| `KEDGE_DOCS_PATH` | (clone from config) | Local path to the docs directory. When set, kedge skips cloning from `[[repos.docs]]` and reads steering files from this path instead. |

### `KEDGE_CODE_REPO_URL`

kedge uses this to match steering file anchors to the current repository. An anchor is only processed if its `repo` field matches this URL.

In local development, the default `file://<cwd>` works when anchors use the same format. In CI, set it to match your anchor URLs:

```yaml
# GitLab CI
variables:
  KEDGE_CODE_REPO_URL: $CI_PROJECT_URL

# GitHub Actions
env:
  KEDGE_CODE_REPO_URL: ${{ github.server_url }}/${{ github.repository }}
```

### `KEDGE_DOCS_PATH`

Useful when:

- The docs repo is already checked out as part of your CI pipeline
- You're testing locally and don't want kedge to clone the docs repo
- You have a monorepo where docs and code are in the same repository

```bash
KEDGE_DOCS_PATH=./docs kedge check
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

These are passed directly to the subprocess environment alongside any inherited variables.
