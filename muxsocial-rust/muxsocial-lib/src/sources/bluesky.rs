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

use crate::http::{DefaultHttpTransport, HttpRequest, HttpTransport};
use crate::post::{AggregatedPost, SourceNetwork, parse_rfc3339_to_epoch_millis};
use crate::timeline::SourcePager;

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
    let (posts, _next_cursor) = fetch_feed_page(http_transport, actor, limit, None).await?;
    Ok(posts)
}

/// Fetch one page of an author's feed, returning the mapped posts plus the
/// opaque `cursor` for the *next* (older) page (None when the feed is
/// exhausted). `cursor` paginates older; `None` starts at the top.
async fn fetch_feed_page(http_transport: &(impl HttpTransport + Sync), actor: &str, limit: u8, cursor: Option<String>) -> anyhow::Result<(Vec<AggregatedPost>, Option<String>)> {
    let xrpc_client = TransportXrpcClient {
        base_uri: PUBLIC_APPVIEW_BASE_URL.to_string(),
        http_transport,
    };

    log::info!("bluesky: fetching up to {limit} posts for actor {actor:?} (cursor={cursor:?})");

    let actor_identifier: AtIdentifier = actor.parse().map_err(|parse_error| anyhow!("invalid Bluesky actor {actor:?}: {parse_error}"))?;
    let parameters = get_author_feed::ParametersData {
        actor: actor_identifier,
        cursor,
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

    log::debug!("bluesky: received {} feed item(s) for actor {actor:?}", author_feed.feed.len());

    let next_cursor = author_feed.cursor.clone();
    let posts = author_feed.feed.iter().map(map_feed_view_post).collect::<anyhow::Result<Vec<AggregatedPost>>>()?;
    Ok((posts, next_cursor))
}

/// A timeline pager for a single Bluesky actor.
///
/// Bluesky paginates older via an opaque `cursor` returned by each page; there
/// is no "since" query, so newer = re-fetch the top page and let the timeline's
/// dedupe surface anything new. The pager remembers the cursor for the next
/// older page so backward paging is preserved across the interleaved
/// newer/older calls.
pub struct BlueskyPager {
    http_transport: DefaultHttpTransport,
    actor: String,
    /// Cursor for the next older page, advanced only by older fetches.
    next_older_cursor: Option<String>,
    /// Whether backward paging has been seeded yet.
    older_started: bool,
}

impl BlueskyPager {
    pub fn new(http_transport: DefaultHttpTransport, actor: impl Into<String>) -> Self {
        Self {
            http_transport,
            actor: actor.into(),
            next_older_cursor: None,
            older_started: false,
        }
    }
}

impl SourcePager for BlueskyPager {
    async fn fetch_newer(&mut self, _newest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        let limit_u8 = limit.clamp(1, u8::MAX as usize) as u8;
        let (posts, next_cursor) = fetch_feed_page(&self.http_transport, &self.actor, limit_u8, None).await?;
        // Seed backward paging right after the top page on the first fetch only,
        // so later older fetches continue beyond the top rather than restarting.
        if !self.older_started {
            self.next_older_cursor = next_cursor;
            self.older_started = true;
        }
        Ok(posts)
    }

    async fn fetch_older(&mut self, _oldest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        let limit_u8 = limit.clamp(1, u8::MAX as usize) as u8;
        if !self.older_started {
            let (posts, next_cursor) = fetch_feed_page(&self.http_transport, &self.actor, limit_u8, None).await?;
            self.next_older_cursor = next_cursor;
            self.older_started = true;
            return Ok(posts);
        }
        match self.next_older_cursor.clone() {
            Some(cursor) => {
                let (posts, next_cursor) = fetch_feed_page(&self.http_transport, &self.actor, limit_u8, Some(cursor)).await?;
                self.next_older_cursor = next_cursor;
                Ok(posts)
            }
            // No further cursor — the feed is exhausted.
            None => Ok(Vec::new()),
        }
    }

    async fn reset(&mut self) -> anyhow::Result<()> {
        self.next_older_cursor = None;
        self.older_started = false;
        Ok(())
    }
}

fn map_feed_view_post(feed_view_post: &atrium_api::app::bsky::feed::defs::FeedViewPost) -> anyhow::Result<AggregatedPost> {
    let post_view = &feed_view_post.post;

    // The post record is loosely typed; recover the typed post Record so we can
    // turn its text + richtext facets into inline HTML. If it isn't a post
    // record we can decode, fall back to empty content rather than failing.
    let post_record = post::RecordData::try_from_unknown(post_view.record.clone());
    let (content_html, record_created_at_millis) = match &post_record {
        Ok(record) => {
            let rendered_html = render_facets_to_html(&record.text, record.facets.as_deref().unwrap_or(&[]));
            (rendered_html, parse_rfc3339_to_epoch_millis(record.created_at.as_str()).ok())
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
        content_html,
        // Permalink from the stable DID + the AT-URI's rkey; the handle can change.
        post_url: bluesky_post_url(&post_view.uri, post_view.author.did.as_str()),
    })
}

/// Build the bsky.app permalink for a post from its AT-URI
/// (`at://did/app.bsky.feed.post/{rkey}`) and the author DID:
/// `https://bsky.app/profile/{did}/post/{rkey}`. `None` if the AT-URI has no rkey.
fn bluesky_post_url(at_uri: &str, did: &str) -> Option<String> {
    let rkey = at_uri.rsplit('/').next().filter(|rkey| !rkey.is_empty())?;
    Some(format!("https://bsky.app/profile/{did}/post/{rkey}"))
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
        html.push_str(&crate::html::escape_text(&text[cursor..byte_start]));
        let segment_html = crate::html::escape_text(&text[byte_start..byte_end]);
        match feature {
            MainFeaturesItem::Link(link) => html.push_str(&format!("<a href=\"{}\">{segment_html}</a>", crate::html::escape_attribute(&link.uri))),
            MainFeaturesItem::Mention(mention) => html.push_str(&format!("<a href=\"https://bsky.app/profile/{}\">{segment_html}</a>", crate::html::escape_attribute(mention.did.as_str()))),
            MainFeaturesItem::Tag(tag) => html.push_str(&format!("<a href=\"https://bsky.app/hashtag/{}\">{segment_html}</a>", crate::html::escape_attribute(&tag.tag))),
        }
        cursor = byte_end;
    }
    html.push_str(&crate::html::escape_text(&text[cursor..]));
    html.replace('\n', "<br>")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facets_from_json(json: &str) -> Vec<Facet> {
        serde_json::from_str(json).expect("valid facets json")
    }

    #[test]
    fn builds_bsky_app_url_from_at_uri_and_did() {
        let url = bluesky_post_url("at://did:plc:abc123/app.bsky.feed.post/3kxyz", "did:plc:abc123");
        assert_eq!(url.as_deref(), Some("https://bsky.app/profile/did:plc:abc123/post/3kxyz"));
    }

    #[test]
    fn bluesky_post_url_is_none_without_an_rkey() {
        assert_eq!(bluesky_post_url("at://did:plc:abc123/", "did:plc:abc123"), None);
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
