# Provenance

Provenance is the fingerprint stored in a steering file anchor that represents the last-known-good state of the code. When kedge detects that the current code no longer matches the provenance, it reports drift.

## Content-addressed provenance (`sig:`)

This is the default and recommended format. The provenance value is a 16-hex-character AST fingerprint prefixed with `sig:`:

```yaml
provenance: "sig:a1b2c3d4e5f67890"
```

### How it's computed

1. Parse the source file with tree-sitter
2. If a symbol is specified, navigate to that subtree
3. Walk the AST, feeding each node's kind and leaf text into a SHA-256 hasher (skipping comment nodes)
4. Truncate the hash to 16 hex characters
5. Prefix with `sig:`

### Properties

**Whitespace/comment immune.** Only the code structure (AST node types and identifiers) is hashed. Reformatting, adding comments, or changing indentation doesn't change the fingerprint.

**Rebase/amend/squash safe.** The fingerprint is derived from the code's structure, not from git history. Rewriting history doesn't invalidate provenance.

**Symbol-scoped.** When tracking `AuthService#validateToken`, changes to other methods in `AuthService.java` don't affect the fingerprint.

**No git history needed.** Detection compares the stored `sig:` value directly against the current file's fingerprint. No `git show` or `git diff` is required.

### 64 bits of collision resistance

The `sig:` value is 16 hex characters = 64 bits. This is not cryptographic security -- it's drift detection. The probability of an accidental collision is negligible for this use case, and the truncation keeps frontmatter readable.

## Legacy SHA-based provenance

kedge also supports plain git commit hashes as provenance:

```yaml
provenance: "abc123def4567890abcdef1234567890abcdef12"
```

### How detection works with SHA provenance

1. Read the file at the provenance commit: `git show <sha>:<path>`
2. Read the file at HEAD: `git show HEAD:<path>`
3. Compute AST fingerprints for both versions
4. If the fingerprints differ, the anchor has drifted

This requires the git history to contain the provenance commit, so shallow clones may not work. The detection output includes a full git diff for SHA-based anchors.

### When to use SHA provenance

SHA provenance is supported for backward compatibility. Prefer `sig:` provenance for new steering files. Migration is simple -- run `kedge link` to replace SHA values with `sig:` fingerprints.

## Stamping provenance

### `kedge link`

Computes the current fingerprint for each anchor and writes it to the steering file:

```bash
kedge link
# Linked docs/auth.md (2/2 anchors stamped)
```

Use after:
- Creating a new steering file (initial stamp)
- Intentionally updating documentation to match current code

### `kedge sync`

Same as `link` -- recomputes fingerprints and writes them. The distinction is semantic: `sync` is for advancing provenance when code changed but the docs are still accurate (e.g., internal refactors).

```bash
kedge sync
# Synced 3 anchors with content-addressed provenance
```

### Automatic advancement during `kedge update`

During the full pipeline, anchors triaged as `no_update` have their provenance automatically advanced without invoking the agent. This is logged in the `provenance_advanced` field of the `RemediationSummary`.

## Provenance lifecycle

```
1. Create steering file with empty provenance
       │
       ▼
2. kedge link  →  stamps sig:abc123...
       │
       ▼
3. Code changes over time
       │
       ▼
4. kedge check  →  detects drift (sig doesn't match)
       │
       ├─ kedge update  →  agent fixes docs, kedge re-stamps provenance
       │
       └─ kedge sync    →  manually advance provenance (docs still accurate)
       │
       ▼
5. Back to step 3
```
