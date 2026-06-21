//! Mastodon OAuth2 — fully client-side (no proxy, no hosted metadata).
//!
//! `begin`: dynamically register an app on the instance (`POST /api/v1/apps`),
//! derive PKCE + state, and build the `/oauth/authorize` URL. `complete`:
//! exchange the returned code at `/oauth/token` (PKCE; `client_secret` included
//! for pre-PKCE instances) and read the account handle for the display label.
//!
//! Most instances are CORS-permissive for these endpoints, and `/oauth/authorize`
//! is a top-level navigation (CORS-exempt), so this works from the browser.

use anyhow::Context;
use serde::Deserialize;

use crate::http::{HttpRequest, HttpResponse, HttpTransport};
use crate::posting::oauth::pkce;
use crate::posting::oauth::url_encode_component;

/// The OAuth scopes requested. `read` lets us resolve the account handle for the
/// label; `write` lets us post.
const MASTODON_SCOPE: &str = "read write";

/// State held between [`begin`] and [`complete`] for one Mastodon OAuth flow.
pub struct MastodonPendingOauth {
    pub instance_base_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub code_verifier: String,
    pub state: String,
}

/// Register an app, derive PKCE/state, and return the pending flow plus the
/// authorize URL to open. `instance_or_host` may be a bare host or a URL;
/// `redirect_uri` is the SPA's OAuth callback (provided by the main thread).
pub async fn begin(http_transport: &impl HttpTransport, instance_or_host: &str, redirect_uri: &str) -> anyhow::Result<(MastodonPendingOauth, String)> {
    let instance_base_url = normalize_instance(instance_or_host);

    let registered: RegisteredApp = post_json(
        http_transport,
        &format!("{instance_base_url}/api/v1/apps"),
        &serde_json::json!({
            "client_name": "mux.social",
            "redirect_uris": redirect_uri,
            "scopes": MASTODON_SCOPE,
            "website": "https://mux.social",
        }),
    )
    .await
    .context("registering app on the Mastodon instance")?;

    let code_verifier = pkce::random_url_safe_token(48)?;
    let code_challenge = pkce::code_challenge_s256(&code_verifier);
    let state = pkce::random_url_safe_token(24)?;

    let authorize_url = format!(
        "{instance_base_url}/oauth/authorize?response_type=code&client_id={client_id}&redirect_uri={redirect}&scope={scope}&code_challenge={challenge}&code_challenge_method=S256&state={state}",
        client_id = url_encode_component(&registered.client_id),
        redirect = url_encode_component(redirect_uri),
        scope = url_encode_component(MASTODON_SCOPE),
        challenge = url_encode_component(&code_challenge),
        state = url_encode_component(&state),
    );

    let pending = MastodonPendingOauth {
        instance_base_url,
        client_id: registered.client_id,
        client_secret: registered.client_secret,
        redirect_uri: redirect_uri.to_string(),
        code_verifier,
        state,
    };
    Ok((pending, authorize_url))
}

/// Exchange `authorization_code` for an access token and resolve the account
/// handle. Returns `(instance_base_url, access_token, display_label)`.
pub async fn complete(http_transport: &impl HttpTransport, pending: &MastodonPendingOauth, authorization_code: &str) -> anyhow::Result<(String, String, String)> {
    let token: TokenResponse = post_json(
        http_transport,
        &format!("{}/oauth/token", pending.instance_base_url),
        &serde_json::json!({
            "grant_type": "authorization_code",
            "code": authorization_code,
            "client_id": pending.client_id,
            "client_secret": pending.client_secret,
            "redirect_uri": pending.redirect_uri,
            "code_verifier": pending.code_verifier,
            "scope": MASTODON_SCOPE,
        }),
    )
    .await
    .context("exchanging the Mastodon authorization code for a token")?;

    let account: VerifyCredentials = get_json_authenticated(http_transport, &format!("{}/api/v1/accounts/verify_credentials", pending.instance_base_url), &token.access_token).await.context("reading the Mastodon account")?;

    let instance_host = pending.instance_base_url.trim_start_matches("https://").trim_start_matches("http://");
    let display_label = format!("@{}@{}", account.username, instance_host);
    Ok((pending.instance_base_url.clone(), token.access_token, display_label))
}

/// Normalize a bare host or URL to `https://host` (no trailing slash).
fn normalize_instance(input: &str) -> String {
    let host = input.trim().trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/');
    format!("https://{host}")
}

async fn post_json<T: serde::de::DeserializeOwned>(http_transport: &impl HttpTransport, url: &str, payload: &serde_json::Value) -> anyhow::Result<T> {
    let request = HttpRequest {
        method: "POST".to_string(),
        url: url.to_string(),
        headers: vec![("Content-Type".to_string(), "application/json".to_string()), ("Accept".to_string(), "application/json".to_string())],
        body: serde_json::to_vec(payload)?,
    };
    parse_json(http_transport.execute(request).await?, url)
}

async fn get_json_authenticated<T: serde::de::DeserializeOwned>(http_transport: &impl HttpTransport, url: &str, access_token: &str) -> anyhow::Result<T> {
    let request = HttpRequest::get(url).header("Accept", "application/json").header("Authorization", format!("Bearer {access_token}"));
    parse_json(http_transport.execute(request).await?, url)
}

fn parse_json<T: serde::de::DeserializeOwned>(response: HttpResponse, url: &str) -> anyhow::Result<T> {
    anyhow::ensure!(response.is_success(), "HTTP {} from {url}: {}", response.status, String::from_utf8_lossy(&response.body));
    serde_json::from_slice(&response.body).map_err(|parse_error| anyhow::anyhow!("parsing response from {url}: {parse_error}"))
}

#[derive(Deserialize)]
struct RegisteredApp {
    client_id: String,
    client_secret: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct VerifyCredentials {
    username: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_instance_forms() {
        assert_eq!(normalize_instance("mastodon.social"), "https://mastodon.social");
        assert_eq!(normalize_instance("https://mastodon.social/"), "https://mastodon.social");
        assert_eq!(normalize_instance("  http://example.test  "), "https://example.test");
    }
}
