// crypto.js — Client-side encryption/decryption using Web Crypto API.
// Deterministic AES-GCM (fixed IV) is used to preserve SQLite GROUP BY/FK functionality.

const STATIC_IV = new Uint8Array([106, 105, 122, 105, 102, 105, 110, 45, 99, 114, 121, 112]); // "jizifin-cryp" (12 bytes)
const SALT = new TextEncoder().encode("jizifin-salt-pbkdf2");

/**
 * Derive a CryptoKey from the user salt.
 * @param {string} saltText
 * @returns {Promise<CryptoKey>}
 */
export async function deriveKey(saltText) {
  const enc = new TextEncoder();
  const keyMaterial = await window.crypto.subtle.importKey(
    "raw",
    enc.encode(saltText),
    { name: "PBKDF2" },
    false,
    ["deriveBits", "deriveKey"]
  );
  return window.crypto.subtle.deriveKey(
    {
      name: "PBKDF2",
      salt: SALT,
      iterations: 100000,
      hash: "SHA-256"
    },
    keyMaterial,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"]
  );
}

/**
 * Converts a Uint8Array to a Base64 string without using spread operator.
 * Avoids stack overflow for large buffers and is faster than spread-based approach.
 * @param {Uint8Array} bytes
 * @returns {string}
 */
function uint8ArrayToBase64(bytes) {
  let binary = '';
  const len = bytes.byteLength;
  for (let i = 0; i < len; i++) {
    binary += String.fromCodePoint(bytes[i]);
  }
  return btoa(binary);
}

/**
 * Encrypt plaintext using the derived key.
 * Returns Base64URL string (no padding).
 * @param {string} plaintext
 * @param {CryptoKey} key
 * @returns {Promise<string>}
 */
export async function encryptText(plaintext, key) {
  if (!plaintext) return plaintext;
  const enc = new TextEncoder();
  const encrypted = await window.crypto.subtle.encrypt(
    { name: "AES-GCM", iv: STATIC_IV },
    key,
    enc.encode(plaintext)
  );
  const base64 = uint8ArrayToBase64(new Uint8Array(encrypted));
  // Convert standard Base64 to Base64URL — use replaceAll for linear O(n) performance
  return base64.replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '');
}

/**
 * Decrypt ciphertext using the derived key.
 * If decryption fails (wrong key/salt), returns the original ciphertext (garbled data).
 * @param {string} ciphertext
 * @param {CryptoKey} key
 * @returns {Promise<string>}
 */
export async function decryptText(ciphertext, key) {
  if (!ciphertext) return ciphertext;
  try {
    // Convert Base64URL to standard Base64
    let base64 = ciphertext.replaceAll('-', '+').replaceAll('_', '/');
    while (base64.length % 4) {
      base64 += '=';
    }

    const binaryStr = atob(base64);
    const len = binaryStr.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
      bytes[i] = binaryStr.codePointAt(i);
    }
    const decrypted = await window.crypto.subtle.decrypt(
      { name: "AES-GCM", iv: STATIC_IV },
      key,
      bytes
    );
    return new TextDecoder().decode(decrypted);
  } catch {
    // If decryption fails, it is garbled/unauthorized access. Return original ciphertext.
    return ciphertext;
  }
}
