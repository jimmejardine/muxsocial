//! The app-level registry of timelines — the single source of truth for "which
//! timelines exist and what sources back each one".
//!
//! The GUI is a pure view: it asks for the list, and every mutation is a command
//! that returns the new snapshot. Each timeline has a Rust-minted GUID the GUI
//! uses to address commands at it, and is backed (conceptually) by a
//! [`crate::timeline::MultiTimeline`] of its [`Source`]s. Every change is
//! serialised to the injected [`ConfigStorage`] before returning.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::config_storage::{ConfigStorage, get_json, set_json};
use crate::post::SourceNetwork;
use crate::timeline::Source;

/// The storage key under which the whole timeline list is persisted.
const TIMELINES_KEY: &str = "timelines";

/// One timeline: a GUID plus the sources whose posts it merges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineConfig {
    pub id: String,
    pub sources: Vec<Source>,
}

/// Owns the list of timelines and persists every change to [`ConfigStorage`].
pub struct TimelineRegistry {
    storage: Arc<dyn ConfigStorage>,
    timelines: Vec<TimelineConfig>,
}

impl TimelineRegistry {
    /// Wrap a storage backend. Call [`load`](Self::load) once to read any saved
    /// state.
    pub fn new(storage: Arc<dyn ConfigStorage>) -> Self {
        Self { storage, timelines: Vec::new() }
    }

    /// Load the persisted timeline list (call once at startup).
    pub async fn load(&mut self) -> anyhow::Result<()> {
        self.timelines = get_json::<Vec<TimelineConfig>>(self.storage.as_ref(), TIMELINES_KEY).await?.unwrap_or_default();
        Ok(())
    }

    /// The current timeline snapshot.
    pub fn list(&self) -> Vec<TimelineConfig> {
        self.timelines.clone()
    }

    /// Serialise the whole list to permanent storage.
    async fn persist(&self) -> anyhow::Result<()> {
        set_json(self.storage.as_ref(), TIMELINES_KEY, &self.timelines).await
    }

    /// Add a new, empty timeline with a freshly-minted GUID. Returns the snapshot.
    pub async fn add_timeline(&mut self) -> anyhow::Result<Vec<TimelineConfig>> {
        self.timelines.push(TimelineConfig {
            id: uuid::Uuid::new_v4().to_string(),
            sources: Vec::new(),
        });
        self.persist().await?;
        Ok(self.list())
    }

    /// Remove the timeline with `id`. A no-op if there is no such timeline.
    /// Returns the snapshot.
    pub async fn remove_timeline(&mut self, id: &str) -> anyhow::Result<Vec<TimelineConfig>> {
        self.timelines.retain(|timeline| timeline.id != id);
        self.persist().await?;
        Ok(self.list())
    }

    /// Parse `address` into a [`Source`] and add it to the timeline `id`'s
    /// sources (deduplicated). Returns the snapshot.
    pub async fn add_source_to_timeline(&mut self, id: &str, address: &str) -> anyhow::Result<Vec<TimelineConfig>> {
        let source = parse_source_address(address)?;
        let timeline = self.timelines.iter_mut().find(|timeline| timeline.id == id).ok_or_else(|| anyhow::anyhow!("no timeline with id {id:?}"))?;
        if !timeline.sources.contains(&source) {
            timeline.sources.push(source);
        }
        self.persist().await?;
        Ok(self.list())
    }
}

/// Parse a pasted address into a [`Source`], detecting the network.
///
/// An explicit `network:identifier` prefix (`nostr:`, `bluesky:`/`bsky:`,
/// `mastodon:`/`masto:`, `hashiverse:`/`hash:`) always wins — needed to
/// disambiguate a bare 64-char hex id (nostr vs Hashiverse). Otherwise:
/// `@user@host` → Mastodon, `npub1…` → nostr, `did:plc:…` or a bare dotted handle
/// → Bluesky. Anything else is an error asking for an explicit prefix.
pub fn parse_source_address(address: &str) -> anyhow::Result<Source> {
    let trimmed = address.trim();
    anyhow::ensure!(!trimmed.is_empty(), "empty address");

    // Explicit "network:identifier" prefix wins.
    if let Some((scheme, identifier)) = trimmed.split_once(':') {
        if let Some(network) = network_from_scheme(scheme) {
            anyhow::ensure!(!identifier.trim().is_empty(), "empty identifier after {scheme:?} prefix");
            return Ok(Source::new(network, identifier.trim()));
        }
        // A `:` that is not a known scheme (e.g. `did:plc:…`) falls through.
    }

    // Mastodon: `@user@host` (or `user@host`).
    let without_leading_at = trimmed.trim_start_matches('@');
    if trimmed.contains('@') && without_leading_at.split('@').filter(|part| !part.is_empty()).count() == 2 {
        return Ok(Source::new(SourceNetwork::Mastodon, trimmed));
    }

    // nostr: bech32 npub.
    if trimmed.starts_with("npub1") {
        return Ok(Source::new(SourceNetwork::Nostr, trimmed));
    }

    // Bluesky: a DID, or a bare dotted handle (no '@').
    if trimmed.starts_with("did:plc:") || (trimmed.contains('.') && !trimmed.contains('@')) {
        return Ok(Source::new(SourceNetwork::Bluesky, trimmed));
    }

    anyhow::bail!("could not determine the network for {address:?}; prefix it like \"nostr:<id>\", \"bluesky:<handle>\", \"mastodon:@user@host\", or \"hashiverse:<hex>\"")
}

fn network_from_scheme(scheme: &str) -> Option<SourceNetwork> {
    match scheme.trim().to_ascii_lowercase().as_str() {
        "nostr" => Some(SourceNetwork::Nostr),
        "bluesky" | "bsky" => Some(SourceNetwork::Bluesky),
        "mastodon" | "masto" => Some(SourceNetwork::Mastodon),
        "hashiverse" | "hash" => Some(SourceNetwork::Hashiverse),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_storage::mem_config_storage::MemConfigStorage;

    fn registry() -> TimelineRegistry {
        TimelineRegistry::new(Arc::new(MemConfigStorage::new()))
    }

    #[tokio::test]
    async fn add_remove_and_add_source() {
        let mut timeline_registry = registry();
        assert!(timeline_registry.list().is_empty());

        let after_add = timeline_registry.add_timeline().await.expect("add");
        assert_eq!(after_add.len(), 1);
        let timeline_id = after_add[0].id.clone();
        assert!(after_add[0].sources.is_empty());

        let after_source = timeline_registry.add_source_to_timeline(&timeline_id, "@Gargron@mastodon.social").await.expect("add source");
        assert_eq!(after_source[0].sources, vec![Source::new(SourceNetwork::Mastodon, "@Gargron@mastodon.social")]);

        // Duplicate source is ignored.
        let after_dup = timeline_registry.add_source_to_timeline(&timeline_id, "@Gargron@mastodon.social").await.expect("add dup");
        assert_eq!(after_dup[0].sources.len(), 1);

        let after_remove = timeline_registry.remove_timeline(&timeline_id).await.expect("remove");
        assert!(after_remove.is_empty());
    }

    #[tokio::test]
    async fn persists_across_registries_over_the_same_storage() {
        let storage: Arc<dyn ConfigStorage> = Arc::new(MemConfigStorage::new());

        let mut first = TimelineRegistry::new(storage.clone());
        let snapshot = first.add_timeline().await.expect("add");
        let id = snapshot[0].id.clone();
        first.add_source_to_timeline(&id, "npub1xyz").await.expect("add source");

        // A fresh registry over the same storage sees the saved state.
        let mut second = TimelineRegistry::new(storage.clone());
        second.load().await.expect("load");
        let reloaded = second.list();
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].id, id);
        assert_eq!(reloaded[0].sources, vec![Source::new(SourceNetwork::Nostr, "npub1xyz")]);
    }

    #[test]
    fn parses_addresses_by_network() {
        let cases = [
            ("@Gargron@mastodon.social", SourceNetwork::Mastodon, "@Gargron@mastodon.social"),
            ("npub1wmr34", SourceNetwork::Nostr, "npub1wmr34"),
            ("jay.bsky.team", SourceNetwork::Bluesky, "jay.bsky.team"),
            ("did:plc:abc123", SourceNetwork::Bluesky, "did:plc:abc123"),
            ("nostr:ddd86177", SourceNetwork::Nostr, "ddd86177"),
            ("hashiverse:ddd86177", SourceNetwork::Hashiverse, "ddd86177"),
            ("bsky:jay.bsky.team", SourceNetwork::Bluesky, "jay.bsky.team"),
        ];
        for (address, network, id) in cases {
            assert_eq!(parse_source_address(address).expect(address), Source::new(network, id), "address {address:?}");
        }
    }

    #[test]
    fn rejects_ambiguous_or_empty_addresses() {
        // Bare 64-hex is ambiguous (nostr vs hashiverse) -> needs a prefix.
        assert!(parse_source_address("ddd86177f252f0d33f32aa3e59fb6b554969faad48af443347c5b72ac2e186f0").is_err());
        assert!(parse_source_address("   ").is_err());
    }
}
