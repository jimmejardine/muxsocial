//! At-rest encryption for cross-post account credentials.
//!
//! Everything lives in the browser with no server, so the only thing protecting
//! a user's keys (Hashiverse keyphrase, nostr nsec, OAuth tokens/DPoP keys) is a
//! master password they enter once per session. We derive a symmetric key from
//! that password with **Argon2id** (memory-hard, salted) and encrypt each secret
//! with **XChaCha20-Poly1305** (AEAD, 192-bit nonce). Both are pure-Rust and
//! build clean on wasm32 (no clang, unlike nostr's secp256k1).
//!
//! The key is derived **once per session** from the master password + a single
//! store-level [`random_salt`] (persisted by the account store), held in memory
//! as a [`MasterKey`], and reused to encrypt/decrypt every credential. Each
//! [`EncryptedBlob`] only carries its own random nonce + ciphertext.

use anyhow::Context;
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};

/// Length of the store-level Argon2id salt, in bytes.
pub const ARGON2_SALT_LEN: usize = 16;
/// Length of the per-blob XChaCha20-Poly1305 nonce, in bytes (192 bits).
pub const XNONCE_LEN: usize = 24;
/// Derived key length, in bytes (XChaCha20-Poly1305 key = 256 bits).
const DERIVED_KEY_LEN: usize = 32;

// Fixed Argon2id parameters. Pinned (rather than `Argon2::default()`) so that a
// future bump of the `argon2` crate's defaults cannot make existing blobs
// underivable. These match the argon2 0.5 defaults (19 MiB, 2 passes, 1 lane).
const ARGON2_M_COST_KIB: u32 = 19_456;
const ARGON2_T_COST: u32 = 2;
const ARGON2_P_COST: u32 = 1;

/// One AEAD-encrypted secret: a fresh random nonce plus the ciphertext (which
/// includes the Poly1305 authentication tag). Base64 strings so it round-trips
/// through the string key/value [`crate::config_storage::ConfigStorage`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedBlob {
    /// Base64 of the 24-byte XChaCha20-Poly1305 nonce.
    pub nonce_b64: String,
    /// Base64 of the ciphertext (plaintext + 16-byte auth tag).
    pub ciphertext_b64: String,
}

/// A session master key: the 32 bytes derived from the user's master password.
/// Held in memory only while the account store is unlocked; never serialized.
pub struct MasterKey([u8; DERIVED_KEY_LEN]);

impl MasterKey {
    /// Derive the session key from `master_password` and the store-level `salt`
    /// (see [`random_salt`]) via Argon2id. Deterministic: the same password +
    /// salt always yields the same key, which is what lets a later session
    /// decrypt secrets stored in an earlier one.
    pub fn derive(master_password: &str, salt: &[u8]) -> anyhow::Result<MasterKey> {
        let params = Params::new(ARGON2_M_COST_KIB, ARGON2_T_COST, ARGON2_P_COST, Some(DERIVED_KEY_LEN)).map_err(|param_error| anyhow::anyhow!("invalid Argon2id params: {param_error}"))?;
        let argon2id = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut derived_key = [0u8; DERIVED_KEY_LEN];
        argon2id
            .hash_password_into(master_password.as_bytes(), salt, &mut derived_key)
            .map_err(|hash_error| anyhow::anyhow!("Argon2id key derivation failed: {hash_error}"))?;
        Ok(MasterKey(derived_key))
    }

    /// AEAD-encrypt `plaintext` under this key with a fresh random nonce.
    pub fn encrypt(&self, plaintext: &[u8]) -> anyhow::Result<EncryptedBlob> {
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.0));
        let mut nonce_bytes = [0u8; XNONCE_LEN];
        getrandom::getrandom(&mut nonce_bytes).map_err(|random_error| anyhow::anyhow!("generating nonce: {random_error}"))?;
        let nonce = XNonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plaintext).map_err(|encrypt_error| anyhow::anyhow!("XChaCha20-Poly1305 encryption failed: {encrypt_error}"))?;
        Ok(EncryptedBlob {
            nonce_b64: base64::engine::general_purpose::STANDARD.encode(nonce_bytes),
            ciphertext_b64: base64::engine::general_purpose::STANDARD.encode(ciphertext),
        })
    }

    /// AEAD-decrypt `blob`. Fails (authentication error) if the key is wrong
    /// (wrong master password) or the ciphertext/nonce was tampered with.
    pub fn decrypt(&self, blob: &EncryptedBlob) -> anyhow::Result<Vec<u8>> {
        let nonce_bytes = base64::engine::general_purpose::STANDARD.decode(&blob.nonce_b64).context("decoding nonce base64")?;
        let ciphertext = base64::engine::general_purpose::STANDARD.decode(&blob.ciphertext_b64).context("decoding ciphertext base64")?;
        if nonce_bytes.len() != XNONCE_LEN {
            anyhow::bail!("nonce has wrong length {} (expected {XNONCE_LEN})", nonce_bytes.len());
        }
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.0));
        let nonce = XNonce::from_slice(&nonce_bytes);
        // A failure here is the expected "wrong password / tampered data" path —
        // the AEAD tag did not verify. Keep the message generic (don't leak which).
        cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_decrypt_error| anyhow::anyhow!("could not decrypt credential (wrong master password or corrupt data)"))
    }
}

/// A fresh random store-level salt for [`MasterKey::derive`]. Persist it next to
/// the encrypted credentials so the same key can be re-derived in a later session.
pub fn random_salt() -> anyhow::Result<[u8; ARGON2_SALT_LEN]> {
    let mut salt = [0u8; ARGON2_SALT_LEN];
    getrandom::getrandom(&mut salt).map_err(|random_error| anyhow::anyhow!("generating salt: {random_error}"))?;
    Ok(salt)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A single fixed salt across these tests; the key derivation is the same as
    // production, just with a known salt so we exercise encrypt/decrypt.
    fn test_salt() -> [u8; ARGON2_SALT_LEN] {
        [7u8; ARGON2_SALT_LEN]
    }

    #[test]
    fn encrypts_and_decrypts_round_trip() {
        let key = MasterKey::derive("correct horse battery staple", &test_salt()).expect("derive");
        let secret = b"nsec1averysecretkeyvalue";
        let blob = key.encrypt(secret).expect("encrypt");
        let recovered = key.decrypt(&blob).expect("decrypt");
        assert_eq!(recovered, secret);
    }

    #[test]
    fn wrong_master_password_fails_to_decrypt() {
        let salt = test_salt();
        let blob = MasterKey::derive("the right password", &salt).expect("derive").encrypt(b"my keyphrase").expect("encrypt");
        let wrong_key = MasterKey::derive("the WRONG password", &salt).expect("derive");
        assert!(wrong_key.decrypt(&blob).is_err(), "decrypt with wrong password must fail");
    }

    #[test]
    fn tampered_ciphertext_fails_to_decrypt() {
        let key = MasterKey::derive("pw", &test_salt()).expect("derive");
        let mut blob = key.encrypt(b"sensitive").expect("encrypt");
        // Flip the last base64 char of the ciphertext to corrupt it.
        let mut ciphertext_bytes = base64::engine::general_purpose::STANDARD.decode(&blob.ciphertext_b64).expect("decode");
        let last = ciphertext_bytes.len() - 1;
        ciphertext_bytes[last] ^= 0x01;
        blob.ciphertext_b64 = base64::engine::general_purpose::STANDARD.encode(&ciphertext_bytes);
        assert!(key.decrypt(&blob).is_err(), "decrypt of tampered ciphertext must fail");
    }

    #[test]
    fn encrypting_twice_uses_distinct_nonces() {
        let key = MasterKey::derive("pw", &test_salt()).expect("derive");
        let first = key.encrypt(b"same plaintext").expect("encrypt");
        let second = key.encrypt(b"same plaintext").expect("encrypt");
        assert_ne!(first.nonce_b64, second.nonce_b64, "each encryption must use a fresh nonce");
        assert_ne!(first.ciphertext_b64, second.ciphertext_b64, "ciphertext must differ under distinct nonces");
    }

    #[test]
    fn random_salt_is_not_constant() {
        let first = random_salt().expect("salt");
        let second = random_salt().expect("salt");
        assert_ne!(first, second, "salts must be random");
    }
}
