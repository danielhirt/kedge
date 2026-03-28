package com.nexus.platform.model;

import java.time.Instant;
import java.util.UUID;

public class Account {

    public enum Status {
        PENDING_VERIFICATION, ACTIVE, SUSPENDED, CLOSED
    }

    public enum Tier {
        STANDARD, PREMIUM, ENTERPRISE
    }

    private UUID id;
    private String merchantId;
    private String businessName;
    private String email;
    private Status status;
    private Tier tier;
    private String kycDocumentId;
    private int dailyTransactionLimit;
    private Instant createdAt;
    private Instant verifiedAt;

    public Account(String merchantId, String businessName, String email) {
        this.id = UUID.randomUUID();
        this.merchantId = merchantId;
        this.businessName = businessName;
        this.email = email;
        this.status = Status.PENDING_VERIFICATION;
        this.tier = Tier.STANDARD;
        this.dailyTransactionLimit = 10000;
        this.createdAt = Instant.now();
    }

    public UUID getId() { return id; }

    public String getMerchantId() { return merchantId; }

    public String getBusinessName() { return businessName; }
    public void setBusinessName(String businessName) { this.businessName = businessName; }

    public String getEmail() { return email; }
    public void setEmail(String email) { this.email = email; }

    public Status getStatus() { return status; }
    public void setStatus(Status status) { this.status = status; }

    public Tier getTier() { return tier; }
    public void setTier(Tier tier) { this.tier = tier; }

    public String getKycDocumentId() { return kycDocumentId; }
    public void setKycDocumentId(String kycDocumentId) { this.kycDocumentId = kycDocumentId; }

    public int getDailyTransactionLimit() { return dailyTransactionLimit; }
    public void setDailyTransactionLimit(int limit) { this.dailyTransactionLimit = limit; }

    public Instant getCreatedAt() { return createdAt; }

    public Instant getVerifiedAt() { return verifiedAt; }
    public void setVerifiedAt(Instant verifiedAt) { this.verifiedAt = verifiedAt; }

    public void activate() {
        this.status = Status.ACTIVE;
        this.verifiedAt = Instant.now();
    }
}
