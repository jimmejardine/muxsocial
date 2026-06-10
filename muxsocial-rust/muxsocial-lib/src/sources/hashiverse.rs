//! Hashiverse source client — built on `hashiverse-lib`.
//!
//! Hashiverse is a DHT + proof-of-work network. Constructing a client requires
//! injected runtime services (transport, PoW, storage, key locker); the
//! ergonomic native defaults live in the `hashiverse-client-rust` crate, which
//! is native-only (sqlite, on-disk key locker). To keep `muxsocial-lib`
//! wasm-clean we depend only on `hashiverse-lib` here and take an
//! already-constructed [`HashiverseClient`] by reference — the caller (the
//! native integration test, or later the wasm client) owns construction.
//!
//! Reading a user's timeline is one call: `single_timeline_get_more` over the
//! user's [`BucketType::User`] bucket.

use anyhow::Context;
use hashiverse_lib::client::hashiverse_client::HashiverseClient;
use hashiverse_lib::protocol::posting::encoded_post::EncodedPostV1;
use hashiverse_lib::tools::buckets::BucketType;
use hashiverse_lib::tools::types::Id;

/// Re-export so callers (tests) can name the client type without depending on
/// `hashiverse-lib` directly.
pub use hashiverse_lib::client::hashiverse_client::HashiverseClient as Client;

use crate::post::{AggregatedPost, SourceNetwork};

/// Fetch a page of recent posts for the Hashiverse user identified by
/// `user_id_hex` (a 32-byte id as 64 hex chars), using an already-constructed
/// [`HashiverseClient`].
pub async fn fetch_recent_posts(hashiverse_client: &HashiverseClient, user_id_hex: &str) -> anyhow::Result<Vec<AggregatedPost>> {
    let user_id = Id::from_hex_str(user_id_hex).with_context(|| format!("parsing hashiverse user id {user_id_hex:?}"))?;

    let (timeline_posts, _oldest_processed_time_millis) = hashiverse_client.single_timeline_get_more(BucketType::User, &user_id).await.context("reading hashiverse user timeline")?;

    Ok(timeline_posts.into_iter().map(|(_bucket_location, encoded_post, _body_bytes, _was_healed)| map_encoded_post(encoded_post)).collect())
}

fn map_encoded_post(encoded_post: EncodedPostV1) -> AggregatedPost {
    AggregatedPost {
        source: SourceNetwork::Hashiverse,
        source_post_id: encoded_post.post_id.to_hex_str(),
        author_identifier: hex::encode(encoded_post.header.verification_key_bytes.0),
        author_display_name: None,
        created_at_millis: encoded_post.header.time_millis.0,
        // Hashiverse post bodies are HTML.
        content_text: encoded_post.post,
    }
}
