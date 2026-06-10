//! Hashiverse source client — built on `hashiverse-lib`.
//!
//! Hashiverse is a DHT + proof-of-work network. Constructing a client requires
//! injected runtime services (transport, PoW, storage, key locker); the
//! ergonomic native defaults live in the `hashiverse-client-rust` crate, which
//! is native-only (sqlite, on-disk key locker). To keep `muxsocial-lib`
//! wasm-clean we depend only on `hashiverse-lib` here and take an
//! already-constructed [`HashiverseClient`] (the caller — the native integration
//! test, or later the wasm client — owns construction).
//!
//! Reading a user's timeline is one call: `single_timeline_get_more` over the
//! user's [`BucketType::User`] bucket. That call is *itself* a stateful
//! backward walk from "now" that returns the next batch of unseen posts
//! (newest-first, deduped) — exactly the latest-then-earlier behaviour we want
//! — so [`HashiversePager`] just wraps it rather than reimplementing paging.

use std::sync::Arc;

use anyhow::Context;
use hashiverse_lib::client::hashiverse_client::HashiverseClient;
use hashiverse_lib::protocol::posting::encoded_post::EncodedPostV1;
use hashiverse_lib::tools::buckets::BucketType;
use hashiverse_lib::tools::types::Id;

/// Re-export so callers (tests) can name the client type without depending on
/// `hashiverse-lib` directly.
pub use hashiverse_lib::client::hashiverse_client::HashiverseClient as Client;

use crate::post::{AggregatedPost, SourceNetwork};
use crate::timeline::SourcePager;

/// Fetch a page of recent posts for the Hashiverse user identified by
/// `user_id_hex` (a 32-byte id as 64 hex chars), using an already-constructed
/// [`HashiverseClient`].
pub async fn fetch_recent_posts(hashiverse_client: &HashiverseClient, user_id_hex: &str) -> anyhow::Result<Vec<AggregatedPost>> {
    timeline_get_more(hashiverse_client, user_id_hex).await
}

/// One step of hashiverse's own `SingleTimeline`: the next batch of unseen
/// posts for the user, mapped into [`AggregatedPost`].
async fn timeline_get_more(hashiverse_client: &HashiverseClient, user_id_hex: &str) -> anyhow::Result<Vec<AggregatedPost>> {
    let user_id = Id::from_hex_str(user_id_hex).with_context(|| format!("parsing hashiverse user id {user_id_hex:?}"))?;
    log::info!("hashiverse: reading user timeline for {user_id_hex}");

    let (timeline_posts, _oldest_processed_time_millis) = hashiverse_client.single_timeline_get_more(BucketType::User, &user_id).await.context("reading hashiverse user timeline")?;
    log::debug!("hashiverse: read {} timeline post(s) for {user_id_hex}", timeline_posts.len());

    Ok(timeline_posts.into_iter().map(|(_bucket_location, encoded_post, _body_bytes, _was_healed)| map_encoded_post(encoded_post)).collect())
}

/// A timeline pager for a single Hashiverse user — a thin wrapper over
/// hashiverse's `SingleTimeline`. Both directions forward to
/// `single_timeline_get_more`, which already returns the next latest-then-earlier
/// deduped batch; since [`crate::timeline::SourceTimeline`] tries `fetch_newer`
/// first and returns when it is non-empty, this is effectively one
/// `single_timeline_get_more` per `get_more`.
pub struct HashiversePager {
    client: Arc<HashiverseClient>,
    user_id_hex: String,
}

impl HashiversePager {
    /// Construct from a shared client. `hashiverse-client-rust`'s `Hashiverse`
    /// wrapper exposes one via `.client().clone()`.
    pub fn new(client: Arc<HashiverseClient>, user_id_hex: impl Into<String>) -> Self {
        Self { client, user_id_hex: user_id_hex.into() }
    }
}

impl SourcePager for HashiversePager {
    async fn fetch_newer(&mut self, _newest_known: Option<&AggregatedPost>, _limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        timeline_get_more(&self.client, &self.user_id_hex).await
    }

    async fn fetch_older(&mut self, _oldest_known: Option<&AggregatedPost>, _limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        timeline_get_more(&self.client, &self.user_id_hex).await
    }

    async fn reset(&mut self) -> anyhow::Result<()> {
        self.client.single_timeline_reset().await.context("resetting hashiverse timeline")
    }
}

fn map_encoded_post(encoded_post: EncodedPostV1) -> AggregatedPost {
    AggregatedPost {
        source: SourceNetwork::Hashiverse,
        source_post_id: encoded_post.post_id.to_hex_str(),
        author_identifier: hex::encode(encoded_post.header.verification_key_bytes.0),
        author_display_name: None,
        created_at_millis: encoded_post.header.time_millis.0,
        // Hashiverse post bodies are HTML.
        content_html: encoded_post.post,
    }
}
