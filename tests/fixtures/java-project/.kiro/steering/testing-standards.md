---
inclusion: fileMatch
fileMatchPattern:
  - "**/src/test/java/**/*.java"
  - "**/*Test.java"
  - "**/*IT.java"
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
