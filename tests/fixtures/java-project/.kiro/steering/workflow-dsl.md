---
inclusion: fileMatch
fileMatchPattern:
  - "workflow-engine/src/main/resources/workflows/**/*.xml"
  - "schema-definitions/src/main/resources/**/*.xml"
  - "workflow-engine/src/main/java/**/*.java"
steer:
  group: payments-platform
  anchors:
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: workflow-engine/src/main/java/com/nexus/platform/dsl/WorkflowEngine.java
      symbol: WorkflowEngine#execute
      provenance: e4a7f1b2
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: workflow-engine/src/main/java/com/nexus/platform/dsl/DslParser.java
      symbol: DslParser#parseWorkflow
      provenance: e4a7f1b2
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: workflow-engine/src/main/resources/workflows/payment-approval.xml
      provenance: e4a7f1b2
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: workflow-engine/src/main/resources/workflows/fraud-detection.xml
      provenance: e4a7f1b2
---

# NWDL - Nexus Workflow Definition Language

## Element Reference

### `<workflow>`
Root element. Attributes: `name` (unique ID), `version` (semver).

### `<trigger>`
Defines the event that starts the workflow. Attribute: `event` (dot-notation event name).

### `<stages>`
Container for ordered processing stages.

### `<stage>`
A named processing step. Attributes: `name`, `requires` (optional predecessor stage).

### `<rule>`
A conditional check within a stage. Attributes: `id` (unique within workflow).

### `<condition>`
Predicate on a transaction field. Attributes: `field`, `operator` (`eq`, `gt`, `lt`, `in`, `regex`), `value`.

### `<action>`
What to do when a rule matches. Attributes: `type` (`pass`, `flag`, `reject`, `route`), `reason` (optional).

### `<gate>`
Manual intervention point. Attributes: `type` (`manual-review`, `dual-approval`), `assignee`.

### `<timeout>`
Escalation timer on a gate. Attributes: `duration` (ISO 8601 or shorthand like `24h`), `action` (`escalate`, `auto-approve`, `reject`).

## Validation Rules

- Every `<workflow>` must have exactly one `<trigger>`
- Stage names must be unique within a workflow
- `requires` must reference an existing stage name
- Rule IDs must be unique within a workflow
- `<condition>` operators must be from the allowed set
