---
inclusion: auto
name: crypto-standards
description: Cryptographic implementation standards for PAN encryption, key management, and signature verification. Use when reviewing or modifying code in CryptoUtils or any PCI-sensitive code paths.
kedge:
  group: platform-security
  anchors:
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: core/src/main/java/com/nexus/platform/util/CryptoUtils.java
      symbol: CryptoUtils#encryptPan
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: core/src/main/java/com/nexus/platform/util/CryptoUtils.java
      symbol: CryptoUtils#decryptPan
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: core/src/main/java/com/nexus/platform/util/CryptoUtils.java
      symbol: CryptoUtils#verifyHmac
      provenance: "sig:0000000000000000"
---

# Cryptographic Standards

## PAN Encryption

All Primary Account Numbers must be encrypted using AES-256-GCM before
storage or transmission between internal services.

### encryptPan

- **Input:** plaintext PAN string (13-19 digits)
- **Output:** Base64-encoded ciphertext with prepended 12-byte IV
- **Key:** Retrieved from AWS KMS; cached in-process for 1 hour
- **IV:** Randomly generated per encryption (never reused)

### decryptPan

- **Input:** Base64-encoded ciphertext (as produced by `encryptPan`)
- **Output:** plaintext PAN string
- **Key:** Same KMS key as encryption; key ID stored in ciphertext metadata

## HMAC Verification

### verifyHmac

Used for webhook signature verification from card network callbacks.

- **Algorithm:** HMAC-SHA256
- **Key:** Per-merchant webhook secret from Secrets Manager
- **Comparison:** Constant-time comparison to prevent timing attacks

## Key Rotation

- Encryption keys rotated quarterly via AWS KMS automatic rotation
- Old keys retained for decryption of existing ciphertext
- Re-encryption job runs monthly to migrate ciphertext to latest key version
