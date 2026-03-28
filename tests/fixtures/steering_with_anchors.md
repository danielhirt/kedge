---
inclusion: fileMatch
fileMatchPattern: ["src/auth/**/*.java", "src/api/routes/auth.*"]
steer:
  group: payments-platform
  anchors:
    - repo: git@gitlab.example.com:team/backend.git
      path: src/auth/AuthService.java
      symbol: AuthService#validateToken
      provenance: a1b2c3d4
    - repo: git@gitlab.example.com:team/backend.git
      path: src/auth/AuthService.java
      symbol: AuthService#refreshSession
      provenance: a1b2c3d4
---

# Authentication Steering

This module handles JWT validation and session management.
