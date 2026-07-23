import { describe, it, expect, beforeEach } from 'vitest';
import { deriveKey, encryptText, decryptText } from '../lib/crypto.js';

describe('crypto.js — Client-Side Encryption/Decryption', () => {
  let cryptoKey;

  beforeEach(async () => {
    cryptoKey = await deriveKey('test-passphrase-123');
  });

  it('derives a valid CryptoKey from a passphrase', async () => {
    expect(cryptoKey).toBeDefined();
    expect(cryptoKey.type).toBe('secret');
    expect(cryptoKey.algorithm.name).toBe('AES-GCM');
  });

  it('encrypts and decrypts text correctly (roundtrip)', async () => {
    const originalText = 'Groceries & Household Essentials';
    const ciphertext = await encryptText(originalText, cryptoKey);
    
    expect(ciphertext).toBeTypeOf('string');
    expect(ciphertext).not.toBe(originalText);

    const decryptedText = await decryptText(ciphertext, cryptoKey);
    expect(decryptedText).toBe(originalText);
  });

  it('produces deterministic ciphertexts for identical plaintexts (static IV)', async () => {
    const text = 'Jim';
    const cipher1 = await encryptText(text, cryptoKey);
    const cipher2 = await encryptText(text, cryptoKey);
    
    expect(cipher1).toBe(cipher2);
  });

  it('handles edge cases: empty strings, null, and undefined', async () => {
    expect(await encryptText('', cryptoKey)).toBe('');
    expect(await encryptText(null, cryptoKey)).toBe(null);
    expect(await encryptText(undefined, cryptoKey)).toBe(undefined);

    expect(await decryptText('', cryptoKey)).toBe('');
    expect(await decryptText(null, cryptoKey)).toBe(null);
    expect(await decryptText(undefined, cryptoKey)).toBe(undefined);
  });

  it('handles special characters, unicode, and emojis', async () => {
    const specialText = 'Café ☕ / Über $100 & 50% split <script>alert("xss")</script>';
    const ciphertext = await encryptText(specialText, cryptoKey);
    const decryptedText = await decryptText(ciphertext, cryptoKey);

    expect(decryptedText).toBe(specialText);
  });

  it('returns original ciphertext if decryption fails with an invalid key', async () => {
    const wrongKey = await deriveKey('wrong-passphrase-456');
    const originalText = 'Secret Data';
    const ciphertext = await encryptText(originalText, cryptoKey);

    const result = await decryptText(ciphertext, wrongKey);
    // When decryption fails, decryptText catches error and returns ciphertext as fallback
    expect(result).toBe(ciphertext);
  });
});
