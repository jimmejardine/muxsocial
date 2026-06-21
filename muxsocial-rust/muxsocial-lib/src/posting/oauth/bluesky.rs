//! Bluesky (ATProto) OAuth + DPoP, driven over our own [`HttpTransport`] via
//! `atrium-oauth`. The library owns the hard parts (PAR, DPoP, PKCE, token
//! refresh); we provide a wasm-capable `HttpClient`, an HTTP-only handle resolver
//! (the AppView's `resolveHandle` XRPC — no browser DNS), and the client
//! metadata.
//!
//! The generic atrium types are contained here behind [`BlueskyOAuthClient`] and
//! a few free functions; `SharedSourceWriters` special-cases Bluesky rather than
//! storing these generics in the `NetworkPoster` enum.

use std::sync::Arc;

use atrium_api::agent::{Agent, SessionManager};
use atrium_api::com::atproto::repo::create_record;
use atrium_api::types::Unknown;
use atrium_api::types::string::{AtIdentifier, Did, Nsid};
use atrium_common::store::Store;
use atrium_identity::did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL};
use atrium_identity::handle::{AppViewHandleResolver, AppViewHandleResolverConfig};
use atrium_oauth::store::session::{MemorySessionStore, Session};
use atrium_oauth::store::state::MemoryStateStore;
use atrium_oauth::{AtprotoClientMetadata, AtprotoLocalhostClientMetadata, AuthMethod, AuthorizeOptions, CallbackParams, GrantType, KnownScope, OAuthClient, OAuthClientConfig, OAuthResolverConfig, Scope};
use atrium_xrpc::HttpClient;
use atrium_xrpc::http::{Request, Response};

use crate::http::{DefaultHttpTransport, HttpRequest, HttpTransport, default_http_transport};
use crate::posting::PublishedPostReference;

/// The AppView used to resolve handles → DIDs (CORS-permissive HTTPS XRPC).
const APPVIEW_SERVICE_URL: &str = "https://bsky.social";
/// The default entryway when the user enters no handle.
const DEFAULT_ENTRYWAY: &str = "https://bsky.social";

/// An owned [`HttpClient`] over our cross-platform transport, for atrium's
/// resolvers and OAuth client. The wasm transport is a unit struct (so this is
/// `Send + Sync`, which atrium's resolver bounds require even on wasm).
pub struct OwnedHttpClient {
    http_transport: DefaultHttpTransport,
}

impl OwnedHttpClient {
    pub fn new() -> Self {
        Self { http_transport: default_http_transport() }
    }
}

impl Default for OwnedHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for OwnedHttpClient {
    async fn send_http(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let (request_parts, request_body) = request.into_parts();
        let headers = request_parts.headers.iter().map(|(name, value)| (name.as_str().to_string(), value.to_str().unwrap_or_default().to_string())).collect();
        let our_request = HttpRequest {
            method: request_parts.method.as_str().to_string(),
            url: request_parts.uri.to_string(),
            headers,
            body: request_body,
        };
        let our_response = self.http_transport.execute(our_request).await.map_err(|transport_error| -> Box<dyn std::error::Error + Send + Sync + 'static> { format!("{transport_error:#}").into() })?;
        let mut response_builder = Response::builder().status(our_response.status);
        for (header_name, header_value) in our_response.headers {
            response_builder = response_builder.header(header_name, header_value);
        }
        response_builder.body(our_response.body).map_err(|build_error| -> Box<dyn std::error::Error + Send + Sync + 'static> { Box::new(build_error) })
    }
}

/// Re-exported so callers can (de)serialize the persisted session without
/// depending on `atrium-oauth` directly.
pub use atrium_oauth::store::session::Session as BlueskySession;

type BlueskyDidResolver = CommonDidResolver<OwnedHttpClient>;
type BlueskyHandleResolver = AppViewHandleResolver<OwnedHttpClient>;

/// The concrete OAuth client type, hiding atrium's five generic parameters.
pub type BlueskyOAuthClient = OAuthClient<MemoryStateStore, MemorySessionStore, BlueskyDidResolver, BlueskyHandleResolver, OwnedHttpClient>;

/// In-flight Bluesky OAuth state, held between authorize and callback. Keeps the
/// client (its `MemoryStateStore` carries the PKCE/state) and a clone of the
/// session store (to extract the `Session` after callback).
pub struct BlueskyFlow {
    pub client: BlueskyOAuthClient,
    pub session_store: MemorySessionStore,
    pub client_id: String,
    pub redirect_uri: String,
    /// The handle the user typed, used as the display label (empty → use the DID).
    pub handle_label: String,
}

/// The scopes we request: `atproto` (identity) + `transition:generic` (write).
fn default_scopes() -> Vec<Scope> {
    vec![Scope::Known(KnownScope::Atproto), Scope::Known(KnownScope::TransitionGeneric)]
}

/// Build an OAuth client. `client_id` empty → the localhost dev client
/// (`AtprotoLocalhostClientMetadata`, for serving at `http://127.0.0.1`);
/// otherwise a hosted public client whose `client_id` is the URL of the served
/// `client-metadata-bluesky.json`. `session_store` is shared (clone) by the
/// caller so it can extract/inject the [`Session`].
pub fn build_client(client_id: &str, redirect_uri: &str, session_store: MemorySessionStore) -> anyhow::Result<BlueskyOAuthClient> {
    let resolver = OAuthResolverConfig {
        did_resolver: CommonDidResolver::new(CommonDidResolverConfig {
            plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.to_string(),
            http_client: Arc::new(OwnedHttpClient::new()),
        }),
        handle_resolver: AppViewHandleResolver::new(AppViewHandleResolverConfig {
            service_url: APPVIEW_SERVICE_URL.to_string(),
            http_client: Arc::new(OwnedHttpClient::new()),
        }),
        authorization_server_metadata: Default::default(),
        protected_resource_metadata: Default::default(),
    };

    if client_id.trim().is_empty() {
        let config = OAuthClientConfig {
            client_metadata: AtprotoLocalhostClientMetadata {
                redirect_uris: Some(vec![redirect_uri.to_string()]),
                scopes: Some(default_scopes()),
            },
            keys: None,
            resolver,
            state_store: MemoryStateStore::default(),
            session_store,
            http_client: OwnedHttpClient::new(),
        };
        OAuthClient::new(config).map_err(|oauth_error| anyhow::anyhow!("building Bluesky OAuth client (localhost): {oauth_error}"))
    }
    else {
        let config = OAuthClientConfig {
            client_metadata: AtprotoClientMetadata {
                client_id: client_id.to_string(),
                client_uri: None,
                redirect_uris: vec![redirect_uri.to_string()],
                token_endpoint_auth_method: AuthMethod::None,
                grant_types: vec![GrantType::AuthorizationCode, GrantType::RefreshToken],
                scopes: default_scopes(),
                jwks_uri: None,
                token_endpoint_auth_signing_alg: None,
            },
            keys: None,
            resolver,
            state_store: MemoryStateStore::default(),
            session_store,
            http_client: OwnedHttpClient::new(),
        };
        OAuthClient::new(config).map_err(|oauth_error| anyhow::anyhow!("building Bluesky OAuth client (hosted): {oauth_error}"))
    }
}

/// Build the authorize URL for `handle_or_entryway` (empty → the bsky.social
/// entryway, which lets the user pick their account at login).
pub async fn authorize(client: &BlueskyOAuthClient, handle_or_entryway: &str) -> anyhow::Result<String> {
    let input = if handle_or_entryway.trim().is_empty() { DEFAULT_ENTRYWAY.to_string() } else { handle_or_entryway.trim().to_string() };
    client
        .authorize(
            &input,
            AuthorizeOptions {
                scopes: default_scopes(),
                ..Default::default()
            },
        )
        .await
        .map_err(|authorize_error| anyhow::anyhow!("Bluesky authorize failed: {authorize_error}"))
}

/// Exchange the callback code for a session. Returns the account DID (string) and
/// the [`Session`] (DPoP key + tokens) for the caller to persist encrypted.
pub async fn complete(client: &BlueskyOAuthClient, session_store: &MemorySessionStore, code: String, state: Option<String>, iss: Option<String>) -> anyhow::Result<(String, Session)> {
    let (session, _app_state) = client.callback(CallbackParams { code, state, iss }).await.map_err(|callback_error| anyhow::anyhow!("Bluesky callback failed: {callback_error}"))?;
    let did = session.did().await.ok_or_else(|| anyhow::anyhow!("Bluesky session has no DID"))?;
    let stored = session_store.get(&did).await.map_err(|store_error| anyhow::anyhow!("reading Bluesky session: {store_error}"))?.ok_or_else(|| anyhow::anyhow!("Bluesky session missing after callback"))?;
    Ok((did.as_ref().to_string(), stored))
}

/// Restore `session` for `did_string` into a fresh client and publish a text post
/// (`app.bsky.feed.post`). On return, `session_store` holds the possibly-rotated
/// session for the caller to re-persist.
pub async fn create_text_post(client_id: &str, redirect_uri: &str, did_string: &str, session: Session, session_store: MemorySessionStore, text: &str, created_at_millis: i64) -> anyhow::Result<PublishedPostReference> {
    use atrium_common::store::Store;

    let did = Did::new(did_string.to_string()).map_err(|did_error| anyhow::anyhow!("invalid Bluesky DID {did_string}: {did_error}"))?;
    session_store.set(did.clone(), session).await.map_err(|store_error| anyhow::anyhow!("seeding Bluesky session: {store_error}"))?;

    let client = build_client(client_id, redirect_uri, session_store)?;
    let oauth_session = client.restore(&did).await.map_err(|restore_error| anyhow::anyhow!("restoring Bluesky session: {restore_error}"))?;
    let agent = Agent::new(oauth_session);

    let created_at = millis_to_rfc3339(created_at_millis);
    let record_json = serde_json::json!({ "$type": "app.bsky.feed.post", "text": text, "createdAt": created_at });
    let record: Unknown = serde_json::from_value(record_json).map_err(|record_error| anyhow::anyhow!("building Bluesky post record: {record_error}"))?;
    let collection = Nsid::new("app.bsky.feed.post".to_string()).map_err(|nsid_error| anyhow::anyhow!("nsid: {nsid_error}"))?;

    let input = create_record::InputData {
        collection,
        record,
        repo: AtIdentifier::Did(did.clone()),
        rkey: None,
        swap_commit: None,
        validate: None,
    };
    let output = agent.api.com.atproto.repo.create_record(input.into()).await.map_err(|post_error| anyhow::anyhow!("Bluesky createRecord failed: {post_error}"))?;

    Ok(PublishedPostReference {
        post_url: at_uri_to_bsky_url(did_string, &output.uri),
        native_post_id: Some(output.uri.clone()),
    })
}

/// A fresh, empty session store for one publish (or one OAuth flow).
pub fn new_session_store() -> MemorySessionStore {
    MemorySessionStore::default()
}

/// Read the (possibly refreshed) session for `did_string` back out of a store,
/// so the caller can re-persist it after a post or callback.
pub async fn read_session(session_store: &MemorySessionStore, did_string: &str) -> Option<Session> {
    let did = Did::new(did_string.to_string()).ok()?;
    session_store.get(&did).await.ok().flatten()
}

/// Format epoch millis as an RFC3339 string for the post `createdAt` (avoids
/// atrium's `Datetime::now`, which needs chrono's `clock` — unavailable on wasm).
fn millis_to_rfc3339(created_at_millis: i64) -> String {
    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(created_at_millis).unwrap_or_else(|| chrono::DateTime::<chrono::Utc>::from_timestamp_millis(0).expect("epoch is valid"));
    datetime.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Build a `bsky.app` permalink from the created record's AT-URI
/// (`at://did/app.bsky.feed.post/rkey`).
fn at_uri_to_bsky_url(did: &str, at_uri: &str) -> Option<String> {
    let rkey = at_uri.rsplit('/').next().filter(|rkey| !rkey.is_empty())?;
    Some(format!("https://bsky.app/profile/{did}/post/{rkey}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_bsky_permalink_from_at_uri() {
        assert_eq!(at_uri_to_bsky_url("did:plc:abc", "at://did:plc:abc/app.bsky.feed.post/3kxyz").as_deref(), Some("https://bsky.app/profile/did:plc:abc/post/3kxyz"));
        assert_eq!(at_uri_to_bsky_url("did:plc:abc", "at://"), None);
    }

    #[test]
    fn formats_millis_as_rfc3339_utc() {
        // 2021-01-01T00:00:00.000Z = 1609459200000 ms.
        assert_eq!(millis_to_rfc3339(1_609_459_200_000), "2021-01-01T00:00:00.000Z");
    }
}
