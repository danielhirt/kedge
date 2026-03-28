---
inclusion: always
kedge:
  anchors:
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: api/src/main/java/com/nexus/platform/api/PaymentController.java
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: api/src/main/java/com/nexus/platform/api/AccountController.java
      provenance: "sig:0000000000000000"
---

# API Conventions

These conventions apply to all REST endpoints across the platform.

## Versioning

- URL path versioning: `/api/v1/payments`
- Breaking changes increment the major version
- Deprecated endpoints return `Sunset` header with removal date

## Request Standards

- Content-Type: `application/json` for all request bodies
- `X-Request-Id` header propagated through the entire call chain for tracing
- `X-Idempotency-Key` required on all mutating operations (POST, PUT, DELETE)

## Response Standards

- All responses wrapped in standard envelope:
  ```json
  { "data": {}, "errors": [], "meta": { "requestId": "..." } }
  ```
- Timestamps in ISO 8601 UTC format
- Monetary amounts as string-encoded decimals (never floating point)
- Null fields omitted from response (not sent as `null`)

## Pagination

- Query params: `page` (1-based), `size` (default 20, max 100)
- Response includes `meta.totalCount`, `meta.totalPages`
- `Link` header with `rel=next` and `rel=prev` for HATEOAS clients
