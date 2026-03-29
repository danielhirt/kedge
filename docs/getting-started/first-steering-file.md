# Your First Steering File

kedge tracks any markdown document that has a `kedge:` block in its YAML frontmatter. The project calls these "steering files" (a term borrowed from Amazon's Kiro agent). They can be standalone docs, `AGENTS.md`, `CLAUDE.md`, or any other markdown file. When the anchored code changes, kedge detects the drift.

## Anatomy of a steering file

```markdown
---
kedge:
  group: payments
  anchors:
    - repo: "git@github.com:your-org/services.git"
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: ""
---

# Token Validation

This document describes how the `validateToken` method works...
```

kedge reads the `kedge:` block in the YAML frontmatter. Everything below the closing `---` is regular markdown that your team writes and maintains.

## Step by step

### 1. Pick a code location to track

Identify the code you want documentation to stay in sync with. You need:

- **repo**: the git URL of the code repository
- **path**: the file path within that repository
- **symbol** (optional): a specific declaration to track

Symbol syntax varies by language:

| Language | Example |
|----------|---------|
| Java | `AuthService#validateToken` |
| Go | `HandleRequest` |
| TypeScript | `UserService#create` or `parseConfig` |
| Python | `AuthService#validate_token` or `parse_config` |
| Rust | `AuthService#validate_token` or `parse_config` |

If you omit `symbol`, kedge fingerprints the entire file.

### 2. Create the steering file

Create a `.md` file in your docs repository. The location depends on your `[[repos.docs]]` config. If `path = "steering/"`, place the file under that directory.

```markdown
---
kedge:
  group: payments
  anchors:
    - repo: "git@github.com:your-org/services.git"
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: ""
---

# Token Validation

The `validateToken` method in `AuthService` verifies JWT tokens
by checking the signature, expiry, and issuer claims...
```

Leave `provenance` empty for now. `kedge link` fills it in.

### 3. Stamp provenance

From your **code** repository (the one `kedge.toml` lives in):

```bash
kedge link
```

kedge reads each steering file, computes an AST fingerprint for every anchor, and writes the result back:

```yaml
provenance: "sig:a1b2c3d4e5f67890"
```

The `sig:` value is a content-addressed hash of the code's AST structure. The hash ignores whitespace and comments, so formatting changes won't trigger false drift.

### 4. Verify

Run a drift check:

```bash
kedge check
```

Since you stamped provenance, the output should show zero drift (exit code `0`).

### 5. Make a code change and re-check

Edit the anchored code. Change a method signature, add a parameter, or rename a variable. Then:

```bash
kedge check
```

Now kedge reports drift (exit code `1`) with a JSON report showing which anchors changed.

## Multiple anchors

A single steering file can track multiple code locations:

```yaml
kedge:
  group: auth
  anchors:
    - repo: "git@github.com:your-org/services.git"
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: "sig:a1b2c3d4e5f67890"
    - repo: "git@github.com:your-org/services.git"
      path: src/auth/AuthService.java
      symbol: AuthService#refreshToken
      provenance: "sig:f0e1d2c3b4a59687"
```

kedge tracks each anchor independently. If `validateToken` changes but `refreshToken` doesn't, only the first anchor shows drift.

## Groups

The `group` field scopes operations. When running `kedge install --group payments`, only steering files with `group: payments` are installed. Use groups to organize docs by team or business unit.

## Next steps

- [Steering File Format](../reference/steering-file-format.md): full specification
- [Provenance](../concepts/provenance.md): how content-addressed fingerprints work
- [How Drift Detection Works](../concepts/how-drift-detection-works.md): the detection algorithm
