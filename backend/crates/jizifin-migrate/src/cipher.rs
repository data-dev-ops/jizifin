//! Recomputes the client's deterministic ciphertext for known plaintext markers.
//! Mirrors `frontend/src/lib/crypto.js`: PBKDF2-SHA256 (100 000 rounds, static salt),
//! AES-256-GCM with a static IV, Base64URL without padding.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::Sha256;

const SALT: &[u8] = b"jizifin-salt-pbkdf2";
const IV: &[u8; 12] = b"jizifin-cryp";
const ROUNDS: u32 = 100_000;

/// Deterministic encryptor for marker strings.
pub struct MarkerCipher {
    cipher: Aes256Gcm,
}

impl MarkerCipher {
    /// Derives the key from `passphrase`.
    pub fn new(passphrase: &str) -> Self {
        let mut key = [0u8; 32];
        pbkdf2::pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), SALT, ROUNDS, &mut key);
        Self { cipher: Aes256Gcm::new(&key.into()) }
    }

    /// Ciphertext of `plaintext` exactly as the browser produces it.
    pub fn encrypt(&self, plaintext: &str) -> String {
        let sealed = self.cipher.encrypt(&Nonce::from(*IV), plaintext.as_bytes()).unwrap_or_default();
        URL_SAFE_NO_PAD.encode(sealed)
    }
}

/// Ciphertexts of every name the legacy code hard-coded.
pub struct Markers {
    /// Personal-pay categories (`PERSONAL COST`, `LEISURE`, `GIFT`).
    pub personal: Vec<String>,
    /// `Combined Fixed` category.
    pub combined_fixed: String,
    /// `Apartment` category.
    pub apartment: String,
    /// `Jane` member.
    pub jane: String,
    /// `John` member.
    pub john: String,
    /// `SALARY` income category.
    pub salary: String,
}

impl Markers {
    /// Encrypts every marker with the household passphrase.
    pub fn derive(passphrase: &str) -> Self {
        let c = MarkerCipher::new(passphrase);
        Self {
            personal: ["PERSONAL COST", "LEISURE", "GIFT"].iter().map(|p| c.encrypt(p)).collect(),
            combined_fixed: c.encrypt("Combined Fixed"),
            apartment: c.encrypt("Apartment"),
            jane: c.encrypt("Jane"),
            john: c.encrypt("John"),
            salary: c.encrypt("SALARY"),
        }
    }
}
