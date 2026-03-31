# Two-Repo Model

kedge separates the **code repository** from the **docs repository**. Steering files live in the docs repo and point to code locations via anchors. The code repo contains `kedge.toml` and is where you run kedge.

## Why separate repos?

### Independent lifecycles

Code and documentation follow different review and release processes. A code change might need urgent deployment while the corresponding doc update goes through technical writing review. Separate repos let each proceed at its own pace.

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

When kedge runs in the code repo, it resolves the current repo URL and only processes anchors whose `repo` matches. The URL is resolved in this order:

1. `KEDGE_CODE_REPO_URL` environment variable (if set)
2. `git remote get-url origin` (auto-detected from the repo)
3. `file://<cwd>` (last resort fallback)

Your anchors typically use the same HTTPS or SSH URL as your `origin` remote, so kedge matches them without configuration.

### Cloning the docs repo

kedge needs access to both repos during detection. Configure docs repos in `kedge.toml` under `[[repos.docs]]`:

```toml
[[repos.docs]]
url = "git@github.com:your-org/docs.git"
path = "steering"
ref = "main"
```

**Runtime behavior:** On every `kedge check` or `kedge update`, kedge clones the configured repo to `~/.cache/kedge/repos/` (first run) or fetches and hard-resets to the latest commit on the configured `ref` (subsequent runs). Docs are read from this cached clone, not from your working tree. Consequences:

- CI pipelines only need the code repo checked out — kedge fetches the docs repo automatically.
- Local changes to doc files are not visible to `kedge check` until they are pushed to the remote at the configured `ref`.
- The `ref` field controls which branch is fetched. If `ref = "main"`, kedge always reads docs from `main`, even if you're on a different branch.

When multiple doc repos are configured, `kedge check` and `kedge update` scan all of them and merge the results into a single drift report.

### Using a local docs directory

Set `KEDGE_DOCS_PATH` to skip the clone and read docs directly from a local directory:

```bash
KEDGE_DOCS_PATH=./path/to/steering kedge check
```

kedge reads steering files from your working tree, so local changes (including `kedge link` stamps) are visible immediately. Use this for local development, monorepos, or pre-cloned CI setups.

In a two-repo setup with `KEDGE_DOCS_PATH`, also set `KEDGE_DOCS_REPO_URL` so agent payloads contain the correct docs repo URL instead of defaulting to the code repo URL.

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

If your code and docs are in the same repository, use `KEDGE_DOCS_PATH` to point kedge at the docs directory:

```bash
KEDGE_DOCS_PATH=./docs/steering kedge check
```

kedge auto-detects the repo URL from `git remote get-url origin`, so anchors using your HTTPS or SSH remote URL match automatically. The full local workflow:

1. Add `kedge:` frontmatter to a steering file, with anchor `repo` fields matching your remote URL
2. Run `kedge link` to stamp provenance
3. Edit code
4. Run `kedge check` — drift is detected immediately, even for uncommitted changes

!!! warning "Don't use `[[repos.docs]]` for monorepo local development"
    `[[repos.docs]]` fetches from the remote and reads from a cached clone. Local changes to docs (such as `kedge link` stamps) are invisible to `kedge check` until pushed. PR branches that modify steering files won't see their own changes if `ref` points to a different branch.

    Use `KEDGE_DOCS_PATH` instead — it reads directly from your working tree.

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
