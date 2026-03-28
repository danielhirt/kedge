package com.nexus.platform.service;

import com.nexus.platform.model.Payment;
import com.nexus.platform.model.Transaction;

import java.math.BigDecimal;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

public class PaymentService {

    private final Map<UUID, Payment> payments = new ConcurrentHashMap<>();
    private final Map<String, UUID> idempotencyStore = new ConcurrentHashMap<>();

    public Payment processPayment(String merchantId, BigDecimal amount, String currency,
                                  String cardToken, String idempotencyKey) {
        // Check idempotency
        UUID existingId = idempotencyStore.get(idempotencyKey);
        if (existingId != null) {
            return payments.get(existingId);
        }

        // Validate input
        if (amount == null || amount.compareTo(BigDecimal.ZERO) <= 0) {
            throw new IllegalArgumentException("Amount must be positive");
        }
        if (currency == null || currency.length() != 3) {
            throw new IllegalArgumentException("Currency must be a 3-letter ISO code");
        }

        // Create payment
        Payment payment = new Payment(merchantId, amount, currency, cardToken);

        // Authorize with card network (simplified)
        String authCode = authorizeWithNetwork(payment);
        if (authCode != null) {
            payment.setAuthorizationCode(authCode);
            payment.setStatus(Payment.Status.AUTHORIZED);
        } else {
            payment.setStatus(Payment.Status.FAILED);
        }

        // Store
        payments.put(payment.getId(), payment);
        idempotencyStore.put(idempotencyKey, payment.getId());

        return payment;
    }

    public Payment refundPayment(String paymentId, BigDecimal amount) {
        UUID id = UUID.fromString(paymentId);
        Payment payment = payments.get(id);
        if (payment == null) {
            throw new IllegalArgumentException("Payment not found: " + paymentId);
        }
        if (payment.getStatus() != Payment.Status.CAPTURED) {
            throw new IllegalStateException("Payment must be in CAPTURED status to refund");
        }
        if (amount.compareTo(payment.getAmount()) > 0) {
            throw new IllegalArgumentException("Refund amount exceeds captured amount");
        }

        // Process refund with network
        boolean refundSuccess = refundWithNetwork(payment, amount);
        if (refundSuccess) {
            if (amount.compareTo(payment.getAmount()) == 0) {
                payment.setStatus(Payment.Status.REFUNDED);
            } else {
                payment.setStatus(Payment.Status.PARTIALLY_REFUNDED);
            }
        }

        return payment;
    }

    public Optional<Payment> getPayment(String paymentId) {
        return Optional.ofNullable(payments.get(UUID.fromString(paymentId)));
    }

    private String authorizeWithNetwork(Payment payment) {
        // Simplified — in production this calls the card network API
        return "AUTH-" + payment.getId().toString().substring(0, 8).toUpperCase();
    }

    private boolean refundWithNetwork(Payment payment, BigDecimal amount) {
        // Simplified — in production this calls the card network API
        return true;
    }
}
