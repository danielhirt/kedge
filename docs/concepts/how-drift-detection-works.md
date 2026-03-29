# How Drift Detection Works

Detection is the first layer of kedge's pipeline. It is **deterministic and uses no AI**. It compares AST fingerprints to identify which documentation anchors have drifted from their last-known-good state.

## Overview

```
Steering files      Code repo (HEAD)
     │                     │
     ▼                     ▼
 Parse anchors      Compute fingerprints
     │                     │
     └─────── Compare ─────┘
                   │
                   ▼
            Drift report
```

## Step by step

### 1. Discover steering files

kedge scans the docs directory for `.md` files with `kedge:` frontmatter containing non-empty `anchors`. The docs directory comes from:

- `KEDGE_DOCS_PATH` environment variable, or
- Cloning the first `[[repos.docs]]` entry in `kedge.toml`

### 2. Filter relevant anchors

Each anchor has a `repo` field. kedge only processes anchors whose `repo` matches the current code repository URL (from `KEDGE_CODE_REPO_URL` or `file://<cwd>`). Anchors pointing to other repos are skipped.

### 3. Compute current fingerprints

For each relevant anchor, kedge computes a fingerprint of the code **as it exists at HEAD**:

1. Determine the language from the file extension
2. Parse the file with tree-sitter (or fall back to content hashing)
3. If a `symbol` is specified, navigate to that symbol's AST subtree
4. Walk the AST, hashing node kinds and leaf text while skipping comments
5. Produce a `sig:<16-hex-chars>` fingerprint

### 4. Compare against provenance

The comparison depends on the provenance format:

**Content-addressed (`sig:` prefix):**

kedge compares the current fingerprint against the stored `sig:` value. If they differ, the anchor has drifted. No git history is needed.

**Legacy SHA-based (hex commit hash):**

1. Read the file at the provenance commit: `git show <sha>:<path>`
2. Read the file at HEAD: `git show HEAD:<path>`
3. Compute fingerprints for both versions
4. Compare the two fingerprints

If the fingerprints differ, the anchor has drifted.

### 5. Build the drift report

kedge partitions docs into `drifted` and `clean`:

- **Drifted:** at least one anchor's fingerprint has changed
- **Clean:** all anchors match their stored provenance

For legacy SHA provenance, the drift report includes a git diff and diff summary. For `sig:` provenance, the diff is empty and the summary is `"content fingerprint changed"`.

## Security validations

During detection, kedge validates:

- **Provenance values** must be hex-only (with `sig:` prefix) or a valid git SHA (7+ hex chars). kedge rejects anything that could be a git flag or arbitrary expression.
- **Anchor paths** must resolve within the code repository root. kedge rejects paths with `..` that escape the repo.

## What triggers drift

| Change type | AST fingerprinting | Content-hash fallback |
|-------------|-------------------|----------------------|
| Method signature change | Drift detected | Drift detected |
| New parameter added | Drift detected | Drift detected |
| Logic change inside a method | Drift detected | Drift detected |
| Whitespace / formatting | **No drift** | Drift detected |
| Comment added / removed | **No drift** | Drift detected |
| Change to a different method in same file | **No drift** (symbol-scoped) | Drift detected |

## Performance

Detection is fast because:

- Tree-sitter parsing is native (compiled C grammars called from Rust)
- `sig:` provenance requires no git history traversal
- Each anchor is independent, with no cross-file analysis
- The operation is fully deterministic with no network calls (git operations are local)
