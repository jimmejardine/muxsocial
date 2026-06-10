//! A tiny cross-platform HTTP transport.
//!
//! The whole app runs in the browser with no backend, so every HTTP-based
//! source client must work both natively (for `cargo test` / the integration
//! harness) and on `wasm32-unknown-unknown` (in the browser). This module hides
//! that split behind a single [`HttpTransport`] trait and a per-target
//! [`DefaultHttpTransport`]:
//!
//! - native: `reqwest` (rustls)
//! - wasm: `gloo-net` over the browser Fetch API
//!
//! nostr is intentionally NOT routed through here — it speaks WebSocket to
//! relays and uses `nostr-sdk`'s own transport. Hashiverse likewise uses
//! `hashiverse-lib`'s own DHT transport. This trait serves the request/response
//! HTTP networks: Mastodon (our own REST client) and Bluesky (the atrium XRPC
//! adapter in [`crate::sources::bluesky`]).

/// An outgoing HTTP request in owned, transport-agnostic form.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// HTTP method, e.g. `"GET"`.
    pub method: String,
    /// Absolute request URL.
    pub url: String,
    /// Header name/value pairs.
    pub headers: Vec<(String, String)>,
    /// Request body bytes (empty for a bodyless GET).
    pub body: Vec<u8>,
}

impl HttpRequest {
    /// A bodyless GET request to `url`.
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: "GET".to_string(),
            url: url.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Append a header, builder-style.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

/// An HTTP response in owned, transport-agnostic form.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response header name/value pairs.
    pub headers: Vec<(String, String)>,
    /// Raw response body bytes.
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Whether the status code is in the 2xx range.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// The cross-platform HTTP capability the source clients depend on.
///
/// On native targets the returned future must be `Send` (atrium's XRPC client
/// requires it); on wasm it must not (browser futures are `!Send`). The trait is
/// duplicated per target to express that — implementors just write
/// `async fn execute`.
#[cfg(not(target_arch = "wasm32"))]
pub trait HttpTransport {
    /// Execute `request` and return the response.
    fn execute(&self, request: HttpRequest) -> impl std::future::Future<Output = anyhow::Result<HttpResponse>> + Send;
}

#[cfg(target_arch = "wasm32")]
pub trait HttpTransport {
    /// Execute `request` and return the response.
    fn execute(&self, request: HttpRequest) -> impl std::future::Future<Output = anyhow::Result<HttpResponse>>;
}

#[cfg(not(target_arch = "wasm32"))]
mod reqwest_transport;
#[cfg(not(target_arch = "wasm32"))]
pub use reqwest_transport::DefaultHttpTransport;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod gloo_transport;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use gloo_transport::DefaultHttpTransport;

/// Construct the HTTP transport appropriate for the current build target.
pub fn default_http_transport() -> DefaultHttpTransport {
    DefaultHttpTransport::new()
}
