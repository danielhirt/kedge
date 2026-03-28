package com.nexus.platform.model;

import java.math.BigDecimal;
import java.time.Instant;
import java.util.UUID;

public class Payment {

    public enum Status {
        PENDING, AUTHORIZED, CAPTURED, REFUNDED, PARTIALLY_REFUNDED, FAILED
    }

    private UUID id;
    private String merchantId;
    private BigDecimal amount;
    private String currency;
    private Status status;
    private String authorizationCode;
    private String cardToken;
    private Instant createdAt;
    private Instant updatedAt;

    public Payment() {
        this.id = UUID.randomUUID();
        this.status = Status.PENDING;
        this.createdAt = Instant.now();
        this.updatedAt = Instant.now();
    }

    public Payment(String merchantId, BigDecimal amount, String currency, String cardToken) {
        this();
        this.merchantId = merchantId;
        this.amount = amount;
        this.currency = currency;
        this.cardToken = cardToken;
    }

    public UUID getId() { return id; }
    public void setId(UUID id) { this.id = id; }

    public String getMerchantId() { return merchantId; }
    public void setMerchantId(String merchantId) { this.merchantId = merchantId; }

    public BigDecimal getAmount() { return amount; }
    public void setAmount(BigDecimal amount) { this.amount = amount; }

    public String getCurrency() { return currency; }
    public void setCurrency(String currency) { this.currency = currency; }

    public Status getStatus() { return status; }
    public void setStatus(Status status) {
        this.status = status;
        this.updatedAt = Instant.now();
    }

    public String getAuthorizationCode() { return authorizationCode; }
    public void setAuthorizationCode(String authorizationCode) {
        this.authorizationCode = authorizationCode;
    }

    public String getCardToken() { return cardToken; }
    public void setCardToken(String cardToken) { this.cardToken = cardToken; }

    public Instant getCreatedAt() { return createdAt; }
    public Instant getUpdatedAt() { return updatedAt; }
}
