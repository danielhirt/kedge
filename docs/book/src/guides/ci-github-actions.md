# GitHub Actions Integration

kedge integrates with GitHub Actions as a **push-triggered check** for drift detection and a **scheduled workflow** for full remediation.

## Drift detection on push

Run `kedge check` on every push to the default branch:

```yaml
name: Documentation Drift Check
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  kedge-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0    # full history needed for legacy SHA provenance
      - name: Install kedge
        run: |
          curl -fsSL https://raw.githubusercontent.com/danielhirt/kedge/main/install.sh | sh
      - name: Check for drift
        run: kedge check
        env:
          KEDGE_CODE_REPO_URL: ${{ github.server_url }}/${{ github.repository }}
```

> **Note:** `fetch-depth: 0` is only required if you use legacy SHA-based provenance. With content-addressed `sig:` provenance, a shallow clone is sufficient.

## Full pipeline on schedule

Run detect-triage-remediate on a cron schedule:

```yaml
name: Documentation Remediation
on:
  schedule:
    - cron: '0 6 * * 1'   # every Monday at 06:00 UTC
  workflow_dispatch:        # allow manual trigger

jobs:
  kedge-update:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install kedge
        run: |
          curl -fsSL https://raw.githubusercontent.com/danielhirt/kedge/main/install.sh | sh
      - name: Install steering files
        run: kedge install --workspace
      - name: Run full pipeline
        run: kedge update
        env:
          KEDGE_CODE_REPO_URL: ${{ github.server_url }}/${{ github.repository }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

## PR status check

Use `kedge check` as a required status check on pull requests. When drift is detected (exit code `1`), the PR is blocked until the author either updates the docs or runs `kedge sync`.

```yaml
name: Kedge PR Gate
on:
  pull_request:
    branches: [main]

jobs:
  drift-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install kedge
        run: |
          curl -fsSL https://raw.githubusercontent.com/danielhirt/kedge/main/install.sh | sh
      - name: Drift check
        run: kedge check --report drift-report.json
        env:
          KEDGE_CODE_REPO_URL: ${{ github.server_url }}/${{ github.repository }}
      - name: Upload drift report
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: drift-report
          path: drift-report.json
```

## Using Docker instead of the installer

If you prefer not to run the shell installer:

```yaml
jobs:
  kedge-check:
    runs-on: ubuntu-latest
    container:
      image: danielhirt/kedge:latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - run: kedge check
        env:
          KEDGE_CODE_REPO_URL: ${{ github.server_url }}/${{ github.repository }}
```

## Secrets

Store API keys as [repository secrets](https://docs.github.com/en/actions/security-for-github-actions/security-guides/using-secrets-in-github-actions):

| Secret | When needed |
|--------|-------------|
| `ANTHROPIC_API_KEY` | Triage provider is `anthropic` |
| `OPENAI_API_KEY` | Triage provider is `openai` |

## Environment variables

| Variable | Purpose |
|----------|---------|
| `KEDGE_CODE_REPO_URL` | Override code repo URL (set to `${{ github.server_url }}/${{ github.repository }}`) |
| `KEDGE_DOCS_PATH` | Use a local docs path instead of cloning from config |

kedge auto-detects the `GITHUB_ACTIONS` environment variable and defaults `kedge install` to `--workspace` mode.
