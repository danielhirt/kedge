---
inclusion: auto
name: security-guidelines
description: PCI DSS compliance, cryptographic standards, and secure coding practices for payment data handling. Use when modifying authentication, encryption, token handling, or any code that touches cardholder data.
---

# Security Guidelines

## PCI DSS Requirements

- Primary Account Numbers (PANs) must be encrypted at rest using AES-256-GCM
- PANs in logs must be masked to show only last 4 digits
- Encryption keys rotated quarterly; managed via AWS KMS
- No PAN data in query parameters, URL paths, or error messages

## Cryptographic Standards

- Use `CryptoUtils.encryptPan()` and `CryptoUtils.decryptPan()` for all PAN operations
- HMAC-SHA256 for webhook signature verification
- TLS 1.3 required for all external API calls
- No custom crypto implementations; use BouncyCastle provider

## Authentication

- JWT tokens signed with RS256 (RSA 2048-bit minimum)
- Token lifetime: 15 minutes for access tokens, 7 days for refresh tokens
- `AuthMiddleware.validateToken()` is the single entry point for token verification
- Failed auth attempts rate-limited to 5 per minute per IP
