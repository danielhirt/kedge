# Severity Classification

Triage is the second layer of kedge's pipeline. It uses a lightweight AI call to classify each drifted anchor into one of three severity levels, which then drive remediation decisions.

## Severity levels

### `no_update`

The code change is purely cosmetic and doesn't affect the documentation's accuracy.

**Examples:**
- Whitespace or formatting changes that got past AST fingerprinting (e.g., in content-hash fallback files)
- Internal variable renaming that doesn't affect the public API
- Refactoring that preserves behavior

**Remediation:** No agent invocation. kedge automatically advances the provenance to the current fingerprint.

### `minor`

The code change is small and the documentation needs a minor update.

**Examples:**
- A new optional parameter was added to a method
- A variable was renamed in a way that's mentioned in the docs
- A default value changed
- A new enum variant was added

**Remediation:** The agent is invoked to update the docs. Whether auto-merge is enabled depends on the `auto_merge_severities` config.

### `major`

The code change is significant and the documentation needs a substantial rewrite.

**Examples:**
- A required parameter was added or removed
- A method's return type changed
- A function was removed or renamed
- The semantics of an operation changed
- A breaking API change

**Remediation:** The agent is invoked to update the docs. Typically requires human review before merging.

## How classification works

For each drifted doc, kedge:

1. Builds a prompt containing the documentation content and the code diffs for each drifted anchor
2. Sends the prompt to the configured triage provider (Anthropic, OpenAI, or custom command)
3. Parses the response as a JSON array of `{ path, symbol, severity }` classifications
4. Maps classifications back to anchors

The AI sees both the current documentation and the code changes, so it can judge whether the change actually affects what the docs describe.

### Triage prompt structure

The prompt includes:

- The full documentation content (the markdown body of the steering file)
- For each drifted anchor: the file path, symbol, diff summary, and full diff
- Instructions to classify as `no_update`, `minor`, or `major` with clear criteria
- Expected JSON response format

### Doc-level severity

Each doc gets an overall severity equal to the **maximum** of its anchor severities. If a doc has one `minor` and one `major` anchor, the doc severity is `major`.

## Cost control

Triage only runs on **drifted** docs. Clean docs (where no anchors have drifted) incur zero AI cost. This is by design -- detection is deterministic and free, and AI is only invoked when there's actual work to classify.

## How severity drives remediation

| Severity | Agent invoked? | Provenance advanced? | Typical auto-merge? |
|----------|---------------|---------------------|-------------------|
| `no_update` | No | Yes (automatic) | N/A |
| `minor` | Yes | After agent completes | Often yes |
| `major` | Yes | After agent completes | Usually no (human review) |

### Auto-merge configuration

The `auto_merge_severities` config controls which severity levels set `auto_merge: true` in the agent payload:

```toml
[remediation]
auto_merge_severities = ["no_update", "minor"]
```

The agent decides what to do with the `auto_merge` flag -- kedge just passes it through. In batch mode, `auto_merge` is `true` only if every target qualifies individually.

## Triage providers

kedge supports three triage backends. See [Configuration](../reference/configuration.md) for setup details.

- **`anthropic`** -- direct Anthropic API (recommended: `claude-haiku-4-5-20251001` for cost efficiency)
- **`openai`** -- any OpenAI-compatible endpoint (Azure OpenAI, vLLM, local models)
- **`command`** -- pipe the prompt to an external command via stdin
