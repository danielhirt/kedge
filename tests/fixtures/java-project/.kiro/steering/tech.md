---
inclusion: always
steer:
  anchors:
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: pom.xml
      provenance: e4a7f1b2
---

# Technology Stack

## Runtime

- Java 21 (LTS) with virtual threads enabled for I/O-bound operations
- Spring Boot 3.3.x with Spring WebFlux for the API layer
- Maven multi-module build with 4 submodules: core, api, workflow-engine, schema-definitions

## Data

- PostgreSQL 16 for transactional data (payment records, accounts)
- Redis 7 for session cache, rate limiting counters, and idempotency keys
- Apache Kafka for event streaming between services

## Custom XML DSL

The `workflow-engine` and `schema-definitions` modules use a proprietary XML dialect
called NWDL (Nexus Workflow Definition Language) for:
- Defining multi-stage approval workflows (`<workflow>`, `<stage>`, `<rule>`)
- Mapping internal models to ISO 20022 and SWIFT formats (`<mapping>`, `<field>`)
- Declarative data transforms for settlement files (`<transform>`, `<rules>`)

These XML files are parsed at startup by `DslParser` and compiled into an in-memory
rule graph by `RuleEvaluator`.

## Testing

- JUnit 5 with Testcontainers for integration tests
- WireMock for external API stubs (card networks, FX providers)
- ArchUnit for enforcing module boundary rules
