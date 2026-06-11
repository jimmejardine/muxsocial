//! nostr source client — built on `rust-nostr` (`nostr-sdk`).
//!
//! nostr uses its own WebSocket relay transport (native and wasm both handled
//! by `nostr-sdk`), so it does not go through [`crate::http`]. We connect to a
//! few public relays, request recent kind-1 (text note) events for an author,
//! and map them into [`AggregatedPost`]. Note content is plain text, so we wrap
//! it into HTML (escape + newlines → `<br>`) to match the other sources; image
//! URLs are pulled out as media and other URLs are linkified.
//!
//! Paging is by timestamp: [`NostrPager`] uses the `Filter` `.since`/`.until`
//! bounds (whole seconds) to fetch newer/older than the posts already held.

use std::time::Duration;

use anyhow::Context;
use nostr_sdk::prelude::*;

use crate::post::{AggregatedPost, PostMedia, SourceNetwork};
use crate::timeline::SourcePager;

/// The nostr SDK client type, re-exported so an owner can hold a shared,
/// pre-connected client and pass clones to multiple [`NostrPager`]s.
pub use nostr_sdk::Client;

/// Default public relays used when a caller does not supply their own.
pub const DEFAULT_RELAYS: &[&str] = &["wss://relay.damus.io", "wss://nos.lol"];

/// Build and connect a nostr client to `relays`. The returned [`Client`] is
/// `Arc`-backed (cheap to clone, shared relay pool) — build it once and hand
/// clones to every nostr source rather than reconnecting per fetch.
pub async fn connect_client(relays: &[&str]) -> anyhow::Result<Client> {
    #[cfg(not(target_arch = "wasm32"))]
    ensure_default_crypto_provider();

    let client = Client::default();
    for relay_url in relays {
        client.add_relay(*relay_url).await.with_context(|| format!("adding nostr relay {relay_url}"))?;
    }
    client.connect().await;
    Ok(client)
}

/// Fetch up to `limit` recent text notes (kind 1) authored by
/// `public_key_hex_or_bech32` from `relays`, waiting at most `timeout`. Builds a
/// transient client; for repeated paging use [`NostrPager`] with a shared client.
///
/// The key may be hex or `npub` bech32 (bech32 requires nostr's nip19 support).
pub async fn fetch_recent_posts(public_key_hex_or_bech32: &str, relays: &[&str], limit: usize, timeout: Duration) -> anyhow::Result<Vec<AggregatedPost>> {
    let client = connect_client(relays).await?;
    let relay_hints: Vec<String> = relays.iter().map(|relay_url| relay_url.to_string()).collect();
    fetch_with_client(&client, public_key_hex_or_bech32, limit, timeout, None, None, &relay_hints).await
}

/// Fetch text notes for an author over an already-connected client, optionally
/// bounded by `since_secs` (only newer) and/or `until_secs` (only older), both
/// in whole epoch seconds.
async fn fetch_with_client(client: &Client, public_key_hex_or_bech32: &str, limit: usize, timeout: Duration, since_secs: Option<u64>, until_secs: Option<u64>, relay_hints: &[String]) -> anyhow::Result<Vec<AggregatedPost>> {
    let author_public_key = PublicKey::parse(public_key_hex_or_bech32).with_context(|| format!("parsing nostr public key {public_key_hex_or_bech32:?}"))?;

    log::info!("nostr: fetching up to {limit} text notes for {public_key_hex_or_bech32} (since={since_secs:?}, until={until_secs:?})");

    let mut filter = Filter::new().author(author_public_key).kind(Kind::TextNote).limit(limit);
    if let Some(since_secs) = since_secs {
        filter = filter.since(Timestamp::from(since_secs));
    }
    if let Some(until_secs) = until_secs {
        filter = filter.until(Timestamp::from(until_secs));
    }

    let events = client.fetch_events(filter, timeout).await.context("fetching nostr events")?;
    log::debug!("nostr: fetched {} event(s)", events.len());

    let mut posts: Vec<AggregatedPost> = events.into_iter().map(|event| map_event(event, relay_hints)).collect();
    // Newest first.
    posts.sort_by_key(|post| std::cmp::Reverse(post.created_at_millis));
    Ok(posts)
}

/// A timeline pager for a single nostr author over a shared, pre-connected
/// [`Client`]. Paginates by event timestamp.
pub struct NostrPager {
    client: Client,
    public_key_hex_or_bech32: String,
    timeout: Duration,
    /// The relays this pager queries; embedded as hints in each post's `nevent`
    /// permalink so it resolves where we actually fetched it from.
    relays: Vec<String>,
}

impl NostrPager {
    /// `client` should be a clone of the shared, already-connected client (see
    /// [`connect_client`]); `relays` are the relay URLs it is connected to.
    pub fn new(client: Client, public_key_hex_or_bech32: impl Into<String>, timeout: Duration, relays: Vec<String>) -> Self {
        Self {
            client,
            public_key_hex_or_bech32: public_key_hex_or_bech32.into(),
            timeout,
            relays,
        }
    }

    async fn fetch_bounded(&self, limit: usize, since_secs: Option<u64>, until_secs: Option<u64>) -> anyhow::Result<Vec<AggregatedPost>> {
        fetch_with_client(&self.client, &self.public_key_hex_or_bech32, limit, self.timeout, since_secs, until_secs, &self.relays).await
    }
}

impl SourcePager for NostrPager {
    async fn fetch_newer(&mut self, newest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        // Strictly newer: one second past the newest we hold.
        let since_secs = newest_known.map(|post| (post.created_at_millis / 1000 + 1).max(0) as u64);
        self.fetch_bounded(limit, since_secs, None).await
    }

    async fn fetch_older(&mut self, oldest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        // Strictly older: one second before the oldest we hold.
        let until_secs = oldest_known.map(|post| (post.created_at_millis / 1000 - 1).max(0) as u64);
        self.fetch_bounded(limit, None, until_secs).await
    }

    async fn reset(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Install aws-lc-rs as the process-wide rustls crypto provider, once. The
/// nostr relay WebSocket TLS needs a provider selected, but reqwest and the
/// websocket stack pull in different providers, so rustls won't auto-select.
/// Idempotent: a no-op (and ignored error) if a provider is already installed.
#[cfg(not(target_arch = "wasm32"))]
fn ensure_default_crypto_provider() {
    use std::sync::Once;
    static INSTALL_ONCE: Once = Once::new();
    INSTALL_ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

fn map_event(event: Event, relay_hints: &[String]) -> AggregatedPost {
    let post_url = njump_event_url(&event, relay_hints);
    // nostr notes are plain text: escape + wrap, pull out image URLs as media, and
    // linkify the remaining URLs.
    let (content_html, media) = render_nostr_content(&event.content);
    AggregatedPost {
        source: SourceNetwork::Nostr,
        source_post_id: event.id.to_hex(),
        // Show the npub (bech32) form so it matches the source the user added (and its chip),
        // rather than the raw hex pubkey. Fall back to hex if bech32 encoding ever fails.
        author_identifier: event.pubkey.to_bech32().unwrap_or_else(|_| event.pubkey.to_hex()),
        author_display_name: None,
        // nostr timestamps are whole seconds since the epoch.
        created_at_millis: event.created_at.as_secs() as i64 * 1000,
        content_html,
        post_url,
        media,
    }
}

/// Image extensions whose URLs become inline images rather than links.
const NOSTR_IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "avif"];

/// Render nostr note text to inline HTML and extract image media. Text is escaped
/// with newlines → `<br>` (via [`crate::html::plain_text_to_html`]); an `http(s)`
/// URL ending in an image extension becomes a [`PostMedia::Image`] and is dropped
/// from the text, while any other URL is linkified (`<a href=…>`). The GUI's
/// sanitizer hardens links with `target`/`rel`.
fn render_nostr_content(text: &str) -> (String, Vec<PostMedia>) {
    let mut html = String::with_capacity(text.len());
    let mut media: Vec<PostMedia> = Vec::new();
    let mut rest = text;

    while let Some(start) = find_url_start(rest) {
        let (before, from_url) = rest.split_at(start);
        html.push_str(&crate::html::plain_text_to_html(before));

        let token_len = url_token_len(from_url);
        let (token, after) = from_url.split_at(token_len);
        // Trailing sentence punctuation is not part of the URL.
        let url = token.trim_end_matches(['.', ',', ')', ']', '!', '?', ';', ':']);
        let trimmed_tail = &token[url.len()..];

        if is_image_url(url) {
            media.push(PostMedia::Image { url: url.to_string(), alt: None });
        }
        else {
            html.push_str(&format!("<a href=\"{}\">{}</a>", crate::html::escape_attribute(url), crate::html::escape_text(url)));
        }
        html.push_str(&crate::html::plain_text_to_html(trimmed_tail));
        rest = after;
    }
    html.push_str(&crate::html::plain_text_to_html(rest));
    (html, media)
}

/// The earliest `http://` / `https://` start in `s`, if any.
fn find_url_start(s: &str) -> Option<usize> {
    match (s.find("http://"), s.find("https://")) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

/// Byte length of the URL token at the start of `s` — up to the first whitespace
/// or `<`/`>`/`"`.
fn url_token_len(s: &str) -> usize {
    s.char_indices().find(|(_, character)| character.is_whitespace() || matches!(character, '<' | '>' | '"')).map_or(s.len(), |(index, _)| index)
}

/// Whether `url`'s path (ignoring query/fragment) ends in an image extension.
fn is_image_url(url: &str) -> bool {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    NOSTR_IMAGE_EXTENSIONS.iter().any(|extension| path.ends_with(&format!(".{extension}")))
}

/// Build the njump permalink for an event: an `nevent` (bech32) carrying the event
/// id, author pubkey, and up to two relay hints (the relays we fetched from), so
/// it resolves on the right relay. `None` if bech32 encoding fails.
fn njump_event_url(event: &Event, relay_hints: &[String]) -> Option<String> {
    let relay_urls: Vec<RelayUrl> = relay_hints.iter().take(2).filter_map(|relay_url| RelayUrl::parse(relay_url).ok()).collect();
    let nip19_event = Nip19Event::new(event.id).author(event.pubkey).relays(relay_urls);
    nip19_event.to_bech32().ok().map(|nevent| format!("https://njump.me/{nevent}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::post::PostMedia;

    #[test]
    fn extracts_image_urls_as_media_and_drops_them_from_text() {
        let (html, media) = render_nostr_content("see https://example.com/cat.jpg now");
        assert_eq!(
            media,
            vec![PostMedia::Image {
                url: "https://example.com/cat.jpg".to_string(),
                alt: None
            }]
        );
        assert!(!html.contains("cat.jpg"), "image url should be removed from text: {html}");
        assert!(html.contains("see") && html.contains("now"));
    }

    #[test]
    fn linkifies_non_image_urls() {
        let (html, media) = render_nostr_content("look https://example.com here");
        assert!(media.is_empty());
        assert!(html.contains("<a href=\"https://example.com\">https://example.com</a>"), "{html}");
    }

    #[test]
    fn escapes_plain_text_and_keeps_newlines() {
        let (html, media) = render_nostr_content(
            "a < b
c",
        );
        assert!(media.is_empty());
        assert_eq!(html, "a &lt; b<br>c");
    }

    #[test]
    fn trailing_punctuation_is_not_part_of_the_url() {
        let (html, _) = render_nostr_content("(see https://example.com).");
        assert!(html.contains("<a href=\"https://example.com\">"), "{html}");
        assert!(html.contains(")."), "trailing punctuation kept as text: {html}");
    }
}
