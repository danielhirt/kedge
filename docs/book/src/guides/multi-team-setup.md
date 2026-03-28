# Multi-Team Setup

kedge supports multiple teams sharing a single docs repository by using **groups** to scope steering files. Each team owns its own set of docs, and `kedge install` distributes only the relevant files to each team's agent workspace.

## Directory structure

Organize steering files in the docs repository by group:

```
docs-repo/
  steering/
    payments/
      payment-flow.md
      refund-handling.md
    auth/
      token-validation.md
      session-management.md
    shared/
      api-conventions.md
      error-handling.md
    _kedge/
      AGENTS.md
      skill.md
```

- **`payments/`**, **`auth/`** -- group-scoped directories, one per team
- **`shared/`** -- files installed for every group
- **`_kedge/`** -- metadata files (agent instructions, skills) installed for all groups

## Steering file groups

Each steering file declares its group in the frontmatter:

```yaml
kedge:
  group: payments
  anchors:
    - repo: "git@github.com:your-org/services.git"
      path: src/payments/PaymentService.java
      symbol: PaymentService#processRefund
      provenance: "sig:a1b2c3d4e5f67890"
```

The `group` field is used by:

- **`kedge install --group payments`** -- only installs steering files from `payments/` and `shared/`
- **`kedge status`** -- displays the group for each doc

## Installing per team

On a developer machine, install steering files for your team:

```bash
kedge install --link --group payments
```

This symlinks files from `payments/` and `shared/` into your local agent steering directory (e.g., `~/.claude/steering/`).

In CI, use `--workspace` to copy instead:

```bash
kedge install --workspace --group payments
```

## Targeting specific agents

If your team uses Kiro but another team uses Claude Code, target a specific agent platform:

```bash
kedge install --workspace --group payments --agent kiro
```

Without `--agent`, kedge installs to all configured agent platforms.

## Cross-cutting documentation

Files in the `shared/` directory are installed regardless of which `--group` is specified. Use this for:

- API conventions that apply across all teams
- Shared error handling patterns
- Company-wide coding standards

## Agent instructions

Place an `AGENTS.md` file in `_kedge/` to provide agent-specific instructions. When installed, this file is renamed to match the platform's `agents_file` config:

- For Kiro: installed as `AGENTS.md`
- For Claude Code: installed as `CLAUDE.md`

Similarly, a `skill.md` in `_kedge/` is installed into the platform's `skill_dir`.

## Multiple docs repositories

`kedge.toml` supports multiple docs repos:

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

Each repo is cloned and processed independently. Steering files from all repos are scanned during `kedge check` and `kedge update`.

## Example: CI with multiple groups

Run separate pipelines per team using CI variables:

```yaml
# GitLab CI
kedge-update-payments:
  stage: docs
  script:
    - kedge install --workspace --group payments
    - kedge update
  variables:
    KEDGE_GROUP: payments
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY

kedge-update-auth:
  stage: docs
  script:
    - kedge install --workspace --group auth
    - kedge update
  variables:
    KEDGE_GROUP: auth
    ANTHROPIC_API_KEY: $ANTHROPIC_API_KEY
```

Or install all groups at once by omitting `--group`:

```bash
kedge install --workspace
```

This installs files from every group directory plus `shared/`.
