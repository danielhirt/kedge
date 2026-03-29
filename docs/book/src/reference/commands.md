# CLI Commands

All commands accept `--config <path>` to override the default config file location (`kedge.toml`).

## `kedge init`

Create a default `kedge.toml` in the current directory.

```bash
kedge init
```

If `kedge.toml` already exists, prints a message and exits without overwriting.

The generated template includes all sections with commented-out optional fields and sensible defaults.

## `kedge link [files...]`

Stamp content-addressed provenance on steering file anchors.

```bash
kedge link                    # all .md files with kedge frontmatter
kedge link docs/auth.md       # specific file
```

For each anchor in each steering file:
1. Reads the anchored code file from the filesystem
2. Computes an AST fingerprint (or content hash for unsupported languages)
3. Writes the `sig:<hex>` value into the anchor's `provenance` field

**Output:** One line per file showing how many anchors were stamped.

```
Linked docs/auth.md (2/2 anchors stamped)
Linked docs/payment.md (1/1 anchors stamped)
```

Use this after creating new steering files or after intentionally updating docs to match current code.

## `kedge check [--report <file>]`

Detect drift and output a JSON report.

```bash
kedge check                       # report to stdout
kedge check --report drift.json   # report to file
```

**Exit codes:**
- `0`: no drift detected, all docs are up to date
- `1`: drift detected, at least one anchor has changed

**Output:** A `DriftReport` JSON object:

```json
{
  "repo": "my-service",
  "ref": "HEAD",
  "commit": "abc123...",
  "drifted": [
    {
      "doc": "steering/auth.md",
      "doc_repo": "git@github.com:org/docs.git",
      "anchors": [
        {
          "path": "src/auth/AuthService.java",
          "symbol": "AuthService#validateToken",
          "provenance": "sig:a1b2c3d4e5f67890",
          "current_commit": "abc123...",
          "diff_summary": "content fingerprint changed",
          "diff": ""
        }
      ]
    }
  ],
  "clean": [
    {
      "doc": "steering/payment.md",
      "anchor_count": 1
    }
  ]
}
```

For content-addressed (`sig:`) provenance, the `diff` field is empty and `diff_summary` is `"content fingerprint changed"`. For legacy SHA provenance, both fields contain git diff output.

## `kedge triage [--report <file>]`

Classify drift severity via AI.

```bash
kedge check --report drift.json && kedge triage --report drift.json
kedge check | kedge triage        # pipe from stdin
```

Reads a `DriftReport` from a file (`--report`) or stdin. Sends each drifted doc to the configured triage provider for severity classification.

**Output:** A `TriagedReport` JSON object:

```json
{
  "repo": "my-service",
  "ref": "HEAD",
  "commit": "abc123...",
  "drifted": [
    {
      "doc": "steering/auth.md",
      "doc_repo": "git@github.com:org/docs.git",
      "severity": "minor",
      "summary": "steering/auth.md: 1 anchor(s) classified",
      "anchors": [
        {
          "path": "src/auth/AuthService.java",
          "symbol": "AuthService#validateToken",
          "severity": "minor",
          "provenance": "sig:a1b2c3d4e5f67890",
          "diff": ""
        }
      ]
    }
  ]
}
```

The doc-level `severity` is the maximum of its anchor severities.

## `kedge update [--report <file>]`

Run the full three-layer pipeline: detection, triage, and remediation.

```bash
kedge update
kedge update --report drift.json   # also save the drift report
```

**Pipeline steps:**

1. **Detection**: scans steering files and computes drift
2. **Triage**: classifies each drifted anchor via the AI provider
3. **Remediation**: for docs with `minor` or `major` anchors, invokes the agent command; for docs where all anchors are `no_update`, advances provenance

**Output:** A `RemediationSummary` JSON object:

```json
{
  "remediated": [
    {
      "doc": "steering/auth.md",
      "repo": "git@github.com:org/docs.git",
      "mr_url": "https://github.com/org/docs/pull/42",
      "severity": "minor",
      "auto_merged": false
    }
  ],
  "provenance_advanced": [
    {
      "doc": "steering/logging.md",
      "anchors_synced": 2,
      "reason": "no_update — code changes did not affect documentation accuracy"
    }
  ],
  "errors": []
}
```

`mr_url` is the merge request URL returned by the agent. `auto_merged` reflects the `auto_merge` signal sent to the agent based on severity config. kedge does not monitor or act on MRs after this point.

If no drift is detected, prints `"No drift detected."` and exits.

## `kedge status`

Show all steering files and their anchors.

```bash
kedge status
```

Reads from `KEDGE_DOCS_PATH` (or defaults to `docs/` directory).

**Output:**

```
steering/auth.md [group: auth]
  anchor: src/auth/AuthService.java#AuthService#validateToken  provenance: sig:a1b2c3d4e5f67890
  anchor: src/auth/TokenStore.java  provenance: sig:0987654321fedcba
steering/payment.md [group: payments]
  anchor: src/pay/PaymentService.java#PaymentService#process  provenance: sig:1122334455667788
```

## `kedge sync [files...]`

Advance provenance markers without changing documentation content.

```bash
kedge sync                    # all .md files with kedge frontmatter
kedge sync docs/auth.md       # specific file
```

Behaves like `kedge link`: recomputes fingerprints and writes them to the steering files. Use this when code has changed but the documentation is still accurate (e.g., internal refactors that don't affect the public API).

**Output:**

```
Synced 3 anchors with content-addressed provenance
```

## `kedge install`

Pull steering files from the docs repository to local agent directories.

```bash
kedge install --link                          # symlink to global dirs (dev)
kedge install --workspace                     # copy to workspace dirs (CI)
kedge install --workspace --group payments    # only the payments group
kedge install --workspace --agent kiro        # only the kiro platform
kedge install --workspace --check             # skip if already up to date
```

### Flags

| Flag | Description |
|------|-------------|
| `--group <name>` | Only install steering files for this group (plus `shared/`) |
| `--agent <name>` | Target a specific agent platform (default: all configured in `[[agents]]`) |
| `--link` | Symlink to the global steering directory. For dev machines. |
| `--workspace` | Copy to the workspace steering directory. For CI. |
| `--check` | Compare local cache against remote HEAD; skip if already up to date. |
| `--recursive` | Include files from subdirectories within group and shared folders. |

`--link` and `--workspace` are mutually exclusive. In CI environments (`CI`, `GITHUB_ACTIONS`, or `GITLAB_CI` env vars set), `--workspace` is the default unless `--link` is explicitly passed.

### What gets installed

From the docs repo source directory:

- `<group>/`: markdown files matching the specified group
- `shared/`: markdown files shared across all groups
- `_kedge/AGENTS.md`: installed as the platform's `agents_file` (e.g., renamed to `CLAUDE.md` for the claude platform)
- `_kedge/skill.md`: installed into the platform's `skill_dir`

Without `--recursive`, kedge installs only top-level `.md` files from each directory. With it, kedge walks subdirectories and preserves the directory structure in the target:

```bash
# Source: payments/services/auth/auth.md
# Target: .kiro/steering/services/auth/auth.md
kedge install --workspace --group payments --recursive
```

kedge skips symlinks in the source directory for security, including within subdirectories during `--recursive` walks.

In `--workspace` mode, kedge also adds the workspace steering directory to `.git/info/exclude` to avoid accidentally committing copied files.
