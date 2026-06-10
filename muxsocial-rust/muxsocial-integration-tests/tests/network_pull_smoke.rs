//! Live smoke tests: pull a handful of recent posts from well-known identifiers
//! on each network and assert they map into well-formed `AggregatedPost`s.
//!
//! These hit the real networks, so they are slow and can be flaky if a relay /
//! instance / AppView is down. They live in the (opt-in) integration-tests
//! crate, not the fast default `cargo test`.
//!
//! Hashiverse has its own live test in `hashiverse_pull_smoke.rs` (it needs a
//! user id supplied via an env var).

use std::time::Duration;

use muxsocial_lib::http::default_http_transport;
use muxsocial_lib::post::SourceNetwork;
use muxsocial_lib::sources::{bluesky, mastodon, nostr};

/// jack's nostr public key (hex form, so it parses without nip19/bech32).
const NOSTR_JACK_PUBKEY_HEX: &str = "82341f882b6eabcd2ba7f1ef90aad961cf074af15b9ef44a09f9d2a8fbfbe6a2";

#[tokio::test]
async fn pulls_recent_text_notes_from_nostr() {
    let posts = nostr::fetch_recent_posts(NOSTR_JACK_PUBKEY_HEX, nostr::DEFAULT_RELAYS, 5, Duration::from_secs(20)).await.expect("nostr fetch should succeed");

    assert!(!posts.is_empty(), "expected at least one nostr note for a well-known author");
    for post in &posts {
        assert_eq!(post.source, SourceNetwork::Nostr);
        assert_eq!(post.author_identifier, NOSTR_JACK_PUBKEY_HEX, "every note should be from the requested author");
        assert!(!post.source_post_id.is_empty(), "note should have an event id");
        assert!(post.created_at_millis > 0, "note should have a timestamp");
    }
}

#[tokio::test]
async fn pulls_recent_posts_from_bluesky() {
    let http_transport = default_http_transport();
    let posts = bluesky::fetch_recent_posts(&http_transport, "bsky.app", 5).await.expect("bluesky fetch should succeed");

    assert!(!posts.is_empty(), "expected at least one Bluesky post for @bsky.app");
    for post in &posts {
        assert_eq!(post.source, SourceNetwork::Bluesky);
        assert!(post.source_post_id.starts_with("at://"), "post id should be an AT-URI, got {:?}", post.source_post_id);
        assert!(!post.author_identifier.is_empty(), "post should have an author handle");
    }
}

#[tokio::test]
async fn pulls_recent_statuses_from_mastodon() {
    let http_transport = default_http_transport();
    let posts = mastodon::fetch_recent_posts(&http_transport, "@Gargron@mastodon.social", 5).await.expect("mastodon fetch should succeed");

    assert!(!posts.is_empty(), "expected at least one Mastodon status for @Gargron");
    for post in &posts {
        assert_eq!(post.source, SourceNetwork::Mastodon);
        assert!(!post.source_post_id.is_empty(), "status should have an id");
        assert!(post.created_at_millis > 0, "status should have a timestamp");
    }
}
