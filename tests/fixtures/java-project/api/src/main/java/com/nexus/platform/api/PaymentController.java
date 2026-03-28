package com.nexus.platform.api;

import com.nexus.platform.api.dto.PaymentRequest;
import com.nexus.platform.api.dto.PaymentResponse;
import com.nexus.platform.model.Payment;
import com.nexus.platform.service.PaymentService;

import java.util.Map;
import java.util.UUID;

public class PaymentController {

    private final PaymentService paymentService;

    public PaymentController(PaymentService paymentService) {
        this.paymentService = paymentService;
    }

    public Map<String, Object> createPayment(PaymentRequest request, String idempotencyKey) {
        if (idempotencyKey == null || idempotencyKey.isBlank()) {
            return errorResponse("MISSING_IDEMPOTENCY_KEY",
                    "X-Idempotency-Key header is required");
        }

        Payment payment = paymentService.processPayment(
                request.merchantId(),
                request.amount(),
                request.currency(),
                request.cardToken(),
                idempotencyKey
        );

        PaymentResponse response = PaymentResponse.from(payment);
        return successResponse(response);
    }

    public Map<String, Object> getPayment(String paymentId) {
        return paymentService.getPayment(paymentId)
                .map(p -> successResponse(PaymentResponse.from(p)))
                .orElseGet(() -> errorResponse("NOT_FOUND",
                        "Payment not found: " + paymentId));
    }

    public Map<String, Object> refundPayment(String paymentId, Map<String, Object> body) {
        var amount = new java.math.BigDecimal(body.get("amount").toString());
        Payment payment = paymentService.refundPayment(paymentId, amount);
        return successResponse(PaymentResponse.from(payment));
    }

    private Map<String, Object> successResponse(Object data) {
        return Map.of(
                "data", data,
                "errors", java.util.List.of(),
                "meta", Map.of("requestId", UUID.randomUUID().toString())
        );
    }

    private Map<String, Object> errorResponse(String code, String message) {
        return Map.of(
                "data", Map.of(),
                "errors", java.util.List.of(Map.of("code", code, "message", message)),
                "meta", Map.of("requestId", UUID.randomUUID().toString())
        );
    }
}
