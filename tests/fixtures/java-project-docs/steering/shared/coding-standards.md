---
inclusion: always
---

# Coding Standards

## Java Style

- Follow Google Java Style Guide with project-specific overrides
- Max line length: 120 characters
- Indentation: 4 spaces (no tabs)
- Imports: no wildcards, organized by package (static imports last)

## Naming

- Classes: PascalCase (`PaymentService`, `TransactionProcessor`)
- Methods: camelCase (`processPayment`, `validateToken`)
- Constants: UPPER_SNAKE_CASE (`MAX_RETRY_COUNT`, `DEFAULT_TIMEOUT_MS`)
- Packages: lowercase, no underscores (`com.nexus.platform.service`)

## Error Handling

- Use domain-specific exceptions extending `NexusException` base class
- Never catch `Exception` or `Throwable` broadly — catch specific types
- `@Transactional` methods must not swallow exceptions that should trigger rollback
- Log exceptions at the point of handling, not at the point of throwing

## Dependencies

- No circular module dependencies (enforced by ArchUnit)
- `core` depends on nothing; `api` and `workflow-engine` depend on `core`
- `schema-definitions` depends only on `core` for model types
- Third-party library additions require team review (update `pom.xml` via PR)

## XML Conventions

- NWDL files must be valid against the corresponding XSD schema
- Element names in kebab-case: `<payment-approval>`, `<manual-review>`
- Attribute names in camelCase: `merchantId`, `maxRetries`
- Version attribute required on all `<workflow>` root elements
