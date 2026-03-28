# GitLab CI Integration

kedge fits into GitLab CI in two ways: as an **MR gate** that blocks merges when docs drift, and as a **scheduled pipeline** that automatically remediates drift.

## MR gate: block merges on drift

Add `kedge check` to your merge request pipeline. It exits `1` when drift is detected, failing the pipeline:

```yaml
kedge-check:
  stage: test
  image: danielhirt/kedge:latest    # or your runner with kedge installed
  script:
    - kedge check
  variables:
    KEDGE_CODE_REPO_URL: $CI_PROJECT_URL
    KEDGE_DOCS_PATH: /path/to/docs  # or omit to let kedge clone from [repos.docs]
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

### How it works

1. kedge reads steering files from `KEDGE_DOCS_PATH` (or clones the docs repo from `kedge.toml`)
2. For each anchor, it computes the current AST fingerprint and compares it to the stored provenance
3. If any anchor has drifted, the job fails with a JSON drift report on stdout

### Handling drift in MRs

When the check fails, the developer has two options:

- **Update the docs** -- edit the steering file content and run `kedge link` to stamp fresh provenance
- **Acknowledge no doc change is needed** -- run `kedge sync` to advance provenance without changing doc content

Either way, the provenance is updated and the next pipeline run will pass.

## Scheduled pipeline: full remediation

Run the complete detect-triage-remediate pipeline on a schedule (e.g., nightly or weekly):

```yaml
kedge-update:
  stage: docs
  image: danielhirt/kedge:latest
  script:
    - kedge install --workspace --group $KEDGE_GROUP
    - kedge update
  variables:
    KEDGE_CODE_REPO_URL: $CI_PROJECT_URL
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
      when: always
```

### What `kedge update` does

1. **Detection** -- scans all steering files and identifies drifted anchors
2. **Triage** -- sends each drifted doc to the AI provider for severity classification
3. **Remediation** -- invokes the agent command for `minor` and `major` docs; auto-advances provenance for `no_update` docs
4. Outputs a `RemediationSummary` JSON with MR URLs, synced provenance, and any errors

### Installing steering files in CI

`kedge install --workspace` copies doc files from the docs repo into the workspace agent directories (e.g., `.kiro/steering/` for Kiro, `docs/` for Claude Code). This gives the agent access to the documentation context when it runs.

Use `--group` to scope to a specific team:

```yaml
kedge install --workspace --group payments
```

Use `--check` to skip installation if the docs repo hasn't changed:

```yaml
kedge install --workspace --check
```

## CI environment detection

kedge auto-detects CI environments by checking for `CI`, `GITHUB_ACTIONS`, or `GITLAB_CI` environment variables. In CI, `kedge install` defaults to `--workspace` mode (copy) unless `--link` is explicitly passed.

## Variables reference

| Variable | Purpose |
|----------|---------|
| `KEDGE_CODE_REPO_URL` | Override the code repo URL (default: `file://<cwd>`) |
| `KEDGE_DOCS_PATH` | Use a local docs path instead of cloning from config |
| `ANTHROPIC_API_KEY` | API key for Anthropic triage provider |
| `OPENAI_API_KEY` | API key for OpenAI-compatible triage provider |

Store API keys as [CI/CD variables](https://docs.gitlab.com/ee/ci/variables/) with the **Masked** flag enabled.

## Full example

```yaml
stages:
  - test
  - docs

# Gate: run on every MR
kedge-check:
  stage: test
  image: danielhirt/kedge:latest
  script:
    - kedge check
  variables:
    KEDGE_CODE_REPO_URL: $CI_PROJECT_URL
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"

# Remediation: run on default branch
kedge-update:
  stage: docs
  image: danielhirt/kedge:latest
  script:
    - kedge install --workspace
    - kedge update
  variables:
    KEDGE_CODE_REPO_URL: $CI_PROJECT_URL
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
  rules:
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
      when: always
```
