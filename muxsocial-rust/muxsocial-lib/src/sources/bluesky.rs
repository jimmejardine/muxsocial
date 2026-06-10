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

use anyhow::anyhow;
use atrium_api::app::bsky::feed::{get_author_feed, post};
use atrium_api::app::bsky::richtext::facet::{Main as Facet, MainFeaturesItem};
use atrium_api::types::string::AtIdentifier;
use atrium_api::types::{LimitedNonZeroU8, TryFromUnknown, Union};
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

    // The post record is loosely typed; recover the typed post Record so we can
    // turn its text + richtext facets into inline HTML. If it isn't a post
    // record we can decode, fall back to empty content rather than failing.
    let post_record = post::RecordData::try_from_unknown(post_view.record.clone());
    let (content_text, record_created_at_millis) = match &post_record {
        Ok(record) => {
            let content_html = render_facets_to_html(&record.text, record.facets.as_deref().unwrap_or(&[]));
            (content_html, parse_rfc3339_to_epoch_millis(record.created_at.as_str()).ok())
        }
        Err(_) => (String::new(), None),
    };

    let created_at_millis = record_created_at_millis.or_else(|| parse_rfc3339_to_epoch_millis(post_view.indexed_at.as_str()).ok()).unwrap_or_default();

    Ok(AggregatedPost {
        source: SourceNetwork::Bluesky,
        source_post_id: post_view.uri.clone(),
        author_identifier: post_view.author.handle.as_str().to_string(),
        author_display_name: post_view.author.display_name.clone(),
        created_at_millis,
        content_text,
    })
}

/// Render Bluesky post `text` plus its richtext `facets` into inline HTML.
///
/// Facets are byte-range annotations over the UTF-8 `text`; each is wrapped in an
/// `<a>` (link / mention / hashtag) and the surrounding plain text is
/// HTML-escaped. Newlines become `<br>`. Invalid or overlapping facets (bad
/// range, off a char boundary) are skipped so the output stays well-formed.
fn render_facets_to_html(text: &str, facets: &[Facet]) -> String {
    // Collect each facet's byte range and first resolvable feature, dropping any
    // facet whose range is out of bounds or not on a UTF-8 char boundary.
    let mut facet_spans: Vec<(usize, usize, &MainFeaturesItem)> = facets
        .iter()
        .filter_map(|facet| {
            let byte_start = facet.index.byte_start;
            let byte_end = facet.index.byte_end;
            let feature = facet.features.iter().find_map(|feature_union| match feature_union {
                Union::Refs(feature) => Some(feature),
                Union::Unknown(_) => None,
            })?;
            let in_bounds = byte_start < byte_end && byte_end <= text.len() && text.is_char_boundary(byte_start) && text.is_char_boundary(byte_end);
            in_bounds.then_some((byte_start, byte_end, feature))
        })
        .collect();
    facet_spans.sort_by_key(|(byte_start, _, _)| *byte_start);

    let mut html = String::new();
    let mut cursor = 0usize;
    for (byte_start, byte_end, feature) in facet_spans {
        // Skip a facet that overlaps the previous one.
        if byte_start < cursor {
            continue;
        }
        html.push_str(&escape_html_text(&text[cursor..byte_start]));
        let segment_html = escape_html_text(&text[byte_start..byte_end]);
        match feature {
            MainFeaturesItem::Link(link) => html.push_str(&format!("<a href=\"{}\">{segment_html}</a>", escape_html_attribute(&link.uri))),
            MainFeaturesItem::Mention(mention) => html.push_str(&format!("<a href=\"https://bsky.app/profile/{}\">{segment_html}</a>", escape_html_attribute(mention.did.as_str()))),
            MainFeaturesItem::Tag(tag) => html.push_str(&format!("<a href=\"https://bsky.app/hashtag/{}\">{segment_html}</a>", escape_html_attribute(&tag.tag))),
        }
        cursor = byte_end;
    }
    html.push_str(&escape_html_text(&text[cursor..]));
    html.replace('\n', "<br>")
}

/// HTML-escape text content (`&`, `<`, `>`).
fn escape_html_text(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// HTML-escape an attribute value (text escapes plus `"`).
fn escape_html_attribute(text: &str) -> String {
    escape_html_text(text).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facets_from_json(json: &str) -> Vec<Facet> {
        serde_json::from_str(json).expect("valid facets json")
    }

    #[test]
    fn renders_link_facet_as_anchor() {
        let text = "see example here";
        let facets = facets_from_json(r#"[{"index":{"byteStart":4,"byteEnd":11},"features":[{"$type":"app.bsky.richtext.facet#link","uri":"https://example.com"}]}]"#);
        assert_eq!(render_facets_to_html(text, &facets), "see <a href=\"https://example.com\">example</a> here");
    }

    #[test]
    fn renders_mention_facet_as_profile_link() {
        let text = "@jay says hi";
        let facets = facets_from_json(r#"[{"index":{"byteStart":0,"byteEnd":4},"features":[{"$type":"app.bsky.richtext.facet#mention","did":"did:plc:z72i7hdynmk6r22z27h6tvur"}]}]"#);
        assert_eq!(render_facets_to_html(text, &facets), "<a href=\"https://bsky.app/profile/did:plc:z72i7hdynmk6r22z27h6tvur\">@jay</a> says hi");
    }

    #[test]
    fn renders_tag_facet_as_hashtag_link() {
        let text = "#rust rocks";
        let facets = facets_from_json(r#"[{"index":{"byteStart":0,"byteEnd":5},"features":[{"$type":"app.bsky.richtext.facet#tag","tag":"rust"}]}]"#);
        assert_eq!(render_facets_to_html(text, &facets), "<a href=\"https://bsky.app/hashtag/rust\">#rust</a> rocks");
    }

    #[test]
    fn escapes_html_special_chars_in_plain_text() {
        assert_eq!(render_facets_to_html("a < b & c > d", &[]), "a &lt; b &amp; c &gt; d");
    }

    #[test]
    fn converts_newlines_to_break_tags() {
        assert_eq!(render_facets_to_html("line1\nline2", &[]), "line1<br>line2");
    }

    #[test]
    fn skips_out_of_bounds_facet_without_panicking() {
        let text = "short";
        let facets = facets_from_json(r#"[{"index":{"byteStart":0,"byteEnd":99},"features":[{"$type":"app.bsky.richtext.facet#link","uri":"https://example.com"}]}]"#);
        assert_eq!(render_facets_to_html(text, &facets), "short");
    }

    #[test]
    fn handles_multibyte_text_offsets() {
        // "🎉" is 4 bytes, then a space, so "example" starts at byte 5.
        let text = "🎉 example";
        let facets = facets_from_json(r#"[{"index":{"byteStart":5,"byteEnd":12},"features":[{"$type":"app.bsky.richtext.facet#link","uri":"https://example.com"}]}]"#);
        assert_eq!(render_facets_to_html(text, &facets), "🎉 <a href=\"https://example.com\">example</a>");
    }

    #[test]
    fn skips_facet_starting_off_a_char_boundary() {
        // Byte 1 is inside the 4-byte "🎉", so the facet is dropped.
        let text = "🎉ab";
        let facets = facets_from_json(r#"[{"index":{"byteStart":1,"byteEnd":3},"features":[{"$type":"app.bsky.richtext.facet#link","uri":"https://example.com"}]}]"#);
        assert_eq!(render_facets_to_html(text, &facets), "🎉ab");
    }
}
