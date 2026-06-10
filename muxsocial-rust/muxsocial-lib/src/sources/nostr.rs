//! nostr source client — built on `rust-nostr` (`nostr-sdk`).
//!
//! nostr uses its own WebSocket relay transport (native and wasm both handled
//! by `nostr-sdk`), so it does not go through [`crate::http`]. We connect to a
//! few public relays, request recent kind-1 (text note) events for an author,
//! and map them into [`AggregatedPost`]. Note content is plain text, so we wrap
//! it into HTML (escape + newlines → `<br>`) to match the other sources; no URL
//! linkification is done.
//!
//! Paging is by timestamp: [`NostrPager`] uses the `Filter` `.since`/`.until`
//! bounds (whole seconds) to fetch newer/older than the posts already held.

use std::time::Duration;

use anyhow::Context;
use nostr_sdk::prelude::*;

use crate::post::{AggregatedPost, SourceNetwork};
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
    AggregatedPost {
        source: SourceNetwork::Nostr,
        source_post_id: event.id.to_hex(),
        author_identifier: event.pubkey.to_hex(),
        author_display_name: None,
        // nostr timestamps are whole seconds since the epoch.
        created_at_millis: event.created_at.as_secs() as i64 * 1000,
        // nostr note content is plain text; wrap it into HTML like the other sources.
        content_html: crate::html::plain_text_to_html(&event.content),
        post_url,
    }
}

/// Build the njump permalink for an event: an `nevent` (bech32) carrying the event
/// id, author pubkey, and up to two relay hints (the relays we fetched from), so
/// it resolves on the right relay. `None` if bech32 encoding fails.
fn njump_event_url(event: &Event, relay_hints: &[String]) -> Option<String> {
    let relay_urls: Vec<RelayUrl> = relay_hints.iter().take(2).filter_map(|relay_url| RelayUrl::parse(relay_url).ok()).collect();
    let nip19_event = Nip19Event::new(event.id).author(event.pubkey).relays(relay_urls);
    nip19_event.to_bech32().ok().map(|nevent| format!("https://njump.me/{nevent}"))
}
