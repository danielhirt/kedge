package com.nexus.platform.api.dto;

import com.nexus.platform.model.Payment;

import java.math.BigDecimal;

public record PaymentResponse(
        String id,
        String merchantId,
        String amount,
        String currency,
        String status,
        String authorizationCode,
        String createdAt
) {
    public static PaymentResponse from(Payment payment) {
        return new PaymentResponse(
                payment.getId().toString(),
                payment.getMerchantId(),
                payment.getAmount().toPlainString(),
                payment.getCurrency(),
                payment.getStatus().name(),
                payment.getAuthorizationCode(),
                payment.getCreatedAt().toString()
        );
    }
}
