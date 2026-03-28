---
inclusion: always
steer:
  group: payments-platform
  anchors:
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: core/src/main/java/com/nexus/platform/service/PaymentService.java
      symbol: PaymentService#processPayment
      provenance: e4a7f1b2
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: core/src/main/java/com/nexus/platform/service/AccountService.java
      symbol: AccountService#createAccount
      provenance: e4a7f1b2
---

# Nexus Payment Platform - Product Context

Nexus is a multi-tenant payment processing platform serving enterprise merchants
across North America and the EU. The platform processes card-present, card-not-present,
and ACH transactions with real-time fraud scoring.

## Key Features

- **Payment Processing**: Real-time authorization, capture, and settlement for
  credit/debit cards and ACH transfers
- **Fraud Detection**: Rule-based and ML-powered fraud scoring with configurable
  thresholds per merchant
- **Workflow Engine**: XML-based DSL for defining approval workflows, compliance
  checks, and routing rules
- **Multi-Currency**: Supports 28 currencies with real-time FX conversion

## Business Constraints

- PCI DSS Level 1 compliance required for all payment data handling
- Settlement must complete within T+1 for domestic and T+2 for cross-border
- 99.99% uptime SLA for the authorization endpoint
- All workflow changes require dual approval via the workflow-engine module
