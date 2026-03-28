# Steer — Documentation Drift Detection & Remediation

**Date:** 2026-03-28
**Status:** Draft
**Author:** Daniel (brainstormed with Claude)

## Problem

Documentation drift in large codebases with legacy code and monolithic paradigms. AI agents are changing code at increasing rates, making it harder to keep architectural documentation and behavioral specs in sync with the code they describe. The team uses Kiro CLI with layered steering files across submodules, and needs an automated, agent-first strategy for keeping docs current.

## Constraints

- **Post-merge only** — no gates or merge blocking
- **Agent-first** — an AI agent handles doc updates, not humans
- **Agnostic CLI** — any CI/CD system can invoke it; not coupled to Kiro, GitLab, or any specific agent
- **Doc repo is source of truth** — steering files are served from a separate documentation repo, not stored in the code repo
- **No submodules or MCP** — doc repo cannot be exposed as a submodule; avoid MCP server complexity
- **Human review by default** — agent opens MRs for review, with a path to tiered autonomy (minor auto-merge)
- **Multi-business-unit** — large department, steering must be scoped per group
- **Headless CI agents** — triage and code review agents run headless in pipeline steps, checking out the codebase

## Languages

- Java (primary backend)
- Go (separate service)
- TypeScript/React (frontend)
- Proprietary XML-based UI language
- Tree-sitter support for Java, Go, TypeScript; XML structural parsing; content-hash fallback for unsupported

## Approach: Hybrid — Deterministic Detection + Semantic Triage + Agent Remediation

Three processing layers:

1. **Detection** (deterministic, no AI cost) — tree-sitter AST fingerprinting to detect when anchored code has changed
2. **Triage** (lightweight AI) — classifies drift severity before committing to full agent invocation
3. **Remediation** (agent invocation) — invokes a configured agent to update docs and open MRs

---

## Architecture

### CLI Commands

| Command | Purpose |
|---|---|
| `steer init` | Initialize a repo — creates config, scans for existing docs |
| `steer link` | Stamp/update provenance anchors in doc frontmatter |
| `steer check` | Detect drift, output report (exit 0 = clean, exit 1 = drift found) |
| `steer triage` | Run semantic triage on detected drift (classify severity) |
| `steer update` | Full pipeline: check -> triage -> invoke agent -> open MR |
| `steer status` | Show all anchors and their current drift state |
| `steer sync` | Advance provenance markers without doc content changes (used for `no_update` anchors). Commits directly to docs repo — no MR needed since no prose changed. When mixed with `minor`/`major` drift, provenance advances are included in the agent's MR instead. |
| `steer install` | Pull steering files from doc repo to local/workspace `.kiro/steering/` |

### Configuration (`steer.toml`)

```toml
[detection]
languages = ["java", "go", "typescript", "xml"]
fallback = "content-hash"

[triage]
provider = "anthropic"
model = "claude-haiku-4-5-20251001"
severity_levels = ["no_update", "minor", "major"]

[remediation]
agent_command = "kiro --agent drift-updater"
auto_merge_severities = []

[repos]
[[repos.docs]]
url = "git@gitlab.example.com:team/docs.git"
path = "steering/"
ref = "main"

# Agent platform targets — steer install writes to these locations
# Multiple targets supported (e.g., team uses both Kiro and Claude Code)
[[agents]]
name = "kiro"
global_steering = "~/.kiro/steering/"           # --link target
workspace_steering = ".kiro/steering/"          # --workspace target
agents_file = "AGENTS.md"                       # always-loaded instruction file
skill_dir = ".kiro/skills/"                     # where to install skill files

[[agents]]
name = "claude"
global_steering = "~/.claude/steering/"
workspace_steering = ".claude/steering/"
agents_file = "CLAUDE.md"
skill_dir = ".claude/skills/"

[[agents]]
name = "codex"
global_steering = "~/.codex/steering/"
workspace_steering = ".codex/steering/"
agents_file = "AGENTS.md"
skill_dir = ".codex/skills/"

# Custom agent platform — any tool that reads markdown from a directory
# [[agents]]
# name = "custom"
# global_steering = "~/my-agent/context/"
# workspace_steering = ".my-agent/context/"
# agents_file = "AGENTS.md"
# skill_dir = ".my-agent/skills/"
```

---

## Layer 1: Detection

Purely deterministic. Answers: "has the code this doc references changed since the doc was last reviewed?"

### Frontmatter Anchor Format

Doc files in the docs repo use dual-purpose frontmatter — Kiro reads the inclusion fields, `steer` reads the anchor fields:

```yaml
---
inclusion: fileMatch
fileMatchPattern: ["src/auth/**/*.java", "src/api/routes/auth.*"]
steer:
  group: payments-platform
  anchors:
    - repo: git@gitlab.example.com:team/backend.git
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: a1b2c3d4
    - repo: git@gitlab.example.com:team/backend.git
      path: src/auth/AuthService.java
      symbol: AuthService#refreshSession
      provenance: a1b2c3d4
---
```

- **path**: file relative to the referenced repo root
- **symbol** (optional): specific declaration to track (class, method, function). When omitted, tracks the entire file via content hash.
- **repo**: the git remote of the code repo this anchor references (required for cross-repo)
- **provenance**: short SHA of the commit when the doc was last verified against this code
- **group**: business unit scope for `steer install --group` filtering

### Detection Algorithm

```
for each doc_file with steer frontmatter:
  for each anchor in doc_file.anchors:
    current_fingerprint = fingerprint(anchor.path, anchor.symbol)
    provenance_fingerprint = fingerprint_at(anchor.provenance, anchor.path, anchor.symbol)

    if current_fingerprint != provenance_fingerprint:
      drift_report.add(doc_file, anchor, diff_since(anchor.provenance))
```

### Fingerprinting Strategy

| Language | Parser | Granularity |
|---|---|---|
| Java | tree-sitter-java | Class, method, field, annotation |
| Go | tree-sitter-go | Function, type, method |
| TypeScript/JSX | tree-sitter-typescript | Function, class, component, type |
| XML (proprietary UI) | tree-sitter-xml | Element-level structure |
| Unsupported | — | Content hash of entire file |

Fingerprint = hash of normalized AST node kinds + token text, stripping whitespace and position data. Reformatting, import reordering, and comment additions do not trigger false positives.

### Drift Report Output (JSON)

```json
{
  "repo": "backend-monolith",
  "ref": "main",
  "commit": "f8e9d0c1",
  "drifted": [
    {
      "doc": "steering/payments-platform/auth-service.md",
      "doc_repo": "git@gitlab.example.com:team/docs.git",
      "anchors": [
        {
          "path": "src/auth/AuthService.java",
          "symbol": "AuthService#validateToken",
          "provenance": "a1b2c3d4",
          "current_commit": "f8e9d0c1",
          "diff_summary": "Method signature changed: added List<String> scopes parameter"
        }
      ]
    }
  ],
  "clean": [
    { "doc": "steering/payments-platform/payments.md", "anchor_count": 4 }
  ]
}
```

---

## Layer 2: Triage

Lightweight AI classification. Runs only on anchors flagged by detection — no AI cost for clean anchors.

### Severity Levels

| Severity | Meaning | Examples |
|---|---|---|
| `no_update` | Code changed but doc is still accurate | Refactored internals without changing behavior, added logging, performance optimization |
| `minor` | Doc needs mechanical update, no judgment required | Renamed method/class, moved file, added parameter with obvious purpose |
| `major` | Doc needs substantive rewrite, behavior or architecture changed | New validation logic, changed data flow, removed/replaced functionality |

### Triage Prompt

Sent per drifted doc (not per anchor) so related changes are evaluated together:

```
You are classifying documentation drift severity.

## Current documentation
{doc_content}

## Code changes since last doc review
{anchors_with_diffs}

For each anchor, classify as:
- no_update: code changed but the documentation is still accurate
- minor: documentation needs a mechanical/obvious update (rename, move, signature change)
- major: documentation needs substantive revision (behavior, architecture, or intent changed)

Respond with JSON only.
```

### Triaged Report Output (JSON)

This is the agent contract — the interface boundary consumed by any downstream agent.

```json
{
  "repo": "backend-monolith",
  "ref": "main",
  "commit": "f8e9d0c1",
  "drifted": [
    {
      "doc": "steering/payments-platform/auth-service.md",
      "doc_repo": "git@gitlab.example.com:team/docs.git",
      "severity": "major",
      "summary": "validateToken now accepts a scopes parameter and performs scope-based authorization. This changes the authentication model from binary (valid/invalid) to permission-scoped.",
      "anchors": [
        {
          "path": "src/auth/AuthService.java",
          "symbol": "AuthService#validateToken",
          "severity": "major",
          "provenance": "a1b2c3d4",
          "diff": "...unified diff content..."
        },
        {
          "path": "src/auth/AuthService.java",
          "symbol": "AuthService#refreshSession",
          "severity": "no_update",
          "provenance": "a1b2c3d4",
          "diff": "...unified diff content..."
        }
      ]
    }
  ]
}
```

- Per-doc severity = max of its anchor severities
- `no_update` anchors still appear so the agent can advance provenance
- The `summary` field gives the remediation agent a semantic head start

---

## Layer 3: Remediation

Orchestration that takes the triaged report and invokes the downstream agent.

### Flow

1. Filter out `no_update` anchors from remediation scope (queue for provenance bump via `steer sync`)
2. Group remaining drift by target doc
3. For each group: assemble agent context payload, invoke configured `agent_command`
4. Agent pushes branch, opens MR in docs repo
5. Output remediation summary (MR URLs, provenance to advance)

### Agent Invocation

The CLI pipes the triaged report via stdin to the configured command:

```bash
cat triaged_report.json | kiro --agent drift-updater
cat triaged_report.json | claude --agent-skill steer-updater
cat triaged_report.json | python update_docs.py
```

### Agent Context Payload

```json
{
  "action": "update_docs",
  "severity": "major",
  "auto_merge": false,
  "target": {
    "repo": "git@gitlab.example.com:team/docs.git",
    "branch_prefix": "steer/auto-update",
    "path": "steering/payments-platform/auth-service.md"
  },
  "drifted_anchors": [
    {
      "path": "src/auth/AuthService.java",
      "symbol": "AuthService#validateToken",
      "severity": "major",
      "summary": "validateToken now accepts a scopes parameter...",
      "diff": "...unified diff..."
    }
  ],
  "instructions": "Update the target document to reflect the code changes. Stamp all modified anchors with provenance commit f8e9d0c1."
}
```

### Auto-Merge Path (Tiered Autonomy)

```toml
[remediation]
auto_merge_severities = ["minor"]  # enable when ready, currently []
```

When a severity is in this list, the payload includes `"auto_merge": true`, signaling the agent to merge after CI passes. The triage classification boundary is the control surface for tuning this.

---

## Agent Interface

`steer` exposes itself to agents through three surfaces.

### `steer install`

Pulls steering files from the doc repo and makes them available to the configured agent platform(s).

```bash
# Install for all configured agent platforms
steer install --group payments-platform --link

# Install for a specific platform only
steer install --group payments-platform --link --agent kiro
steer install --group payments-platform --link --agent claude

# CI/ephemeral — copies to workspace steering directories
steer install --group payments-platform --workspace

# Auto-detect: --workspace if CI env vars present (CI, GITLAB_CI, etc.),
# otherwise --link
steer install --group payments-platform
```

When no `--agent` flag is provided, installs to all platforms defined in `[[agents]]` config. In `--workspace` mode, adds each platform's workspace steering directory to `.git/info/exclude` so injected files don't appear as untracked.

The install command reads each agent's configured paths from `steer.toml`:
- `--link` mode: symlinks steering files to each platform's `global_steering` path
- `--workspace` mode: copies steering files to each platform's `workspace_steering` path
- Copies `_steer/AGENTS.md` to each platform's `agents_file` location
- Copies `_steer/skill.md` to each platform's `skill_dir` location

### Agent Hooks

Each platform has its own hook mechanism. `steer install --check` is the universal command — the hook config is platform-specific:

**Kiro (AgentSpawn hook):**
```json
{
  "hooks": [
    {
      "type": "AgentSpawn",
      "command": "steer install --group payments-platform --check --agent kiro",
      "timeout_ms": 15000,
      "cache_ttl_seconds": 3600
    }
  ]
}
```

**Claude Code (hook in settings.json):**
```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "steer install --group payments-platform --check --agent claude"
          }
        ]
      }
    ]
  }
}
```

**Codex / other agents:** Configure via each platform's native hook or pre-session script.

`--check` compares local steering against docs repo HEAD, only re-syncs if stale. Cached for 1 hour.

### Shipped Agent Configs

The docs repo includes meta-files that teach agents about `steer`:

```
docs-repo/
└── steering/
    ├── _steer/
    │   ├── skill.md            # manual skill for proactive drift checks
    │   └── AGENTS.md           # always-loaded baseline instructions
    ├── payments-platform/
    │   ├── auth-service.md
    │   └── payments.md
    ├── customer-experience/
    │   └── onboarding.md
    └── shared/                 # always installed regardless of group
        ├── api-conventions.md
        └── security-policies.md
```

**`_steer/AGENTS.md`:**

```markdown
# Steer — Documentation Drift Management

When you modify code, check if your changes affect any steering files
currently in your context. Each steering file has frontmatter with
`steer.anchors` listing the code it documents.

If you change code covered by an anchor:
1. Note the drift in your commit message or MR description
2. Do NOT update the steering file directly — drift remediation
   is handled by the automated pipeline
3. If you believe a steering file is inaccurate, flag it rather
   than silently working around it

When reviewing code: check the steer anchors in relevant steering
files to understand architectural intent before suggesting changes.
```

### Agent Type Matrix

| Agent Type | Install Method | Steering Discovery | Skill Usage |
|---|---|---|---|
| Developer (any platform) | AgentSpawn hook | `~/.<agent>/steering/` via symlinks, platform-native file matching | Manual skill for proactive checks |
| Triage/Review (CI) | Pipeline step before agent invocation | `--workspace` copies to `.<agent>/steering/` per configured platform | AGENTS.md/CLAUDE.md context for review guidance |
| Remediation (CI) | Already has docs repo access | Reads drift report JSON directly | Follows `instructions` field in payload |

The `[[agents]]` config in `steer.toml` makes this extensible — adding a new agent platform is a config change, not a code change.

---

## Doc Repo Structure

```
docs-repo/
└── steering/
    ├── _steer/                        # meta, always installed
    │   ├── skill.md
    │   └── AGENTS.md
    ├── payments-platform/             # --group payments-platform
    │   ├── auth-service.md
    │   └── payments.md
    ├── customer-experience/           # --group customer-experience
    │   ├── onboarding.md
    │   └── support-portal.md
    └── shared/                        # always installed (cross-cutting)
        ├── api-conventions.md
        └── security-policies.md
```

`shared/` always installs regardless of `--group` flag. Handles cross-cutting concerns all business units need.

---

## End-to-End Workflows

### Workflow 1: Post-Merge Drift Detection & Remediation

```
Developer merges MR to main (code repo)
    |
    v
GitLab pipeline triggers
    |
    v
steer check
    |- Reads steer.toml for config
    |- Clones/caches docs repo
    |- Scans docs for steer: anchors referencing this repo
    |- Fingerprints anchored code at HEAD vs provenance SHA
    '- Outputs drift report JSON
    |
    |- Exit 0 (no drift) -> done
    |
    v  Exit 1 (drift found)
steer triage
    |- Sends diffs + doc content to triage model
    |- Classifies: no_update / minor / major
    |- Generates semantic summary per drifted doc
    '- Outputs triaged report JSON
    |
    v
steer update
    |- Filters out no_update (queues provenance bump)
    |- Groups remaining drift by target doc
    |- Invokes agent_command with triaged report via stdin
    |     |
    |     v
    |   Agent (headless)
    |     |- Reads triaged report
    |     |- Checks out docs repo, creates branch
    |     |- Updates affected doc(s)
    |     |- Stamps anchors with new provenance SHA
    |     |- Opens MR in docs repo
    |     '- Returns MR URL
    |
    |- Runs steer sync for no_update anchors
    '- Outputs remediation summary
```

**GitLab pipeline:**

```yaml
drift_check:
  stage: post-merge
  rules:
    - if: $CI_COMMIT_BRANCH == "main"
      when: always
  script:
    - steer update --report artifacts/drift-report.json
  artifacts:
    paths:
      - artifacts/drift-report.json
    when: always
```

### Workflow 2: Code Review with Steering Context

```
MR opened in code repo
    |
    v
GitLab pipeline triggers review job
    |
    v
git checkout $CI_MERGE_REQUEST_SHA
    |
    v
steer install --group payments-platform --workspace
    |- Clones docs repo (cached)
    |- Copies group's steering files to .kiro/steering/
    |- Copies _steer/AGENTS.md to .kiro/steering/
    '- Adds .kiro/steering/ to .git/info/exclude
    |
    v
kiro --headless --agent code-reviewer
    |- Agent reads changed files in the MR
    |- fileMatchPattern triggers: relevant steering loads
    |- AGENTS.md tells reviewer to check anchors
    |- Reviewer flags if changes contradict documented architecture
    '- Posts review comments on MR (informational, not blocking)
```

### Workflow 3: Developer Session with Live Steering

```
Developer starts Kiro session in code repo
    |
    v
AgentSpawn hook fires: steer install --group payments-platform --check
    |- Compares local symlinks against docs repo HEAD
    |- If stale: pulls latest, updates symlinks in ~/.kiro/steering/
    |- If current: no-op (cached 1 hour)
    '- Exits 0
    |
    v
Developer asks agent to modify AuthService.java
    |
    v
Kiro loads auth-service.md via fileMatchPattern
    |- Agent sees architectural intent and behavioral specs
    |- Agent sees steer anchors (knows this code is documented)
    |- AGENTS.md reminds agent to flag drift, not silently work around it
    '- Agent makes changes with full context
    |
    v
Developer commits and merges -> triggers Workflow 1
```

### Workflow Connection

Workflow 3 (dev session) -> merge -> Workflow 1 (drift detection) -> doc MR merges -> Workflow 3 (next session picks up updated steering via AgentSpawn hook). Workflow 2 (code review) runs in parallel, catching drift signals early without blocking.

---

## Technology Decisions

- **CLI implementation language**: TBD (Rust or Go recommended for portability and tree-sitter bindings)
- **Tree-sitter bindings**: Required for Java, Go, TypeScript, XML
- **Triage model**: Haiku-class (fast, cheap classification) via configurable provider
- **Agent invocation**: stdin pipe of JSON — any process that reads JSON from stdin works
- **Distribution**: Binary releases, installable via package managers

## Open Questions

- Should `steer init` scan an existing codebase and suggest initial anchors, or is this a manual/agent-assisted process?
- How should anchor conflicts be handled when multiple docs reference the same code symbol?
- Should the triage model be configurable per group (some teams may want different sensitivity)?
- What is the caching strategy for cloned repos in CI (GitLab cache, persistent volumes, etc.)?
