package com.nexus.platform.util;

import javax.crypto.Cipher;
import javax.crypto.Mac;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;
import javax.crypto.spec.SecretKeySpec;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.util.Base64;

public class CryptoUtils {

    private static final String AES_GCM = "AES/GCM/NoPadding";
    private static final int GCM_IV_LENGTH = 12;
    private static final int GCM_TAG_LENGTH = 128;
    private static final String HMAC_SHA256 = "HmacSHA256";
    private static final SecureRandom SECURE_RANDOM = new SecureRandom();

    public static String encryptPan(String pan, byte[] keyBytes) throws Exception {
        if (pan == null || pan.length() < 13 || pan.length() > 19) {
            throw new IllegalArgumentException("PAN must be 13-19 digits");
        }
        if (!pan.matches("\\d+")) {
            throw new IllegalArgumentException("PAN must contain only digits");
        }

        SecretKey key = new SecretKeySpec(keyBytes, "AES");

        // Generate random IV
        byte[] iv = new byte[GCM_IV_LENGTH];
        SECURE_RANDOM.nextBytes(iv);

        Cipher cipher = Cipher.getInstance(AES_GCM);
        GCMParameterSpec spec = new GCMParameterSpec(GCM_TAG_LENGTH, iv);
        cipher.init(Cipher.ENCRYPT_MODE, key, spec);

        byte[] ciphertext = cipher.doFinal(pan.getBytes());

        // Prepend IV to ciphertext
        byte[] combined = new byte[iv.length + ciphertext.length];
        System.arraycopy(iv, 0, combined, 0, iv.length);
        System.arraycopy(ciphertext, 0, combined, iv.length, ciphertext.length);

        return Base64.getEncoder().encodeToString(combined);
    }

    public static String decryptPan(String encryptedPan, byte[] keyBytes) throws Exception {
        byte[] combined = Base64.getDecoder().decode(encryptedPan);

        // Extract IV and ciphertext
        byte[] iv = new byte[GCM_IV_LENGTH];
        byte[] ciphertext = new byte[combined.length - GCM_IV_LENGTH];
        System.arraycopy(combined, 0, iv, 0, GCM_IV_LENGTH);
        System.arraycopy(combined, GCM_IV_LENGTH, ciphertext, 0, ciphertext.length);

        SecretKey key = new SecretKeySpec(keyBytes, "AES");
        Cipher cipher = Cipher.getInstance(AES_GCM);
        GCMParameterSpec spec = new GCMParameterSpec(GCM_TAG_LENGTH, iv);
        cipher.init(Cipher.DECRYPT_MODE, key, spec);

        byte[] plaintext = cipher.doFinal(ciphertext);
        return new String(plaintext);
    }

    public static String computeHmac(String data, byte[] secretKey) throws Exception {
        Mac mac = Mac.getInstance(HMAC_SHA256);
        SecretKeySpec keySpec = new SecretKeySpec(secretKey, HMAC_SHA256);
        mac.init(keySpec);
        byte[] hmacBytes = mac.doFinal(data.getBytes());
        return Base64.getEncoder().encodeToString(hmacBytes);
    }

    public static boolean verifyHmac(String data, String expectedHmac, byte[] secretKey)
            throws Exception {
        String computedHmac = computeHmac(data, secretKey);
        // Constant-time comparison to prevent timing attacks
        return MessageDigest.isEqual(
                computedHmac.getBytes(), expectedHmac.getBytes());
    }

    public static String maskPan(String pan) {
        if (pan == null || pan.length() < 4) {
            return "****";
        }
        String lastFour = pan.substring(pan.length() - 4);
        return "*".repeat(pan.length() - 4) + lastFour;
    }
}
