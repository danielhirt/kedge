---
inclusion: fileMatch
fileMatchPattern: "core/src/main/java/**/TransactionProcessor.java"
steer:
  group: payments-platform
  anchors:
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: core/src/main/java/com/nexus/platform/service/TransactionProcessor.java
      symbol: TransactionProcessor#settle
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: core/src/main/java/com/nexus/platform/service/TransactionProcessor.java
      symbol: TransactionProcessor#batchSettle
      provenance: "sig:0000000000000000"
---

# Transaction Processing

The `TransactionProcessor` handles settlement — the process of moving authorized
funds from the cardholder's bank to the merchant's account.

## settle

Settles a single authorized payment.

**Parameters:**
- `String paymentId` — the authorized payment to settle
- `SettlementOptions options` — optional overrides (partial amount, priority)

**Flow:**
1. Load the payment; verify status is `AUTHORIZED`
2. Generate the settlement instruction in ISO 20022 format using `schema-definitions`
3. Submit to the settlement network
4. Update payment status to `CAPTURED`
5. Publish `payment.captured` event

## batchSettle

Settles all authorized payments for a given merchant within a date range.
Used by the nightly settlement job.

**Parameters:**
- `String merchantId` — merchant to settle for
- `LocalDate from` / `LocalDate to` — date range

**Constraints:**
- Maximum 10,000 payments per batch (paginate if more)
- Failed settlements retry up to 3 times with exponential backoff
- Partial batch failures do not roll back successful settlements
