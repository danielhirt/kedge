package com.nexus.platform.api.middleware;

import java.time.Instant;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;

public class RateLimiter {

    private static final int DEFAULT_LIMIT = 1000; // requests per minute
    private static final long WINDOW_MS = 60_000; // 1 minute

    private final Map<String, SlidingWindow> windows = new ConcurrentHashMap<>();
    private final Map<String, Integer> merchantLimits = new ConcurrentHashMap<>();

    public RateLimitResult checkLimit(String merchantId) {
        int limit = merchantLimits.getOrDefault(merchantId, DEFAULT_LIMIT);
        SlidingWindow window = windows.computeIfAbsent(merchantId,
                k -> new SlidingWindow());

        long now = System.currentTimeMillis();
        window.evictExpired(now);

        int currentCount = window.count.get();
        if (currentCount >= limit) {
            long retryAfterSeconds = (window.windowStart + WINDOW_MS - now) / 1000 + 1;
            return new RateLimitResult(false, currentCount, limit,
                    Math.max(1, retryAfterSeconds));
        }

        window.increment(now);
        return new RateLimitResult(true, currentCount + 1, limit, 0);
    }

    public void setMerchantLimit(String merchantId, int limit) {
        merchantLimits.put(merchantId, limit);
    }

    public void resetLimit(String merchantId) {
        windows.remove(merchantId);
    }

    public record RateLimitResult(
            boolean allowed,
            int currentCount,
            int limit,
            long retryAfterSeconds
    ) {}

    private static class SlidingWindow {
        final AtomicInteger count = new AtomicInteger(0);
        volatile long windowStart = System.currentTimeMillis();

        void evictExpired(long now) {
            if (now - windowStart > WINDOW_MS) {
                count.set(0);
                windowStart = now;
            }
        }

        void increment(long now) {
            evictExpired(now);
            count.incrementAndGet();
        }
    }
}
