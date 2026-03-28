package com.nexus.platform.api;

import com.nexus.platform.model.Account;
import com.nexus.platform.service.AccountService;

import java.util.List;
import java.util.Map;
import java.util.UUID;

public class AccountController {

    private final AccountService accountService;

    public AccountController(AccountService accountService) {
        this.accountService = accountService;
    }

    public Map<String, Object> createAccount(Map<String, String> body) {
        String merchantId = body.get("merchantId");
        String businessName = body.get("businessName");
        String email = body.get("email");

        Account account = accountService.createAccount(merchantId, businessName, email);
        return successResponse(accountToMap(account));
    }

    public Map<String, Object> getAccount(String accountId) {
        return accountService.getAccount(accountId)
                .map(a -> successResponse(accountToMap(a)))
                .orElseGet(() -> errorResponse("NOT_FOUND",
                        "Account not found: " + accountId));
    }

    public Map<String, Object> verifyAccount(String accountId, Map<String, String> body) {
        String kycDocumentId = body.get("kycDocumentId");
        Account account = accountService.verifyAccount(accountId, kycDocumentId);
        return successResponse(accountToMap(account));
    }

    public Map<String, Object> listAccounts(int page, int size) {
        List<Account> accounts = accountService.listActiveAccounts();
        int fromIndex = Math.min((page - 1) * size, accounts.size());
        int toIndex = Math.min(fromIndex + size, accounts.size());
        List<Map<String, Object>> items = accounts.subList(fromIndex, toIndex)
                .stream()
                .map(this::accountToMap)
                .toList();

        return Map.of(
                "data", items,
                "errors", List.of(),
                "meta", Map.of(
                        "requestId", UUID.randomUUID().toString(),
                        "totalCount", accounts.size(),
                        "totalPages", (int) Math.ceil((double) accounts.size() / size)
                )
        );
    }

    private Map<String, Object> accountToMap(Account account) {
        return Map.of(
                "id", account.getId().toString(),
                "merchantId", account.getMerchantId(),
                "businessName", account.getBusinessName(),
                "status", account.getStatus().name(),
                "tier", account.getTier().name()
        );
    }

    private Map<String, Object> successResponse(Object data) {
        return Map.of(
                "data", data,
                "errors", List.of(),
                "meta", Map.of("requestId", UUID.randomUUID().toString())
        );
    }

    private Map<String, Object> errorResponse(String code, String message) {
        return Map.of(
                "data", Map.of(),
                "errors", List.of(Map.of("code", code, "message", message)),
                "meta", Map.of("requestId", UUID.randomUUID().toString())
        );
    }
}
