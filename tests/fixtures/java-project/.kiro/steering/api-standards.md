---
inclusion: fileMatch
fileMatchPattern:
  - "api/src/main/java/**/*.java"
  - "api/src/test/java/**/*.java"
steer:
  group: payments-platform
  anchors:
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: api/src/main/java/com/nexus/platform/api/PaymentController.java
      symbol: PaymentController#createPayment
      provenance: e4a7f1b2
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: api/src/main/java/com/nexus/platform/api/middleware/RateLimiter.java
      symbol: RateLimiter#checkLimit
      provenance: e4a7f1b2
---

# API Standards

## REST Conventions

- All endpoints under `/api/v{major}` prefix
- Use plural nouns for resources: `/payments`, `/accounts`, `/transactions`
- Return `201 Created` with `Location` header for POST that creates a resource
- Idempotency key required in `X-Idempotency-Key` header for all POST/PUT

## Request / Response

- Request bodies use `PaymentRequest`, `AccountRequest` DTOs — never expose domain models
- Responses wrap in `ApiResponse<T>` with `data`, `errors`, and `meta` fields
- Pagination via `?page=1&size=20` with `Link` headers for next/prev
- All monetary amounts as `BigDecimal` serialized as strings to avoid floating-point loss

## Error Handling

- `4xx` errors return `ErrorResponse` with `code`, `message`, and `details` array
- `5xx` errors omit internal details; log correlation ID for support tracing
- Rate limit exceeded returns `429` with `Retry-After` header in seconds

## Authentication

- Bearer token in `Authorization` header validated by `AuthMiddleware`
- Token introspection cached in Redis for 5 minutes
- Merchant-scoped API keys supported via `X-Api-Key` header for server-to-server calls
