//! Browser HTTP transport backed by `gloo-net` (the Fetch API).
//!
//! Only built for `wasm32-unknown-unknown`. Note that in the browser every
//! request is subject to CORS — networks without permissive
//! `Access-Control-Allow-Origin` headers (notably most Mastodon instances) will
//! need a proxy. That is out of scope for the first feasibility step.

use anyhow::{anyhow, bail};
use gloo_net::http::{Request, RequestBuilder};

use super::{HttpRequest, HttpResponse, HttpTransport};

/// The wasm HTTP transport. Stateless — each call builds a fresh fetch.
#[derive(Clone)]
pub struct DefaultHttpTransport;

impl DefaultHttpTransport {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultHttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransport for DefaultHttpTransport {
    async fn execute(&self, request: HttpRequest) -> anyhow::Result<HttpResponse> {
        let mut request_builder: RequestBuilder = match request.method.as_str() {
            "GET" => Request::get(&request.url),
            "POST" => Request::post(&request.url),
            "PUT" => Request::put(&request.url),
            "DELETE" => Request::delete(&request.url),
            other_method => bail!("unsupported HTTP method {other_method:?} on the wasm transport"),
        };
        for (header_name, header_value) in &request.headers {
            request_builder = request_builder.header(header_name, header_value);
        }

        // gloo-net's error type is not Send/Sync, so flatten it to a string
        // before it touches anyhow.
        log::debug!("http: {} {} ({} body bytes)", request.method, request.url, request.body.len());
        let response = if request.body.is_empty() {
            request_builder.send().await
        }
        else {
            // Send the raw body bytes as a Uint8Array (works for JSON/form/binary).
            let body_array = js_sys::Uint8Array::from(request.body.as_slice());
            request_builder.body(body_array).map_err(|build_error| anyhow!("building HTTP request body for {}: {build_error}", request.url))?.send().await
        };
        let response = response.map_err(|send_error| anyhow!("HTTP request to {} failed: {send_error}", request.url))?;

        let status = response.status();
        log::debug!("http: {status} <- {} {}", request.method, request.url);
        let headers = response.headers().entries().collect();
        let body = response.binary().await.map_err(|body_error| anyhow!("reading HTTP response body failed: {body_error}"))?;

        Ok(HttpResponse { status, headers, body })
    }
}
