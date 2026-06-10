//! nostr source client — built on `rust-nostr` (`nostr-sdk`).
//!
//! nostr uses its own WebSocket relay transport (native and wasm both handled
//! by `nostr-sdk`), so it does not go through [`crate::http`]. We connect to a
//! few public relays, request recent kind-1 (text note) events for an author,
//! and map them into [`AggregatedPost`].

use std::time::Duration;

use anyhow::Context;
use nostr_sdk::prelude::*;

use crate::post::{AggregatedPost, SourceNetwork};

/// Default public relays used when a caller does not supply their own.
pub const DEFAULT_RELAYS: &[&str] = &["wss://relay.damus.io", "wss://nos.lol"];

/// Fetch up to `limit` recent text notes (kind 1) authored by
/// `public_key_hex_or_bech32` from `relays`, waiting at most `timeout`.
///
/// The key may be hex or `npub` bech32 (bech32 requires nostr's nip19 support).
pub async fn fetch_recent_posts(public_key_hex_or_bech32: &str, relays: &[&str], limit: usize, timeout: Duration) -> anyhow::Result<Vec<AggregatedPost>> {
    #[cfg(not(target_arch = "wasm32"))]
    ensure_default_crypto_provider();

    let author_public_key = PublicKey::parse(public_key_hex_or_bech32).with_context(|| format!("parsing nostr public key {public_key_hex_or_bech32:?}"))?;

    let client = Client::default();
    for relay_url in relays {
        client.add_relay(*relay_url).await.with_context(|| format!("adding nostr relay {relay_url}"))?;
    }
    client.connect().await;

    log::info!("nostr: fetching up to {limit} text notes for {public_key_hex_or_bech32} from {} relay(s)", relays.len());

    let filter = Filter::new().author(author_public_key).kind(Kind::TextNote).limit(limit);
    let events = client.fetch_events(filter, timeout).await.context("fetching nostr events")?;
    log::debug!("nostr: fetched {} event(s)", events.len());

    let mut posts: Vec<AggregatedPost> = events.into_iter().map(map_event).collect();
    // Newest first.
    posts.sort_by_key(|post| std::cmp::Reverse(post.created_at_millis));
    Ok(posts)
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
        content_text: event.content.clone(),
    }
}
