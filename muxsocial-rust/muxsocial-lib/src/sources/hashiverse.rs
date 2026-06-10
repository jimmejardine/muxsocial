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
use hashiverse_lib::tools::buckets::{BucketLocation, BucketType};
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

/// Build a read-only **guest** [`HashiverseClient`] for the browser, wiring
/// hashiverse-lib's own wasm runtime services (IndexedDB key locker + storage,
/// fetch transport) with the single-threaded PoW generator — reading a timeline
/// needs no PoW workers. Guest mode is the empty keyphrase. wasm32 only; native
/// callers inject a client built by `hashiverse-client-rust` instead.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub async fn build_guest_client() -> anyhow::Result<Arc<HashiverseClient>> {
    use hashiverse_lib::client::args::Args;
    use hashiverse_lib::client::client_storage::client_storage::ClientStorage;
    use hashiverse_lib::client::client_storage::wasm_client_storage::WasmClientStorage;
    use hashiverse_lib::client::key_locker::key_locker::{KeyLocker, KeyLockerManager};
    use hashiverse_lib::client::key_locker::wasm_key_locker::WasmKeyLockerManager;
    use hashiverse_lib::tools::pow_generator::pow_generator::PowGenerator;
    use hashiverse_lib::tools::pow_generator::single_threaded_pow_generator::SingleThreadedPowGenerator;
    use hashiverse_lib::tools::runtime_services::RuntimeServices;
    use hashiverse_lib::tools::time_provider::time_provider::{RealTimeProvider, TimeProvider};
    use hashiverse_lib::transport::transport::TransportFactory;
    use hashiverse_lib::transport::wasm_transport::WasmTransportFactory;

    let key_locker_manager = WasmKeyLockerManager::new().await?;
    // Empty keyphrase = the deterministic read-only guest identity.
    let key_locker: Arc<dyn KeyLocker> = key_locker_manager.create(String::new()).await?;
    let client_storage: Arc<dyn ClientStorage> = WasmClientStorage::new().await?;
    let transport_factory: Arc<dyn TransportFactory> = Arc::new(WasmTransportFactory::default());
    let time_provider: Arc<dyn TimeProvider> = Arc::new(RealTimeProvider::default());
    let pow_generator: Arc<dyn PowGenerator> = Arc::new(SingleThreadedPowGenerator::new());
    let runtime_services = Arc::new(RuntimeServices { time_provider, transport_factory, pow_generator });
    let hashiverse_client = HashiverseClient::new(runtime_services, client_storage, key_locker, Args::new()).await?;
    Ok(Arc::new(hashiverse_client))
}

/// One step of hashiverse's own `SingleTimeline`: the next batch of unseen
/// posts for the user, mapped into [`AggregatedPost`].
async fn timeline_get_more(hashiverse_client: &HashiverseClient, user_id_hex: &str) -> anyhow::Result<Vec<AggregatedPost>> {
    let user_id = Id::from_hex_str(user_id_hex).with_context(|| format!("parsing hashiverse user id {user_id_hex:?}"))?;
    log::info!("hashiverse: reading user timeline for {user_id_hex}");

    let (timeline_posts, _oldest_processed_time_millis) = hashiverse_client.single_timeline_get_more(BucketType::User, &user_id).await.context("reading hashiverse user timeline")?;
    log::debug!("hashiverse: read {} timeline post(s) for {user_id_hex}", timeline_posts.len());

    Ok(timeline_posts.into_iter().map(|(bucket_location, encoded_post, _body_bytes, _was_healed)| map_encoded_post(encoded_post, &bucket_location)).collect())
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

/// Base of the Hashiverse web app's single-post route (hash-routed):
/// `{base}/#/post/{post_id}/{bucket_location}`.
const HASHIVERSE_POST_URL_BASE: &str = "https://app.hashiverse.com";

fn map_encoded_post(encoded_post: EncodedPostV1, bucket_location: &BucketLocation) -> AggregatedPost {
    let post_id_hex = encoded_post.post_id.to_hex_str();
    let post_url = format!(
        "{HASHIVERSE_POST_URL_BASE}/#/post/{}/{}",
        percent_encode_component(&post_id_hex),
        percent_encode_component(&bucket_location.to_html_attr()),
    );
    AggregatedPost {
        source: SourceNetwork::Hashiverse,
        source_post_id: post_id_hex,
        author_identifier: hex::encode(encoded_post.header.verification_key_bytes.0),
        author_display_name: None,
        created_at_millis: encoded_post.header.time_millis.0,
        // Hashiverse post bodies are HTML.
        content_html: encoded_post.post,
        post_url: Some(post_url),
    }
}

/// Percent-encode one URL path segment like JS `encodeURIComponent`: everything
/// except the RFC3986 unreserved set (`A-Z a-z 0-9 - _ . ~`) is escaped. Used for
/// the Hashiverse permalink segments, matching the hashiverse web client.
fn percent_encode_component(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => encoded.push(byte as char),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encodes_reserved_chars_only() {
        // Unreserved chars pass through; reserved ones (':', '/', '#') are escaped.
        assert_eq!(percent_encode_component("abc-123_.~"), "abc-123_.~");
        assert_eq!(percent_encode_component("a:b/c#d"), "a%3Ab%2Fc%23d");
    }
}
