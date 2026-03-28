---
inclusion: fileMatch
fileMatchPattern:
  - "api/src/main/java/**/middleware/**/*.java"
  - "api/src/main/java/**/AuthMiddleware.java"
steer:
  group: platform-security
  anchors:
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: api/src/main/java/com/nexus/platform/api/middleware/AuthMiddleware.java
      symbol: AuthMiddleware#validateToken
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: api/src/main/java/com/nexus/platform/api/middleware/AuthMiddleware.java
      symbol: AuthMiddleware#extractClaims
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: api/src/main/java/com/nexus/platform/api/middleware/RateLimiter.java
      symbol: RateLimiter#checkLimit
      provenance: "sig:0000000000000000"
---

# Authentication Middleware

The `AuthMiddleware` is a Spring WebFilter that intercepts all incoming requests
to validate authentication tokens before they reach controller methods.

## validateToken

Validates a JWT bearer token from the `Authorization` header.

**Flow:**
1. Extract the token from `Authorization: Bearer <token>`
2. Check the Redis token cache for a previously validated result
3. If not cached, verify the JWT signature using the RS256 public key
4. Validate claims: `exp`, `iss`, `aud`, and custom `merchant_id` claim
5. Cache the validation result in Redis with 5-minute TTL
6. Populate the `SecurityContext` with the authenticated principal

**Error cases:**
- Missing/malformed header -> `401 Unauthorized`
- Expired token -> `401` with `token_expired` error code
- Invalid signature -> `401` with `invalid_token` error code

## extractClaims

Parses JWT claims without full validation. Used internally after
`validateToken` has already verified the signature.

## Rate Limiter

The `RateLimiter` enforces per-merchant request limits using a sliding
window counter in Redis.

- Default: 1000 requests per minute per merchant
- Configurable per merchant via `merchant_config` table
- Returns `429 Too Many Requests` with `Retry-After` header when exceeded
