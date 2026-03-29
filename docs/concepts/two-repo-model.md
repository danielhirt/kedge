# Two-Repo Model

kedge is designed around a separation between the **code repository** and the **docs repository**. Steering files live in the docs repo and point to code locations via anchors. The code repo contains `kedge.toml` and is where kedge runs.

## Why separate repos?

### Independent lifecycles

Code and documentation often follow different review and release processes. A code change might need urgent deployment while the corresponding doc update goes through a technical writing review. Separate repos let each proceed at its own pace.

### Different ownership

Code is owned by engineers; documentation may be owned by technical writers, product teams, or developer relations. Separate repos allow different access controls and review workflows.

### Agent workspace isolation

When kedge invokes an agent to update docs, the agent operates on the docs repo. Keeping docs in a separate repo means the agent doesn't need write access to the code repo.

## How it works

```
Code repo (kedge.toml)              Docs repo (steering files)
┌──────────────────────┐            ┌─────────────────────────┐
│ src/                 │            │ steering/               │
│   auth/              │◄───anchor──│   auth-validation.md    │
│     AuthService.java │            │   payment-flow.md       │
│   pay/               │◄───anchor──│   shared/               │
│     PaymentService.java           │     api-conventions.md  │
│ kedge.toml           │            │   _kedge/               │
└──────────────────────┘            │     AGENTS.md           │
                                    └─────────────────────────┘
```

### The code repo

- Contains the source code being tracked
- Contains `kedge.toml` with configuration
- Where you run `kedge check`, `kedge update`, and other commands
- Doesn't need to contain any documentation

### The docs repo

- Contains steering `.md` files with `kedge:` frontmatter
- Each anchor's `repo` field points to the code repo's git URL
- May contain multiple group directories and shared docs
- May contain `_kedge/` metadata (agent instructions, skills)

## Bridging the repos

### Anchors

The `repo` field in each anchor is the bridge. It tells kedge which code repo to match against:

```yaml
kedge:
  anchors:
    - repo: "git@github.com:your-org/services.git"
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: "sig:a1b2c3d4e5f67890"
```

When kedge runs in the code repo, it resolves the current repo URL (from `KEDGE_CODE_REPO_URL` or `file://<cwd>`) and only processes anchors whose `repo` matches.

### Cloning the docs repo

kedge needs access to both repos during detection. Configure docs repos in `kedge.toml` under `[[repos.docs]]`. kedge clones them automatically. CI pipelines only need the code repo checked out.

```toml
[[repos.docs]]
url = "git@github.com:your-org/docs.git"
path = "steering"
ref = "main"
```

When multiple doc repos are configured, `kedge check` and `kedge update` scan all of them and merge the results into a single drift report. Cloned repos are cached in `~/.cache/kedge/repos/` (keyed by URL and ref) and fetched on subsequent runs. The cache uses `0o700` permissions.

Set `KEDGE_DOCS_PATH` to skip the clone and use a local directory instead. This is useful for local testing or monorepos. In a two-repo setup with `KEDGE_DOCS_PATH`, also set `KEDGE_DOCS_REPO_URL` so agent payloads contain the correct docs repo URL.

### `kedge install`

The `install` command copies or symlinks steering files from the docs repo to agent workspace directories in the code repo:

```bash
# Dev machine: symlink to global agent directory
kedge install --link --group payments

# CI: copy to workspace agent directory
kedge install --workspace --group payments
```

Agents (Kiro, Claude Code, etc.) gain access to the steering context without needing to clone the docs repo themselves.

## Monorepo alternative

If your code and docs are in the same repository, the two-repo model still works:

1. Set `KEDGE_DOCS_PATH` to the docs directory within your repo
2. Set anchor `repo` fields to match your repo URL
3. Use a single `[[repos.docs]]` entry pointing to the same repo

```toml
[[repos.docs]]
url = "git@github.com:your-org/monorepo.git"
path = "docs/steering/"
ref = "main"
```

Or:

```bash
KEDGE_DOCS_PATH=./docs/steering kedge check
```

## Multiple code repos

A single docs repo can contain steering files anchoring to different code repos. Each anchor specifies its `repo`:

```yaml
kedge:
  anchors:
    - repo: "git@github.com:your-org/auth-service.git"
      path: src/AuthService.java
      symbol: AuthService#validateToken
      provenance: "sig:a1b2c3d4e5f67890"
    - repo: "git@github.com:your-org/payment-service.git"
      path: src/PaymentService.java
      symbol: PaymentService#process
      provenance: "sig:0987654321fedcba"
```

When kedge runs in `auth-service`, it only processes the first anchor. When it runs in `payment-service`, it only processes the second. This lets a single steering file track cross-service documentation.
