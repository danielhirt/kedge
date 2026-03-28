package com.nexus.platform.service;

import com.nexus.platform.model.Account;

import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Collectors;

public class AccountService {

    private final Map<UUID, Account> accounts = new ConcurrentHashMap<>();

    public Account createAccount(String merchantId, String businessName, String email) {
        // Validate uniqueness
        boolean exists = accounts.values().stream()
                .anyMatch(a -> a.getMerchantId().equals(merchantId));
        if (exists) {
            throw new IllegalArgumentException("Merchant ID already registered: " + merchantId);
        }

        Account account = new Account(merchantId, businessName, email);
        accounts.put(account.getId(), account);
        return account;
    }

    public Account verifyAccount(String accountId, String kycDocumentId) {
        Account account = getAccountOrThrow(accountId);
        if (account.getStatus() != Account.Status.PENDING_VERIFICATION) {
            throw new IllegalStateException("Account is not pending verification");
        }

        account.setKycDocumentId(kycDocumentId);
        account.activate();
        return account;
    }

    public Account suspendAccount(String accountId, String reason) {
        Account account = getAccountOrThrow(accountId);
        if (account.getStatus() == Account.Status.CLOSED) {
            throw new IllegalStateException("Cannot suspend a closed account");
        }
        account.setStatus(Account.Status.SUSPENDED);
        return account;
    }

    public Account upgradeTier(String accountId, Account.Tier newTier) {
        Account account = getAccountOrThrow(accountId);
        if (account.getStatus() != Account.Status.ACTIVE) {
            throw new IllegalStateException("Account must be active to upgrade tier");
        }
        account.setTier(newTier);

        // Adjust limits based on tier
        switch (newTier) {
            case PREMIUM -> account.setDailyTransactionLimit(50000);
            case ENTERPRISE -> account.setDailyTransactionLimit(500000);
            default -> account.setDailyTransactionLimit(10000);
        }

        return account;
    }

    public Optional<Account> getAccount(String accountId) {
        return Optional.ofNullable(accounts.get(UUID.fromString(accountId)));
    }

    public List<Account> listActiveAccounts() {
        return accounts.values().stream()
                .filter(a -> a.getStatus() == Account.Status.ACTIVE)
                .collect(Collectors.toList());
    }

    private Account getAccountOrThrow(String accountId) {
        UUID id = UUID.fromString(accountId);
        Account account = accounts.get(id);
        if (account == null) {
            throw new IllegalArgumentException("Account not found: " + accountId);
        }
        return account;
    }
}
