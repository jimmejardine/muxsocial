//! Worker-driven OAuth for the networks that need it (Mastodon now; Bluesky
//! later). The worker builds the authorize URL and performs all token work; the
//! main thread only opens the popup and returns the redirect query. No
//! per-request signer bridge is needed.
//!
//! Flow: `begin_oauth` registers/derives state, stows it as a [`PendingOauth`]
//! keyed by a minted id, and returns the authorize URL; the GUI opens it, the
//! user approves, and the callback's query comes back to `complete_oauth`, which
//! exchanges the code for credentials.

pub mod bluesky;
pub mod mastodon;
pub mod pkce;

use serde::Serialize;

/// Returned by `begin_oauth`: the URL to open and the id to pass back to
/// `complete_oauth`.
#[derive(Debug, Clone, Serialize)]
pub struct BeginOauthResult {
    pub authorize_url: String,
    pub oauth_flow_id: String,
}

/// In-flight OAuth state, held between `begin_oauth` and `complete_oauth`.
/// `BlueskyFlow` is boxed: it carries the whole OAuth client, which is much larger
/// than the Mastodon variant.
pub enum PendingOauth {
    Mastodon(mastodon::MastodonPendingOauth),
    Bluesky(Box<bluesky::BlueskyFlow>),
}

/// Read a single query-string value by `key`, percent-decoded. For the Bluesky
/// callback (`code`/`state`/`iss`/`error`).
pub fn query_value(redirect_query: &str, key: &str) -> Option<String> {
    let query = redirect_query.trim_start_matches('?');
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (pair_key, pair_value) = pair.split_once('=').unwrap_or((pair, ""));
        if pair_key == key {
            return Some(percent_decode(pair_value));
        }
    }
    None
}

/// Parse the OAuth redirect query (`?code=…&state=…` or an `error`), verifying
/// the `state` matches what we issued (CSRF guard) and returning the code.
pub fn extract_authorization_code(redirect_query: &str, expected_state: &str) -> anyhow::Result<String> {
    let query = redirect_query.trim_start_matches('?');
    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    let mut error: Option<String> = None;
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let decoded = percent_decode(value);
        match key {
            "code" => code = Some(decoded),
            "state" => state = Some(decoded),
            "error" => error = Some(decoded),
            "error_description" => error = error.or(Some(decoded)),
            _ => {}
        }
    }
    if let Some(error) = error {
        anyhow::bail!("authorization was denied or failed: {error}");
    }
    let state = state.ok_or_else(|| anyhow::anyhow!("OAuth callback missing state"))?;
    anyhow::ensure!(state == expected_state, "OAuth state mismatch (possible CSRF); ignoring callback");
    code.ok_or_else(|| anyhow::anyhow!("OAuth callback missing authorization code"))
}

/// Percent-encode one query-string component (everything outside the RFC3986
/// unreserved set is escaped). Matches JS `encodeURIComponent` closely enough for
/// OAuth params.
pub fn url_encode_component(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => encoded.push(byte as char),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// Percent-decode a query-string value (`%XX` escapes and `+` → space).
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok().and_then(|hex| u8::from_str_radix(hex, 16).ok());
                if let Some(byte) = hex {
                    decoded.push(byte);
                    index += 3;
                }
                else {
                    decoded.push(bytes[index]);
                    index += 1;
                }
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_code_when_state_matches() {
        let code = extract_authorization_code("?code=abc123&state=xyz", "xyz").expect("code");
        assert_eq!(code, "abc123");
    }

    #[test]
    fn rejects_state_mismatch() {
        assert!(extract_authorization_code("code=abc&state=evil", "xyz").is_err());
    }

    #[test]
    fn surfaces_provider_error() {
        let result = extract_authorization_code("error=access_denied&state=xyz", "xyz");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("access_denied"));
    }

    #[test]
    fn percent_decodes_values() {
        let code = extract_authorization_code("code=a%2Bb%3Dc&state=s", "s").expect("code");
        assert_eq!(code, "a+b=c");
    }

    #[test]
    fn encodes_reserved_characters() {
        assert_eq!(url_encode_component("a b/c?d"), "a%20b%2Fc%3Fd");
    }
}
