//! PKCE (RFC 7636) helpers for the OAuth flows.

use base64::Engine;
use sha2::{Digest, Sha256};

/// A random URL-safe token of `num_bytes` entropy, base64url (no padding) — used
/// for the PKCE `code_verifier` and the CSRF `state`.
pub fn random_url_safe_token(num_bytes: usize) -> anyhow::Result<String> {
    let mut bytes = vec![0u8; num_bytes];
    getrandom::getrandom(&mut bytes).map_err(|random_error| anyhow::anyhow!("generating random token: {random_error}"))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

/// The S256 `code_challenge` for a `code_verifier`: base64url(SHA-256(verifier)).
pub fn code_challenge_s256(code_verifier: &str) -> String {
    let digest = Sha256::digest(code_verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_matches_rfc7636_appendix_b_vector() {
        // The canonical RFC 7636 Appendix B example.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(code_challenge_s256(verifier), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn tokens_are_random_and_url_safe() {
        let first = random_url_safe_token(32).expect("token");
        let second = random_url_safe_token(32).expect("token");
        assert_ne!(first, second);
        assert!(first.chars().all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_'));
    }
}
