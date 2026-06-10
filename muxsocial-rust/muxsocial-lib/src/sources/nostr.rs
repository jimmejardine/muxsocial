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

/// Default public relays used when a caller does not supply their own.
pub const DEFAULT_RELAYS: &[&str] = &["wss://relay.damus.io", "wss://nos.lol"];

/// Fetch up to `limit` recent text notes (kind 1) authored by
/// `public_key_hex_or_bech32` from `relays`, waiting at most `timeout`.
///
/// The key may be hex or `npub` bech32 (bech32 requires nostr's nip19 support).
pub async fn fetch_recent_posts(public_key_hex_or_bech32: &str, relays: &[&str], limit: usize, timeout: Duration) -> anyhow::Result<Vec<AggregatedPost>> {
    fetch_text_notes(public_key_hex_or_bech32, relays, limit, timeout, None, None).await
}

/// Fetch text notes for an author, optionally bounded by `since_secs` (only
/// newer) and/or `until_secs` (only older), both in whole epoch seconds.
async fn fetch_text_notes(public_key_hex_or_bech32: &str, relays: &[&str], limit: usize, timeout: Duration, since_secs: Option<u64>, until_secs: Option<u64>) -> anyhow::Result<Vec<AggregatedPost>> {
    #[cfg(not(target_arch = "wasm32"))]
    ensure_default_crypto_provider();

    let author_public_key = PublicKey::parse(public_key_hex_or_bech32).with_context(|| format!("parsing nostr public key {public_key_hex_or_bech32:?}"))?;

    let client = Client::default();
    for relay_url in relays {
        client.add_relay(*relay_url).await.with_context(|| format!("adding nostr relay {relay_url}"))?;
    }
    client.connect().await;

    log::info!("nostr: fetching up to {limit} text notes for {public_key_hex_or_bech32} (since={since_secs:?}, until={until_secs:?}) from {} relay(s)", relays.len());

    let mut filter = Filter::new().author(author_public_key).kind(Kind::TextNote).limit(limit);
    if let Some(since_secs) = since_secs {
        filter = filter.since(Timestamp::from(since_secs));
    }
    if let Some(until_secs) = until_secs {
        filter = filter.until(Timestamp::from(until_secs));
    }

    let events = client.fetch_events(filter, timeout).await.context("fetching nostr events")?;
    log::debug!("nostr: fetched {} event(s)", events.len());

    let mut posts: Vec<AggregatedPost> = events.into_iter().map(map_event).collect();
    // Newest first.
    posts.sort_by_key(|post| std::cmp::Reverse(post.created_at_millis));
    Ok(posts)
}

/// A timeline pager for a single nostr author. Paginates by event timestamp.
pub struct NostrPager {
    public_key_hex_or_bech32: String,
    relays: Vec<String>,
    timeout: Duration,
}

impl NostrPager {
    pub fn new(public_key_hex_or_bech32: impl Into<String>, relays: &[&str], timeout: Duration) -> Self {
        Self {
            public_key_hex_or_bech32: public_key_hex_or_bech32.into(),
            relays: relays.iter().map(|relay| relay.to_string()).collect(),
            timeout,
        }
    }

    async fn fetch_bounded(&self, limit: usize, since_secs: Option<u64>, until_secs: Option<u64>) -> anyhow::Result<Vec<AggregatedPost>> {
        let relay_refs: Vec<&str> = self.relays.iter().map(|relay| relay.as_str()).collect();
        fetch_text_notes(&self.public_key_hex_or_bech32, &relay_refs, limit, self.timeout, since_secs, until_secs).await
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

fn map_event(event: Event) -> AggregatedPost {
    AggregatedPost {
        source: SourceNetwork::Nostr,
        source_post_id: event.id.to_hex(),
        author_identifier: event.pubkey.to_hex(),
        author_display_name: None,
        // nostr timestamps are whole seconds since the epoch.
        created_at_millis: event.created_at.as_secs() as i64 * 1000,
        // nostr note content is plain text; wrap it into HTML like the other sources.
        content_html: crate::html::plain_text_to_html(&event.content),
    }
}
