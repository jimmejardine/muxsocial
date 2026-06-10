//! Native HTTP transport backed by `reqwest` (rustls).

use anyhow::Context;

use super::{HttpRequest, HttpResponse, HttpTransport};

/// The native HTTP transport. Holds a shared `reqwest::Client` (connection pool).
pub struct DefaultHttpTransport {
    reqwest_client: reqwest::Client,
}

impl DefaultHttpTransport {
    pub fn new() -> Self {
        Self { reqwest_client: reqwest::Client::new() }
    }
}

impl Default for DefaultHttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransport for DefaultHttpTransport {
    async fn execute(&self, request: HttpRequest) -> anyhow::Result<HttpResponse> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes()).with_context(|| format!("invalid HTTP method {:?}", request.method))?;

        let mut request_builder = self.reqwest_client.request(method, &request.url);
        for (header_name, header_value) in &request.headers {
            request_builder = request_builder.header(header_name, header_value);
        }
        if !request.body.is_empty() {
            request_builder = request_builder.body(request.body);
        }

        log::debug!("http: {} {}", request.method, request.url);
        let response = request_builder.send().await.with_context(|| format!("HTTP request to {} failed", request.url))?;
        let status = response.status().as_u16();
        log::debug!("http: {status} <- {} {}", request.method, request.url);
        let headers = response.headers().iter().map(|(name, value)| (name.as_str().to_string(), value.to_str().unwrap_or_default().to_string())).collect();
        let body = response.bytes().await.context("reading HTTP response body failed")?.to_vec();

        Ok(HttpResponse { status, headers, body })
    }
}
