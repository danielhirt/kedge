---
inclusion: always
---

# Project Structure

## Module Layout

```
nexus-payment-platform/
  core/           -> Domain models, business services, crypto utilities
  api/            -> REST controllers, DTOs, middleware (auth, rate-limiting)
  workflow-engine/ -> NWDL parser, rule evaluator, workflow execution engine
  schema-definitions/ -> ISO 20022 mappings, SWIFT transforms, field definitions
```

## Package Conventions

- Base package: `com.nexus.platform`
- Module-specific: `com.nexus.platform.{module}` (e.g., `com.nexus.platform.api`)
- DTOs live under `*.dto` sub-packages, never in the same package as services
- No cross-module imports except through `core` — api and workflow-engine depend
  on core, but never on each other

## File Naming

- Java: PascalCase class names matching file names
- XML workflows: kebab-case (e.g., `payment-approval.xml`)
- XML schemas: kebab-case with suffix indicating type (`-mapping.xml`, `-transform.xml`)
- Tests mirror source structure under `src/test/java/`

## Key Directories

- `workflow-engine/src/main/resources/workflows/` — NWDL workflow definitions
- `schema-definitions/src/main/resources/mappings/` — Field mapping XMLs
- `schema-definitions/src/main/resources/transforms/` — Data transformation XMLs
- `.kiro/steering/` — Agent steering files (this directory)
- `docs/steering/` — Documentation repository steering files with drift anchors
