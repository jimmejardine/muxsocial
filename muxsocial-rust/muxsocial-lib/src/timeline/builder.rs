//! Build live timelines ([`SourceTimeline`] / [`MultiTimeline`]) from a list of
//! [`Source`]s, sharing one set of singleton source clients.
//!
//! This is the reusable analogue of the integration harness's `HarnessTimelines`
//! (`muxsocial-integration-tests`): one HTTP transport (shared reqwest/fetch pool)
//! serves Bluesky + Mastodon; one connected nostr client (shared relay pool)
//! serves every nostr source; one guest-mode Hashiverse client serves Hashiverse.
//! The slow clients (nostr connect) are built lazily on first use.
//!
//! Hashiverse is special: constructing a [`HashiverseClient`] requires native-only
//! runtime services (sqlite, on-disk key locker) from `hashiverse-client-rust`,
//! which is not available under wasm32. So the client is not built here — a
//! caller that can build one (the native harness) injects it via
//! [`SharedSourceClients::with_hashiverse_client`]; without it, Hashiverse sources
//! produce a clear error rather than failing the whole timeline.

use std::sync::Arc;
use std::time::Duration;

use crate::http::{DefaultHttpTransport, default_http_transport};
use crate::post::SourceNetwork;
use crate::sources::bluesky::BlueskyPager;
use crate::sources::hashiverse::{Client as HashiverseClient, HashiversePager};
use crate::sources::mastodon::MastodonPager;
use crate::sources::nostr::{self, Client as NostrClient, NostrPager};
use crate::timeline::{MultiTimeline, NetworkPager, Source, SourceTimeline};

/// How long a nostr relay fetch waits before giving up.
const NOSTR_RELAY_TIMEOUT: Duration = Duration::from_secs(20);

/// Owns the shared, singleton source clients used to build timelines. Hand one of
/// these out per logical "client" (the wasm worker, the native harness) and reuse
/// it across every timeline so the relay/HTTP pools and pager cursors are shared.
pub struct SharedSourceClients {
    http_transport: DefaultHttpTransport,
    nostr_client: Option<NostrClient>,
    hashiverse_client: Option<Arc<HashiverseClient>>,
    nostr_relays: Vec<String>,
    nostr_timeout: Duration,
}

impl SharedSourceClients {
    /// A fresh set of clients: a ready HTTP transport, the default nostr relays,
    /// and no nostr/Hashiverse client yet (nostr connects lazily; Hashiverse must
    /// be injected via [`Self::with_hashiverse_client`]).
    pub fn new() -> Self {
        Self {
            http_transport: default_http_transport(),
            nostr_client: None,
            hashiverse_client: None,
            nostr_relays: nostr::DEFAULT_RELAYS.iter().map(|relay_url| relay_url.to_string()).collect(),
            nostr_timeout: NOSTR_RELAY_TIMEOUT,
        }
    }

    /// Inject a pre-built shared Hashiverse client so Hashiverse sources can be
    /// paged. Without this, building a Hashiverse source errors.
    pub fn with_hashiverse_client(mut self, hashiverse_client: Arc<HashiverseClient>) -> Self {
        self.hashiverse_client = Some(hashiverse_client);
        self
    }

    /// Replace the nostr relay list. Drops the cached client so the next read
    /// reconnects to the new relays (callers should also drop live pagers).
    pub fn set_relays(&mut self, relays: Vec<String>) {
        self.nostr_relays = relays;
        self.nostr_client = None;
    }

    /// The current nostr relay list.
    pub fn relays(&self) -> &[String] {
        &self.nostr_relays
    }

    /// Connect the shared nostr client once; hand out cheap clones (shared pool).
    async fn shared_nostr_client(&mut self) -> anyhow::Result<NostrClient> {
        if self.nostr_client.is_none() {
            let relay_urls: Vec<&str> = self.nostr_relays.iter().map(|relay_url| relay_url.as_str()).collect();
            self.nostr_client = Some(nostr::connect_client(&relay_urls).await?);
        }
        Ok(self.nostr_client.as_ref().expect("nostr client just connected").clone())
    }

    /// Build a single-source timeline for `source`, acquiring/sharing the right
    /// client for its network.
    pub async fn build_source_timeline(&mut self, source: &Source) -> anyhow::Result<SourceTimeline<NetworkPager>> {
        let pager = match source.network {
            SourceNetwork::Nostr => {
                let nostr_client = self.shared_nostr_client().await?;
                NetworkPager::Nostr(NostrPager::new(nostr_client, source.id.clone(), self.nostr_timeout, self.nostr_relays.clone()))
            }
            SourceNetwork::Bluesky => NetworkPager::Bluesky(BlueskyPager::new(self.http_transport.clone(), source.id.clone())),
            SourceNetwork::Mastodon => NetworkPager::Mastodon(MastodonPager::new(self.http_transport.clone(), source.id.clone())),
            SourceNetwork::Hashiverse => {
                let hashiverse_client = self.ensure_hashiverse_client().await?;
                NetworkPager::Hashiverse(HashiversePager::new(hashiverse_client, source.id.clone()))
            }
        };
        Ok(SourceTimeline::new(source.clone(), pager))
    }

    /// The Hashiverse client to page with: the injected one if present (native
    /// callers inject via [`Self::with_hashiverse_client`]), otherwise — on wasm —
    /// a lazily-built read-only guest client (cached for reuse). On native with no
    /// injected client, Hashiverse is unsupported.
    async fn ensure_hashiverse_client(&mut self) -> anyhow::Result<Arc<HashiverseClient>> {
        if let Some(hashiverse_client) = &self.hashiverse_client {
            return Ok(hashiverse_client.clone());
        }
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            let hashiverse_client = crate::sources::hashiverse::build_guest_client().await?;
            self.hashiverse_client = Some(hashiverse_client.clone());
            Ok(hashiverse_client)
        }
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            anyhow::bail!("Hashiverse sources are not supported by this client (no Hashiverse client available)")
        }
    }

    /// Build a [`MultiTimeline`] mixing every source, sharing the singleton
    /// clients across the children.
    pub async fn build_multi_timeline(&mut self, sources: &[Source]) -> anyhow::Result<MultiTimeline<NetworkPager>> {
        let mut children: Vec<SourceTimeline<NetworkPager>> = Vec::with_capacity(sources.len());
        for source in sources {
            children.push(self.build_source_timeline(source).await?);
        }
        Ok(MultiTimeline::new(children))
    }
}

impl Default for SharedSourceClients {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bluesky + Mastodon pagers construct without any network, so the builder
    /// produces a MultiTimeline whose sources echo the inputs in order.
    #[tokio::test]
    async fn builds_http_sources_in_order() {
        let mut clients = SharedSourceClients::new();
        let sources = vec![Source::new(SourceNetwork::Bluesky, "jay.bsky.team"), Source::new(SourceNetwork::Mastodon, "@Gargron@mastodon.social")];

        let multi = clients.build_multi_timeline(&sources).await.expect("build multi timeline");

        let built_sources: Vec<&Source> = multi.sources();
        assert_eq!(built_sources.len(), 2);
        assert_eq!(built_sources[0], &sources[0]);
        assert_eq!(built_sources[1], &sources[1]);
    }

    /// Without an injected Hashiverse client, a Hashiverse source errors clearly
    /// rather than panicking or silently dropping.
    #[tokio::test]
    async fn hashiverse_without_client_errors() {
        let mut clients = SharedSourceClients::new();
        let source = Source::new(SourceNetwork::Hashiverse, "ddd86177f252f0d33f32aa3e59fb6b554969faad48af443347c5b72ac2e186f0");

        match clients.build_source_timeline(&source).await {
            Ok(_) => panic!("Hashiverse should be unsupported without a client"),
            Err(error) => assert!(error.to_string().contains("Hashiverse"), "error should name Hashiverse: {error}"),
        }
    }

    /// `new()` seeds the default relays; `set_relays` replaces the effective list.
    #[test]
    fn set_relays_replaces_the_effective_list() {
        let mut clients = SharedSourceClients::new();
        assert_eq!(clients.relays(), crate::sources::nostr::DEFAULT_RELAYS);

        let custom = vec!["wss://relay.primal.net".to_string(), "wss://nos.lol".to_string()];
        clients.set_relays(custom.clone());
        assert_eq!(clients.relays(), custom.as_slice());
    }
}
