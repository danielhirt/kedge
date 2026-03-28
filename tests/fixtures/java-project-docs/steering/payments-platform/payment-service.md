---
inclusion: fileMatch
fileMatchPattern:
  - "core/src/main/java/**/PaymentService.java"
  - "core/src/main/java/**/Payment.java"
kedge:
  group: payments-platform
  anchors:
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: core/src/main/java/com/nexus/platform/service/PaymentService.java
      symbol: PaymentService#processPayment
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: core/src/main/java/com/nexus/platform/service/PaymentService.java
      symbol: PaymentService#refundPayment
      provenance: "sig:0000000000000000"
    - repo: git@github.com:nexus-corp/payment-platform.git
      path: core/src/main/java/com/nexus/platform/model/Payment.java
      symbol: Payment
      provenance: "sig:0000000000000000"
---

# Payment Service Architecture

The `PaymentService` is the core orchestrator for all payment operations. It
coordinates authorization, capture, and settlement across card networks and
ACH processors.

## processPayment

Handles the full lifecycle of a payment from authorization through capture.

**Parameters:**
- `PaymentRequest request` — contains amount, currency, merchant ID, and card token
- `String idempotencyKey` — prevents duplicate processing

**Flow:**
1. Validate the request fields and merchant configuration
2. Check for duplicate via idempotency key in Redis
3. Run the payment through the fraud detection workflow
4. Authorize with the card network (Visa/Mastercard/Amex)
5. On success, create a `Payment` record with status `AUTHORIZED`
6. Publish `payment.authorized` event to Kafka

**Returns:** `PaymentResult` with authorization code and payment ID

## refundPayment

Processes a full or partial refund against an existing captured payment.

**Parameters:**
- `String paymentId` — the original payment to refund
- `BigDecimal amount` — refund amount (must be <= original capture amount)

**Flow:**
1. Load the original payment and verify it is in `CAPTURED` status
2. Validate the refund amount does not exceed the captured amount
3. Submit refund to the card network
4. Update payment status to `REFUNDED` or `PARTIALLY_REFUNDED`

## Payment Model

The `Payment` entity tracks the full state of a payment through its lifecycle:
- `id` (UUID) — unique payment identifier
- `merchantId` — the merchant who initiated the payment
- `amount` / `currency` — monetary value
- `status` — one of: `PENDING`, `AUTHORIZED`, `CAPTURED`, `REFUNDED`, `FAILED`
- `authorizationCode` — returned by the card network on successful auth
- `createdAt` / `updatedAt` — audit timestamps
