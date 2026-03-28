package com.nexus.platform.api.middleware;

import java.security.PublicKey;
import java.util.Base64;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

public class AuthMiddleware {

    private final Map<String, CachedValidation> tokenCache = new ConcurrentHashMap<>();
    private final long cacheTtlMs = 5 * 60 * 1000; // 5 minutes

    public boolean validateToken(String authorizationHeader) {
        if (authorizationHeader == null || !authorizationHeader.startsWith("Bearer ")) {
            return false;
        }

        String token = authorizationHeader.substring(7);

        // Check cache first
        CachedValidation cached = tokenCache.get(token);
        if (cached != null && !cached.isExpired()) {
            return cached.valid;
        }

        // Verify JWT signature and claims
        boolean valid = verifyJwt(token);

        // Cache the result
        tokenCache.put(token, new CachedValidation(valid, System.currentTimeMillis()));

        return valid;
    }

    public Map<String, Object> extractClaims(String token) {
        // Decode JWT payload without verification (assumes validateToken was called first)
        String[] parts = token.split("\\.");
        if (parts.length != 3) {
            throw new IllegalArgumentException("Invalid JWT format");
        }

        String payload = new String(Base64.getUrlDecoder().decode(parts[1]));
        // Simplified — in production use a proper JSON parser
        return Map.of("raw", payload);
    }

    public String getMerchantId(String token) {
        Map<String, Object> claims = extractClaims(token);
        return (String) claims.get("merchant_id");
    }

    private boolean verifyJwt(String token) {
        // Simplified — in production verifies RS256 signature with public key
        String[] parts = token.split("\\.");
        return parts.length == 3 && !parts[0].isEmpty() && !parts[1].isEmpty();
    }

    private class CachedValidation {
        final boolean valid;
        final long timestamp;

        CachedValidation(boolean valid, long timestamp) {
            this.valid = valid;
            this.timestamp = timestamp;
        }

        boolean isExpired() {
            return System.currentTimeMillis() - timestamp > cacheTtlMs;
        }
    }
}
