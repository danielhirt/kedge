# Steering File Format

Any markdown document with a `kedge:` block in its YAML frontmatter is a tracked doc. kedge borrows the term "steering file" from Amazon's Kiro agent, but the tool is agent-agnostic. It works with standalone docs, `AGENTS.md`, `CLAUDE.md`, or any markdown file. These files live in the docs repository and anchor documentation to specific code locations.

## Structure

```markdown
---
kedge:
  group: payments
  anchors:
    - repo: "git@github.com:your-org/services.git"
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: "sig:a1b2c3d4e5f67890"
    - repo: "git@github.com:your-org/services.git"
      path: src/auth/TokenStore.java
      provenance: "sig:0987654321fedcba"
---

# Authentication Token Validation

This document describes how token validation works...
```

## Frontmatter fields

### `kedge.group`

| | |
|---|---|
| Type | string (optional) |
| Purpose | Scopes the steering file to a business unit or team |

Used by `kedge install --group <name>` to install only relevant steering files. Also displayed by `kedge status`.

### `kedge.anchors`

An array of code anchors. Each anchor points to a specific location in a code repository.

#### `anchors[].repo`

| | |
|---|---|
| Type | string (required) |
| Purpose | Git URL of the code repository |

Must match the code repo URL that kedge resolves. kedge auto-detects this from `git remote get-url origin`; override with `KEDGE_CODE_REPO_URL` if needed. Anchors pointing to other repos are skipped during detection.

#### `anchors[].path`

| | |
|---|---|
| Type | string (required) |
| Purpose | File path within the code repository |

Relative to the repo root. Examples: `src/auth/AuthService.java`, `pkg/handler.go`.

kedge validates against path traversal. Paths containing `..` that escape the repo root are rejected.

#### `anchors[].symbol`

| | |
|---|---|
| Type | string (optional) |
| Purpose | Specific declaration to track within the file |

When set, kedge fingerprints only that symbol's AST subtree instead of the entire file. Changes to other parts of the file don't trigger drift.

Symbol syntax by language:

| Language | Class + method | Standalone function |
|----------|---------------|-------------------|
| Java | `ClassName#methodName` | -- |
| Go | -- | `FunctionName` |
| TypeScript | `ClassName#methodName` | `functionName` |
| Python | `ClassName#method_name` | `function_name` |
| Rust | `StructName#method_name` | `function_name` |
| XML | (not supported) | -- |

#### `anchors[].provenance`

| | |
|---|---|
| Type | string (required) |
| Purpose | Fingerprint of the code at the last-known-good state |

Two formats:

- **Content-addressed** (`sig:` prefix): `"sig:a1b2c3d4e5f67890"`, a 16-hex-char AST fingerprint. The default and recommended format.
- **Legacy SHA**: `"abc123def456..."`, a git commit hash (7+ hex characters). Requires git history traversal during detection.

Stamped by `kedge link` and advanced by `kedge sync`. Leave empty (`""`) when creating a new steering file, then run `kedge link` to populate it.

## Frontmatter parsing

kedge parses the YAML between the `---` delimiters. Only the `kedge:` key is read. Other frontmatter keys (e.g., `title`, `tags`) are preserved but kedge ignores them.

```yaml
---
title: Token Validation Guide
tags: [auth, security]
kedge:
  group: auth
  anchors:
    - repo: "git@github.com:your-org/services.git"
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: "sig:a1b2c3d4e5f67890"
---
```

## Provenance updates

When `kedge link` or `kedge sync` updates provenance, it modifies only the `provenance` field in the YAML. The rest of the frontmatter and document body are preserved.

## File discovery

kedge discovers steering files by:

1. Reading docs directories (from `KEDGE_DOCS_PATH` env var, or cloned from all `[[repos.docs]]` entries)
2. Scanning recursively for `*.md` files, skipping directories listed in `exclude_dirs` (defaults: `.git`, `node_modules`, `target`, `.venv`, `__pycache__`, `.tox`, `vendor`)
3. Parsing each file for `kedge:` frontmatter with non-empty `anchors`
4. Filtering by repo URL (only anchors matching the current code repo are processed)

kedge skips files without `kedge:` frontmatter or with empty anchors. See [Configuration](configuration.md#exclude_dirs) to customize the exclusion list.
