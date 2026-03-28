---
inclusion: fileMatch
fileMatchPattern:
  - "**/src/test/java/**/*.java"
  - "**/*Test.java"
  - "**/*IT.java"
steer:
  anchors:
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: core/src/main/java/com/nexus/platform/service/PaymentService.java
      symbol: PaymentService#processPayment
      provenance: e4a7f1b2
---

# Testing Standards

## Test Categories

- **Unit tests** (`*Test.java`): No Spring context, no I/O. Use Mockito for collaborators.
  Target: every public method on service and utility classes.
- **Integration tests** (`*IT.java`): Full Spring context with Testcontainers (Postgres, Redis, Kafka).
  Target: end-to-end flows through controller -> service -> repository.
- **Contract tests**: WireMock stubs for card network APIs. Verify request/response schemas
  match the published OpenAPI spec.

## Conventions

- Test class name mirrors source class: `PaymentService` -> `PaymentServiceTest`
- Use `@Nested` for grouping related scenarios within a test class
- Arrange-Act-Assert pattern; one assertion concept per test method
- No `Thread.sleep` in tests; use Awaitility for async assertions

## Workflow XML Tests

- Validate all NWDL XML files against the XSD schemas at build time
- `WorkflowEngineTest` must cover: stage ordering, rule evaluation, timeout handling
- Any new `<rule>` element requires a corresponding test case

## Coverage

- Minimum 80% line coverage on core/ and api/ modules
- workflow-engine/ and schema-definitions/ exempt from line coverage (XML-heavy)
- Mutation testing via PIT on critical paths (payment processing, fraud scoring)
