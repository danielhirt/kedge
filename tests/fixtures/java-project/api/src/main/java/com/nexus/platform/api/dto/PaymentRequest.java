package com.nexus.platform.api.dto;

import java.math.BigDecimal;

public record PaymentRequest(
        String merchantId,
        BigDecimal amount,
        String currency,
        String cardToken,
        String description
) {
    public PaymentRequest {
        if (merchantId == null || merchantId.isBlank()) {
            throw new IllegalArgumentException("merchantId is required");
        }
        if (amount == null || amount.compareTo(BigDecimal.ZERO) <= 0) {
            throw new IllegalArgumentException("amount must be positive");
        }
        if (currency == null || currency.length() != 3) {
            throw new IllegalArgumentException("currency must be a 3-letter ISO code");
        }
        if (cardToken == null || cardToken.isBlank()) {
            throw new IllegalArgumentException("cardToken is required");
        }
    }
}
