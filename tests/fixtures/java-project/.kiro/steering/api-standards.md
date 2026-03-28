---
inclusion: fileMatch
fileMatchPattern:
  - "api/src/main/java/**/*.java"
  - "api/src/test/java/**/*.java"
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
