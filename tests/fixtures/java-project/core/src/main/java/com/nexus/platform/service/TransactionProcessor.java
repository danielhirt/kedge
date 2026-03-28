package com.nexus.platform.service;

import com.nexus.platform.model.Payment;
import com.nexus.platform.model.Transaction;

import java.math.BigDecimal;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

public class TransactionProcessor {

    private final Map<UUID, List<Transaction>> transactionLog = new ConcurrentHashMap<>();
    private final PaymentService paymentService;

    public TransactionProcessor(PaymentService paymentService) {
        this.paymentService = paymentService;
    }

    public Transaction settle(String paymentId, SettlementOptions options) {
        Payment payment = paymentService.getPayment(paymentId)
                .orElseThrow(() -> new IllegalArgumentException("Payment not found: " + paymentId));

        if (payment.getStatus() != Payment.Status.AUTHORIZED) {
            throw new IllegalStateException(
                    "Payment must be AUTHORIZED to settle, current status: " + payment.getStatus());
        }

        BigDecimal settleAmount = options.partialAmount() != null
                ? options.partialAmount()
                : payment.getAmount();

        // Create settlement transaction
        Transaction txn = new Transaction(
                payment.getId(), Transaction.Type.CAPTURE, settleAmount, payment.getCurrency());

        // Submit to settlement network (simplified)
        String networkRef = submitSettlement(payment, settleAmount);
        txn.setNetworkReference(networkRef);
        txn.setResponseCode("00");
        txn.setSuccess(true);

        // Update payment status
        payment.setStatus(Payment.Status.CAPTURED);

        // Log transaction
        transactionLog.computeIfAbsent(payment.getId(), k -> new ArrayList<>()).add(txn);

        return txn;
    }

    public List<Transaction> batchSettle(String merchantId, LocalDate from, LocalDate to) {
        List<Transaction> results = new ArrayList<>();
        int batchSize = 0;
        int maxBatchSize = 10000;

        // In production, this queries the database for authorized payments
        // within the date range for the given merchant
        for (Map.Entry<UUID, List<Transaction>> entry : transactionLog.entrySet()) {
            if (batchSize >= maxBatchSize) {
                break;
            }

            Payment payment = paymentService.getPayment(entry.getKey().toString())
                    .orElse(null);
            if (payment != null
                    && payment.getMerchantId().equals(merchantId)
                    && payment.getStatus() == Payment.Status.AUTHORIZED) {
                try {
                    Transaction txn = settle(
                            payment.getId().toString(), SettlementOptions.defaults());
                    results.add(txn);
                    batchSize++;
                } catch (Exception e) {
                    // Log and continue — partial batch failures are acceptable
                }
            }
        }

        return results;
    }

    public List<Transaction> getTransactionHistory(String paymentId) {
        UUID id = UUID.fromString(paymentId);
        return transactionLog.getOrDefault(id, List.of());
    }

    private String submitSettlement(Payment payment, BigDecimal amount) {
        // Simplified — in production generates ISO 20022 message via schema-definitions
        return "STL-" + payment.getId().toString().substring(0, 8).toUpperCase();
    }

    public record SettlementOptions(BigDecimal partialAmount, boolean priority) {
        public static SettlementOptions defaults() {
            return new SettlementOptions(null, false);
        }
    }
}
