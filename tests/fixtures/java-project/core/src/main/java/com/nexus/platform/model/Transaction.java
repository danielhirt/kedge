package com.nexus.platform.model;

import java.math.BigDecimal;
import java.time.Instant;
import java.util.UUID;

public class Transaction {

    public enum Type {
        AUTHORIZATION, CAPTURE, REFUND, VOID
    }

    private UUID id;
    private UUID paymentId;
    private Type type;
    private BigDecimal amount;
    private String currency;
    private String networkReference;
    private String responseCode;
    private boolean success;
    private Instant processedAt;

    public Transaction(UUID paymentId, Type type, BigDecimal amount, String currency) {
        this.id = UUID.randomUUID();
        this.paymentId = paymentId;
        this.type = type;
        this.amount = amount;
        this.currency = currency;
        this.processedAt = Instant.now();
    }

    public UUID getId() { return id; }

    public UUID getPaymentId() { return paymentId; }

    public Type getType() { return type; }

    public BigDecimal getAmount() { return amount; }

    public String getCurrency() { return currency; }

    public String getNetworkReference() { return networkReference; }
    public void setNetworkReference(String networkReference) {
        this.networkReference = networkReference;
    }

    public String getResponseCode() { return responseCode; }
    public void setResponseCode(String responseCode) { this.responseCode = responseCode; }

    public boolean isSuccess() { return success; }
    public void setSuccess(boolean success) { this.success = success; }

    public Instant getProcessedAt() { return processedAt; }
}
