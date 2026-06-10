//! Bluesky source client — built on ATrium (`atrium-api` + `atrium-xrpc`).
//!
//! ATrium abstracts the XRPC HTTP transport behind the [`HttpClient`] trait, so
//! instead of pulling in atrium's own reqwest client we implement that trait
//! over our cross-platform [`crate::http::HttpTransport`]. This keeps Bluesky on
//! the same transport as Mastodon and wasm-ready.
//!
//! Public posts are readable unauthenticated from the public AppView
//! (`public.api.bsky.app`), which also sends permissive CORS headers, so this
//! works from a browser too.

use anyhow::{Context, anyhow};
use atrium_api::app::bsky::feed::get_author_feed;
use atrium_api::types::LimitedNonZeroU8;
use atrium_api::types::string::AtIdentifier;
use atrium_xrpc::{HttpClient, OutputDataOrBytes, XrpcClient, XrpcRequest};
use http::{Method, Request, Response};

use crate::http::{HttpRequest, HttpTransport};
use crate::post::{AggregatedPost, SourceNetwork, parse_rfc3339_to_epoch_millis};

/// The unauthenticated public AppView base URL.
pub const PUBLIC_APPVIEW_BASE_URL: &str = "https://public.api.bsky.app";

/// Adapts our [`HttpTransport`] to atrium's [`XrpcClient`].
struct TransportXrpcClient<'transport, T: HttpTransport> {
    base_uri: String,
    http_transport: &'transport T,
}

impl<'transport, T: HttpTransport + Sync> HttpClient for TransportXrpcClient<'transport, T> {
    async fn send_http(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let (request_parts, request_body) = request.into_parts();

        let headers = request_parts.headers.iter().map(|(name, value)| (name.as_str().to_string(), value.to_str().unwrap_or_default().to_string())).collect();
        let our_request = HttpRequest {
            method: request_parts.method.as_str().to_string(),
            url: request_parts.uri.to_string(),
            headers,
            body: request_body,
        };

        let our_response = self
            .http_transport
            .execute(our_request)
            .await
            .map_err(|transport_error| -> Box<dyn std::error::Error + Send + Sync + 'static> { format!("{transport_error:#}").into() })?;

        let mut response_builder = Response::builder().status(our_response.status);
        for (header_name, header_value) in our_response.headers {
            response_builder = response_builder.header(header_name, header_value);
        }
        response_builder.body(our_response.body).map_err(|build_error| -> Box<dyn std::error::Error + Send + Sync + 'static> { Box::new(build_error) })
    }
}

impl<'transport, T: HttpTransport + Sync> XrpcClient for TransportXrpcClient<'transport, T> {
    fn base_uri(&self) -> String {
        self.base_uri.clone()
    }
}

/// Fetch up to `limit` recent posts authored by `actor` (a handle like
/// `"bsky.app"` or a DID) from the public AppView. Unauthenticated.
pub async fn fetch_recent_posts(http_transport: &(impl HttpTransport + Sync), actor: &str, limit: u8) -> anyhow::Result<Vec<AggregatedPost>> {
    let xrpc_client = TransportXrpcClient {
        base_uri: PUBLIC_APPVIEW_BASE_URL.to_string(),
        http_transport,
    };

    let actor_identifier: AtIdentifier = actor.parse().map_err(|parse_error| anyhow!("invalid Bluesky actor {actor:?}: {parse_error}"))?;
    let parameters = get_author_feed::ParametersData {
        actor: actor_identifier,
        cursor: None,
        filter: Some("posts_no_replies".to_string()),
        include_pins: None,
        limit: LimitedNonZeroU8::try_from(limit.max(1)).ok(),
    };

    let xrpc_request = XrpcRequest {
        method: Method::GET,
        nsid: get_author_feed::NSID.to_string(),
        parameters: Some(parameters.into()),
        input: None,
        encoding: None,
    };

    let xrpc_response = xrpc_client
        .send_xrpc::<get_author_feed::Parameters, (), get_author_feed::Output, get_author_feed::Error>(&xrpc_request)
        .await
        .map_err(|xrpc_error| anyhow!("Bluesky getAuthorFeed failed: {xrpc_error:?}"))?;

    let author_feed = match xrpc_response {
        OutputDataOrBytes::Data(output) => output,
        OutputDataOrBytes::Bytes(_) => return Err(anyhow!("Bluesky getAuthorFeed returned an unexpected non-JSON response")),
    };

    author_feed.feed.iter().map(map_feed_view_post).collect()
}

fn map_feed_view_post(feed_view_post: &atrium_api::app::bsky::feed::defs::FeedViewPost) -> anyhow::Result<AggregatedPost> {
    let post_view = &feed_view_post.post;

    // The post record is loosely typed (`Unknown`); serialize it to JSON and
    // pull out the bits we need.
    let record_value = serde_json::to_value(&post_view.record).context("serializing Bluesky post record")?;
    let content_text = record_value.get("text").and_then(|value| value.as_str()).unwrap_or_default().to_string();

    let created_at_millis = record_value
        .get("createdAt")
        .and_then(|value| value.as_str())
        .and_then(|timestamp_text| parse_rfc3339_to_epoch_millis(timestamp_text).ok())
        .or_else(|| parse_rfc3339_to_epoch_millis(post_view.indexed_at.as_str()).ok())
        .unwrap_or_default();

    Ok(AggregatedPost {
        source: SourceNetwork::Bluesky,
        source_post_id: post_view.uri.clone(),
        author_identifier: post_view.author.handle.as_str().to_string(),
        author_display_name: post_view.author.display_name.clone(),
        created_at_millis,
        content_text,
    })
}
