---
inclusion: fileMatch
fileMatchPattern:
  - "workflow-engine/**/*.java"
  - "workflow-engine/**/*.xml"
steer:
  group: payments-platform
  anchors:
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: workflow-engine/src/main/java/com/nexus/platform/dsl/WorkflowEngine.java
      symbol: WorkflowEngine#execute
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: workflow-engine/src/main/java/com/nexus/platform/dsl/RuleEvaluator.java
      symbol: RuleEvaluator#evaluate
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: workflow-engine/src/main/resources/workflows/payment-approval.xml
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: workflow-engine/src/main/resources/workflows/fraud-detection.xml
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: workflow-engine/src/main/resources/workflows/account-onboarding.xml
      provenance: "sig:0000000000000000"
---

# Workflow Engine

The workflow engine executes NWDL (Nexus Workflow Definition Language) workflows
against payment transactions. Workflows are loaded from XML files at startup
and compiled into an in-memory directed acyclic graph.

## WorkflowEngine#execute

Entry point for running a transaction through a named workflow.

**Parameters:**
- `String workflowName` — matches the `name` attribute of a `<workflow>` element
- `TransactionContext context` — the transaction data and metadata to evaluate

**Flow:**
1. Look up the compiled workflow by name
2. Execute stages in dependency order (respecting `requires` attributes)
3. For each stage, evaluate all rules via `RuleEvaluator`
4. If a `<gate>` is encountered, pause execution and create a review task
5. Return `WorkflowResult` with the outcome and any flagged rules

## RuleEvaluator#evaluate

Evaluates a single `<rule>` against a transaction context.

**Parameters:**
- `Rule rule` — the compiled rule (from `<rule>` XML element)
- `TransactionContext context` — current transaction state

**Returns:** `RuleResult` — `PASS`, `FLAG`, `REJECT`, or `ROUTE`

## Workflow XML Files

| File | Purpose |
|------|---------|
| `payment-approval.xml` | Multi-stage approval for high-value payments |
| `fraud-detection.xml` | Real-time fraud scoring with velocity checks |
| `account-onboarding.xml` | KYC verification workflow for new accounts |
