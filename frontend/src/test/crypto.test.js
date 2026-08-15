import { describe, it, expect, beforeEach } from 'vitest';
import { deriveKey, encryptText, decryptText } from '../lib/crypto.js';

describe('crypto.js — Client-Side Encryption/Decryption', () => {
  let cryptoKey;

  beforeEach(async () => {
    cryptoKey = await deriveKey('test-passphrase-123');
  });

  it.each([
    { passphrase: 'test-passphrase-123' },
    { passphrase: 'master-key-456' },
  ])('derives a valid CryptoKey from a passphrase ($passphrase)', async ({ passphrase }) => {
    const key = await deriveKey(passphrase);
    expect(key).toBeDefined();
    expect(key.type).toBe('secret');
    expect(key.algorithm.name).toBe('AES-GCM');
  });

  it.each([
    { originalText: 'Groceries & Household Essentials' },
    { originalText: 'Utilities & Bills 2026' },
  ])('encrypts and decrypts text correctly (roundtrip: $originalText)', async ({ originalText }) => {
    const ciphertext = await encryptText(originalText, cryptoKey);
    
    expect(ciphertext).toBeTypeOf('string');
    expect(ciphertext).not.toBe(originalText);

    const decryptedText = await decryptText(ciphertext, cryptoKey);
    expect(decryptedText).toBe(originalText);
  });

  it.each([
    { text: 'John' },
    { text: 'Jane' },
  ])('produces deterministic ciphertexts for identical plaintexts (static IV: $text)', async ({ text }) => {
    const cipher1 = await encryptText(text, cryptoKey);
    const cipher2 = await encryptText(text, cryptoKey);
    
    expect(cipher1).toBe(cipher2);
  });

  it.each([
    { val: '' },
    { val: null },
    { val: undefined },
  ])('handles edge case inputs ($val)', async ({ val }) => {
    expect(await encryptText(val, cryptoKey)).toBe(val);
    expect(await decryptText(val, cryptoKey)).toBe(val);
  });

  it.each([
    { specialText: 'Café ☕ / Über $100 & 50% split <script>alert("xss")</script>' },
    { specialText: '100% Euro 欧元 🚀' },
  ])('handles special characters, unicode, and emojis ($specialText)', async ({ specialText }) => {
    const ciphertext = await encryptText(specialText, cryptoKey);
    const decryptedText = await decryptText(ciphertext, cryptoKey);

    expect(decryptedText).toBe(specialText);
  });

  it.each([
    { wrongPass: 'wrong-passphrase-456' },
  ])('returns original ciphertext if decryption fails with an invalid key ($wrongPass)', async ({ wrongPass }) => {
    const wrongKey = await deriveKey(wrongPass);
    const originalText = 'Secret Data';
    const ciphertext = await encryptText(originalText, cryptoKey);

    const result = await decryptText(ciphertext, wrongKey);
    expect(result).toBe(ciphertext);
  });
});
