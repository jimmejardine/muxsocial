//! The app-level registry of timelines — the single source of truth for "which
//! timelines exist and what sources back each one".
//!
//! The GUI is a pure view: it asks for the list, and every mutation is a command
//! that returns the new snapshot. Each timeline has a Rust-minted GUID the GUI
//! uses to address commands at it, and is backed (conceptually) by a
//! [`crate::timeline::MultiTimeline`] of its [`Source`]s. Every change is
//! serialised to the injected [`ConfigStorage`] before returning.

use std::sync::{Arc, LazyLock};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::config_storage::{ConfigStorage, get_json, set_json};
use crate::post::SourceNetwork;
use crate::timeline::Source;

/// The storage key under which the whole timeline list is persisted.
const TIMELINES_KEY: &str = "timelines";

/// One timeline as persisted: a GUID, an optional user-set name, and the sources
/// whose posts it merges. `name` is `None` until the user names it; the effective
/// name is then derived from the sources (see [`TimelineConfig::display_name`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineConfig {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub sources: Vec<Source>,
    /// Whether this timeline auto-polls for new posts on the recurring tick. The
    /// initial pull is automatic regardless; this only gates the periodic refresh.
    /// Defaults to `true` so timelines persisted before the flag keep polling.
    #[serde(default = "default_autopoll")]
    pub autopoll: bool,
}

/// The default for [`TimelineConfig::autopoll`] (and pre-flag persisted timelines).
fn default_autopoll() -> bool {
    true
}

/// A timeline as handed to the GUI: the persisted fields plus the computed
/// effective name (`display_name`), so the truncation/concatenation rule lives
/// only in Rust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineView {
    pub id: String,
    pub name: Option<String>,
    pub display_name: String,
    pub sources: Vec<Source>,
    pub autopoll: bool,
}

impl TimelineConfig {
    /// The effective name to show: a non-empty user-set name wins; otherwise the
    /// source ids each truncated to 16 chars (`+ "..."` when longer) joined with
    /// `", "`; otherwise empty (the GUI supplies a fallback for a nameless,
    /// sourceless timeline).
    pub fn display_name(&self) -> String {
        if let Some(name) = self.name.as_deref().map(str::trim).filter(|name| !name.is_empty()) {
            return name.to_string();
        }
        self.sources.iter().map(|source| truncate_16(&source.id)).collect::<Vec<_>>().join(", ")
    }

    fn to_view(&self) -> TimelineView {
        TimelineView {
            id: self.id.clone(),
            name: self.name.clone(),
            display_name: self.display_name(),
            sources: self.sources.clone(),
            autopoll: self.autopoll,
        }
    }
}

/// Take the first 16 characters of `id`, appending `"..."` only if it was longer.
fn truncate_16(id: &str) -> String {
    let mut characters = id.chars();
    let prefix: String = characters.by_ref().take(16).collect();
    if characters.next().is_some() { format!("{prefix}...") } else { prefix }
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

    /// The current timeline snapshot (with computed display names).
    pub fn list(&self) -> Vec<TimelineView> {
        self.timelines.iter().map(TimelineConfig::to_view).collect()
    }

    /// The sources backing the timeline `id`, if such a timeline exists. Used by
    /// the client to build that timeline's live [`crate::timeline::MultiTimeline`].
    pub fn sources_for(&self, id: &str) -> Option<&[Source]> {
        self.timelines.iter().find(|timeline| timeline.id == id).map(|timeline| timeline.sources.as_slice())
    }

    /// Serialise the whole list to permanent storage.
    async fn persist(&self) -> anyhow::Result<()> {
        set_json(self.storage.as_ref(), TIMELINES_KEY, &self.timelines).await
    }

    /// Add a new, empty timeline with a freshly-minted GUID. Returns the snapshot.
    pub async fn add_timeline(&mut self) -> anyhow::Result<Vec<TimelineView>> {
        self.timelines.push(TimelineConfig {
            id: uuid::Uuid::new_v4().to_string(),
            name: None,
            sources: Vec::new(),
            autopoll: default_autopoll(),
        });
        self.persist().await?;
        Ok(self.list())
    }

    /// Remove the timeline with `id`. A no-op if there is no such timeline.
    /// Returns the snapshot.
    pub async fn remove_timeline(&mut self, id: &str) -> anyhow::Result<Vec<TimelineView>> {
        self.timelines.retain(|timeline| timeline.id != id);
        self.persist().await?;
        Ok(self.list())
    }

    /// Parse `address` into a [`Source`] and add it to the timeline `id`'s
    /// sources (deduplicated). Returns the snapshot.
    pub async fn add_source_to_timeline(&mut self, id: &str, address: &str) -> anyhow::Result<Vec<TimelineView>> {
        let source = parse_source_address(address)?;
        let timeline = self.timeline_mut(id)?;
        if !timeline.sources.contains(&source) {
            timeline.sources.push(source);
        }
        self.persist().await?;
        Ok(self.list())
    }

    /// Remove `source` from the timeline `id`'s sources (a no-op if absent).
    /// Returns the snapshot.
    pub async fn remove_source_from_timeline(&mut self, id: &str, source: &Source) -> anyhow::Result<Vec<TimelineView>> {
        let timeline = self.timeline_mut(id)?;
        timeline.sources.retain(|existing| existing != source);
        self.persist().await?;
        Ok(self.list())
    }

    /// Set (or clear) the timeline `id`'s custom name. An empty/whitespace name
    /// clears it, reverting to the source-derived default. Returns the snapshot.
    pub async fn set_timeline_name(&mut self, id: &str, name: &str) -> anyhow::Result<Vec<TimelineView>> {
        let trimmed = name.trim();
        let timeline = self.timeline_mut(id)?;
        timeline.name = if trimmed.is_empty() { None } else { Some(trimmed.to_string()) };
        self.persist().await?;
        Ok(self.list())
    }

    /// Set the timeline `id`'s autopoll flag (whether it refreshes on the
    /// recurring tick). Returns the snapshot.
    pub async fn set_timeline_autopoll(&mut self, id: &str, autopoll: bool) -> anyhow::Result<Vec<TimelineView>> {
        let timeline = self.timeline_mut(id)?;
        timeline.autopoll = autopoll;
        self.persist().await?;
        Ok(self.list())
    }

    fn timeline_mut(&mut self, id: &str) -> anyhow::Result<&mut TimelineConfig> {
        self.timelines.iter_mut().find(|timeline| timeline.id == id).ok_or_else(|| anyhow::anyhow!("no timeline with id {id:?}"))
    }

    /// The whole timeline list as a JSON string — the same shape persisted under
    /// the `"timelines"` key, and the `"timelines"` root element of the GUI's
    /// config-transfer dialog.
    pub fn export_timelines_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&self.timelines)?)
    }

    /// Replace the whole timeline list with one parsed from `timelines_json` (the
    /// shape produced by [`export_timelines_json`](Self::export_timelines_json)).
    /// The import is all-or-nothing: nothing changes unless the JSON parses and
    /// every timeline has a non-empty unique id and non-empty source ids.
    /// Persists and returns the new snapshot.
    pub async fn import_timelines_json(&mut self, timelines_json: &str) -> anyhow::Result<Vec<TimelineView>> {
        let imported_timelines: Vec<TimelineConfig> = serde_json::from_str(timelines_json).map_err(|parse_error| anyhow::anyhow!("invalid timelines JSON: {parse_error}"))?;
        let mut seen_ids = std::collections::HashSet::new();
        for timeline in &imported_timelines {
            anyhow::ensure!(!timeline.id.trim().is_empty(), "a timeline has an empty id");
            anyhow::ensure!(seen_ids.insert(timeline.id.as_str()), "duplicate timeline id {:?}", timeline.id);
            for source in &timeline.sources {
                anyhow::ensure!(!source.id.trim().is_empty(), "timeline {:?} has a source with an empty id", timeline.id);
            }
        }
        self.timelines = imported_timelines;
        self.persist().await?;
        Ok(self.list())
    }
}

/// Parse a pasted address into a [`Source`], detecting the network.
///
/// Resolution is a cascade of four fall-through layers, each reusing the layer
/// before it to classify (so "what is a valid id" lives in exactly one place):
///
/// 1. **Explicit prefix** — `nostr:`, `bluesky:`/`bsky:`, `mastodon:`/`masto:`,
///    `hashiverse:`/`hash:` (see [`match_explicit_prefix`]).
/// 2. **Bare id** — `@user@host` → Mastodon, `npub1…`/`nprofile1…` → nostr,
///    `did:plc:…` or a dotted handle → Bluesky, a 64-char hex → Hashiverse
///    (see [`match_pure_id`]).
/// 3. **A profile URL** (scheme optional, any host) — the user id is located by
///    URL *structure* and fed back through layers 1–2 (see [`match_url`]).
/// 4. **A URL embedded in free text** — a URL (or bare `npub`) is extracted from
///    surrounding prose, then run through layer 3 (see [`match_in_text`]).
///
/// Anything else is an error.
pub fn parse_source_address(address: &str) -> anyhow::Result<Source> {
    let trimmed = address.trim();
    anyhow::ensure!(!trimmed.is_empty(), "empty address");

    match_explicit_prefix(trimmed)
        .or_else(|| match_pure_id(trimmed))
        .or_else(|| match_url(trimmed))
        .or_else(|| match_in_text(trimmed))
        .ok_or_else(|| anyhow::anyhow!("could not determine the network for {address:?}; prefix it like \"nostr:<id>\", \"bluesky:<handle>\", \"mastodon:@user@host\", or \"hashiverse:<hex>\""))
}

/// Layer 1: an explicit `network:identifier` prefix. The user named the network,
/// so the identifier is trusted verbatim (only `nprofile1…` is decoded to its
/// `npub`, and a Bluesky leading `@` is stripped).
fn match_explicit_prefix(address: &str) -> Option<Source> {
    let (scheme, identifier) = address.split_once(':')?;
    let network = network_from_scheme(scheme)?;
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return None;
    }
    let id = match network {
        // ATProto handles have no leading '@'; tolerate one if pasted.
        SourceNetwork::Bluesky => identifier.trim_start_matches('@').to_string(),
        SourceNetwork::Nostr => normalize_nostr(identifier)?,
        _ => identifier.to_string(),
    };
    if id.is_empty() {
        return None;
    }
    Some(Source::new(network, id))
}

/// Layer 2: a bare, well-formed id. Rejects anything containing whitespace or `/`
/// (a real id has neither) so a schemeless URL like `mastodon.social/@user` falls
/// through to the URL layer instead of being misgrabbed here.
fn match_pure_id(candidate: &str) -> Option<Source> {
    if candidate.is_empty() || candidate.contains('/') || candidate.contains(char::is_whitespace) {
        return None;
    }

    // Mastodon: `@user@host` (or `user@host`).
    let without_leading_at = candidate.trim_start_matches('@');
    if candidate.contains('@') && without_leading_at.split('@').filter(|part| !part.is_empty()).count() == 2 {
        return Some(Source::new(SourceNetwork::Mastodon, candidate));
    }

    // nostr: bech32 `npub` (or an `nprofile`, decoded to its `npub`).
    if candidate.starts_with("npub1") || candidate.starts_with("nprofile1") {
        return normalize_nostr(candidate).map(|id| Source::new(SourceNetwork::Nostr, id));
    }

    // Bluesky: a DID, or a bare dotted handle (optionally with a single leading '@').
    if without_leading_at.starts_with("did:plc:") || (without_leading_at.contains('.') && !without_leading_at.contains('@')) {
        return Some(Source::new(SourceNetwork::Bluesky, without_leading_at));
    }

    // A bare 64-char hex id is a Hashiverse user id.
    if candidate.len() == 64 && candidate.chars().all(|character| character.is_ascii_hexdigit()) {
        return Some(Source::new(SourceNetwork::Hashiverse, candidate));
    }

    None
}

/// Layer 3: a single profile URL (scheme optional, host-agnostic). Locates the id
/// by URL *structure* and delegates classification to the previous layers — so this
/// only encodes *where the id lives*, not how to recognise a network.
fn match_url(input: &str) -> Option<Source> {
    // A whole-input URL: `[scheme://]host[/path?query#fragment]`, no spaces.
    static URL_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?i)(?:https?://)?([^/?#\s]+)([/?#]\S*)?$").unwrap());
    let captures = URL_SPLIT.captures(input)?;
    let host = captures.get(1)?.as_str();
    let rest = captures.get(2).map_or("", |m| m.as_str());

    // 1. nostr — an `npub`/`nprofile` token anywhere in the URL.
    if let Some(token) = find_nostr_token(input) {
        if let Some(id) = normalize_nostr(token) {
            return Some(Source::new(SourceNetwork::Nostr, id));
        }
    }
    // 2. Hashiverse — a 64-hex segment in the path/fragment (route-name agnostic).
    if let Some(hex) = find_hex64(rest) {
        if let Some(source) = match_pure_id(hex) {
            return Some(source);
        }
    }
    // 3. Bluesky — the segment after a `profile/` path segment (drops a trailing `/post/…`).
    static PROFILE_SEG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|/)profile/([^/?#]+)").unwrap());
    if let Some(segment) = PROFILE_SEG.captures(rest).and_then(|c| c.get(1)).map(|m| m.as_str()) {
        if let Some(source) = match_pure_id(segment) {
            return Some(source);
        }
    }
    // 4. Mastodon — a `/@user[@host]` (or `/users/<user>`) segment + the URL host.
    if let Some(candidate) = mastodon_candidate(rest, host) {
        if let Some(source) = match_pure_id(&candidate) {
            return Some(source);
        }
    }

    None
}

/// Layer 4: a URL (or a bare nostr token) embedded in surrounding prose.
fn match_in_text(text: &str) -> Option<Source> {
    // A nostr token can appear bare in prose ("follow me: npub1…").
    if let Some(token) = find_nostr_token(text) {
        if let Some(id) = normalize_nostr(token) {
            return Some(Source::new(SourceNetwork::Nostr, id));
        }
    }
    // Otherwise pull out URL-shaped tokens (schemed, or a bare `host.tld/path`) and
    // run the URL layer on each.
    static URL_TOKEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)(?:https?://\S+)|(?:(?:[a-z0-9-]+\.)+[a-z]{2,}[/#?]\S*)").unwrap());
    for matched in URL_TOKEN.find_iter(text) {
        let token = matched.as_str().trim_end_matches(['.', ',', ')', ']', '>', '"']);
        if let Some(source) = match_url(token) {
            return Some(source);
        }
    }
    None
}

/// Build a Mastodon `@user@host` candidate from a URL path: a `@user[@homehost]`
/// segment (the home host wins when present, else the URL host), or a `/users/<user>`
/// actor segment. Trailing path segments (status ids) are ignored.
fn mastodon_candidate(rest: &str, url_host: &str) -> Option<String> {
    static AT_SEG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|/)@([^/?#]+)").unwrap());
    static USERS_SEG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|/)users/([^/?#]+)").unwrap());

    if let Some(segment) = AT_SEG.captures(rest).and_then(|c| c.get(1)).map(|m| m.as_str()) {
        // `@user@homehost` already names its home instance; `@user` lives on the URL host.
        if segment.contains('@') {
            return Some(format!("@{segment}"));
        }
        return Some(format!("@{segment}@{url_host}"));
    }
    if let Some(user) = USERS_SEG.captures(rest).and_then(|c| c.get(1)).map(|m| m.as_str()) {
        return Some(format!("@{user}@{url_host}"));
    }
    None
}

/// The first `npub1…` or `nprofile1…` bech32 token in `text`, if any.
fn find_nostr_token(text: &str) -> Option<&str> {
    static NOSTR_TOKEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:npub1|nprofile1)[02-9ac-hj-np-z]+").unwrap());
    NOSTR_TOKEN.find(text).map(|m| m.as_str())
}

/// The first 64-char hex run in `text` bounded by non-hex characters (so a 63- or
/// 65-long run does not match), if any.
fn find_hex64(text: &str) -> Option<&str> {
    static HEX64: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|[^0-9a-fA-F])([0-9a-fA-F]{64})(?:[^0-9a-fA-F]|$)").unwrap());
    HEX64.captures(text).and_then(|c| c.get(1)).map(|m| m.as_str())
}

/// Normalise a nostr identifier to the string the fetch client wants: an `npub1…`
/// (or any non-bech32 form, e.g. hex from an explicit `nostr:` prefix) passes
/// through verbatim; an `nprofile1…` is decoded to its public key's `npub`.
fn normalize_nostr(token: &str) -> Option<String> {
    if token.starts_with("nprofile1") {
        use nostr_sdk::prelude::{FromBech32, Nip19Profile, ToBech32};
        let profile = Nip19Profile::from_bech32(token).ok()?;
        return profile.public_key.to_bech32().ok();
    }
    Some(token.to_string())
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
    async fn removes_one_source_and_keeps_the_other() {
        let mut timeline_registry = registry();
        let id = timeline_registry.add_timeline().await.expect("add")[0].id.clone();
        timeline_registry.add_source_to_timeline(&id, "npub1abc").await.expect("add nostr");
        timeline_registry.add_source_to_timeline(&id, "@Gargron@mastodon.social").await.expect("add masto");

        let after = timeline_registry.remove_source_from_timeline(&id, &Source::new(SourceNetwork::Nostr, "npub1abc")).await.expect("remove source");
        assert_eq!(after[0].sources, vec![Source::new(SourceNetwork::Mastodon, "@Gargron@mastodon.social")]);

        // Persisted: a fresh registry over the same storage sees the removal.
        let mut reloaded = TimelineRegistry::new(timeline_registry.storage.clone());
        reloaded.load().await.expect("load");
        assert_eq!(reloaded.list()[0].sources.len(), 1);
    }

    #[tokio::test]
    async fn default_name_is_truncated_joined_source_ids() {
        let mut timeline_registry = registry();
        let id = timeline_registry.add_timeline().await.expect("add")[0].id.clone();

        // No sources -> empty default name.
        assert_eq!(timeline_registry.list()[0].display_name, "");

        // Two sources -> "<id-trunc>, <id-trunc>", each cut at 16 chars + "...".
        timeline_registry.add_source_to_timeline(&id, "@Gargron@mastodon.social").await.expect("add masto");
        timeline_registry.add_source_to_timeline(&id, "nostr:abcdef").await.expect("add nostr");
        // "@Gargron@mastodon.social" is 24 chars -> first 16 + "...". "abcdef" is short.
        assert_eq!(timeline_registry.list()[0].display_name, "@Gargron@mastodo..., abcdef");
        assert_eq!(timeline_registry.list()[0].name, None);
    }

    #[tokio::test]
    async fn custom_name_overrides_and_clearing_reverts() {
        let mut timeline_registry = registry();
        let id = timeline_registry.add_timeline().await.expect("add")[0].id.clone();
        timeline_registry.add_source_to_timeline(&id, "nostr:abcdef").await.expect("add source");

        let named = timeline_registry.set_timeline_name(&id, "  My feed  ").await.expect("set name");
        assert_eq!(named[0].name.as_deref(), Some("My feed"));
        assert_eq!(named[0].display_name, "My feed");

        // Clearing reverts to the source-derived default.
        let cleared = timeline_registry.set_timeline_name(&id, "   ").await.expect("clear name");
        assert_eq!(cleared[0].name, None);
        assert_eq!(cleared[0].display_name, "abcdef");
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

    #[tokio::test]
    async fn autopoll_defaults_on_and_toggles_and_persists() {
        let storage: Arc<dyn ConfigStorage> = Arc::new(MemConfigStorage::new());
        let mut registry = TimelineRegistry::new(storage.clone());
        let id = registry.add_timeline().await.expect("add")[0].id.clone();

        // New timelines default to autopoll on.
        assert!(registry.list()[0].autopoll);

        let after = registry.set_timeline_autopoll(&id, false).await.expect("set autopoll");
        assert!(!after[0].autopoll);

        // The toggled value survives a reload.
        let mut reloaded = TimelineRegistry::new(storage.clone());
        reloaded.load().await.expect("load");
        assert!(!reloaded.list()[0].autopoll);
    }

    #[tokio::test]
    async fn export_import_roundtrips_across_storages() {
        let mut original = registry();
        let id = original.add_timeline().await.expect("add")[0].id.clone();
        original.add_source_to_timeline(&id, "@Gargron@mastodon.social").await.expect("add source");
        original.set_timeline_name(&id, "My feed").await.expect("set name");
        original.set_timeline_autopoll(&id, false).await.expect("set autopoll");

        let exported_json = original.export_timelines_json().expect("export");

        // Import into a registry over a *different* storage; the lists match.
        let storage: Arc<dyn ConfigStorage> = Arc::new(MemConfigStorage::new());
        let mut imported = TimelineRegistry::new(storage.clone());
        let snapshot = imported.import_timelines_json(&exported_json).await.expect("import");
        assert_eq!(snapshot, original.list());

        // The import persisted: a fresh registry over the same storage sees it.
        let mut reloaded = TimelineRegistry::new(storage);
        reloaded.load().await.expect("load");
        assert_eq!(reloaded.list(), original.list());
    }

    #[tokio::test]
    async fn import_replaces_the_existing_timelines() {
        let mut timeline_registry = registry();
        timeline_registry.add_timeline().await.expect("add");

        let snapshot = timeline_registry
            .import_timelines_json(r#"[{"id":"imported-id","name":null,"sources":[{"network":"Nostr","id":"npub1xyz"}],"autopoll":true}]"#)
            .await
            .expect("import");
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].id, "imported-id");
        assert_eq!(snapshot[0].sources, vec![Source::new(SourceNetwork::Nostr, "npub1xyz")]);

        // An empty list clears everything.
        let cleared = timeline_registry.import_timelines_json("[]").await.expect("import empty");
        assert!(cleared.is_empty());
    }

    #[tokio::test]
    async fn import_applies_the_same_serde_defaults_as_load() {
        // `name` and `autopoll` are optional in persisted JSON; an import without
        // them gets the same defaults a load would (None / autopoll on).
        let mut timeline_registry = registry();
        let snapshot = timeline_registry.import_timelines_json(r#"[{"id":"a","sources":[]}]"#).await.expect("import");
        assert_eq!(snapshot[0].name, None);
        assert!(snapshot[0].autopoll);
    }

    #[tokio::test]
    async fn import_rejects_invalid_payloads_and_changes_nothing() {
        let mut timeline_registry = registry();
        let id = timeline_registry.add_timeline().await.expect("add")[0].id.clone();

        let invalid_payloads = [
            "not json at all",
            r#"{"id":"not-a-list"}"#,
            // Unknown network variant.
            r#"[{"id":"a","sources":[{"network":"Myspace","id":"x"}],"autopoll":true}]"#,
            // Empty timeline id.
            r#"[{"id":"  ","sources":[],"autopoll":true}]"#,
            // Duplicate timeline ids.
            r#"[{"id":"a","sources":[],"autopoll":true},{"id":"a","sources":[],"autopoll":true}]"#,
            // Empty source id.
            r#"[{"id":"a","sources":[{"network":"Nostr","id":""}],"autopoll":true}]"#,
        ];
        for invalid_payload in invalid_payloads {
            assert!(timeline_registry.import_timelines_json(invalid_payload).await.is_err(), "payload should be rejected: {invalid_payload:?}");
        }

        // The rejected imports were all-or-nothing: the original timeline survives.
        let snapshot = timeline_registry.list();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].id, id);
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
            // A Bluesky handle pasted with a leading '@' is still Bluesky; the '@' is stripped.
            ("@sherif.eurosky.social", SourceNetwork::Bluesky, "sherif.eurosky.social"),
            ("sherif.eurosky.social", SourceNetwork::Bluesky, "sherif.eurosky.social"),
            ("@jay.bsky.team", SourceNetwork::Bluesky, "jay.bsky.team"),
            ("bluesky:@sherif.eurosky.social", SourceNetwork::Bluesky, "sherif.eurosky.social"),
            ("bsky:@jay.bsky.team", SourceNetwork::Bluesky, "jay.bsky.team"),
            // A bare 64-char hex id defaults to Hashiverse.
            (
                "ddd86177f252f0d33f32aa3e59fb6b554969faad48af443347c5b72ac2e186f0",
                SourceNetwork::Hashiverse,
                "ddd86177f252f0d33f32aa3e59fb6b554969faad48af443347c5b72ac2e186f0",
            ),
        ];
        for (address, network, id) in cases {
            assert_eq!(parse_source_address(address).expect(address), Source::new(network, id), "address {address:?}");
        }
    }

    #[test]
    fn rejects_empty_or_unrecognised_addresses() {
        assert!(parse_source_address("   ").is_err());
        // Not hex, not a handle/npub/did -> unrecognised.
        assert!(parse_source_address("just some text").is_err());
        // 63 hex chars (wrong length) is not a Hashiverse id.
        assert!(parse_source_address("ddd86177f252f0d33f32aa3e59fb6b554969faad48af443347c5b72ac2e186").is_err());
    }

    const HEX: &str = "ddd86177f252f0d33f32aa3e59fb6b554969faad48af443347c5b72ac2e186f0";
    const NPUB_A: &str = "npub1j6jh3a44q3jxmc2ph2gta3t9r9j65qwlqcze9zehsksnwfgyay7szpg7fw";
    const NPUB_B: &str = "npub1wmr34t36fy03m8hvgl96zl3znndyzyaqhwmwdtshwmtkg03fetaqhjg240";

    #[test]
    fn parses_profile_urls_to_sources() {
        let cases: [(String, Source); 22] = [
            // Hashiverse — any host with a 64-hex segment; http or schemeless; route-name agnostic.
            (format!("https://app.hashiverse.com/#/user/{HEX}"), Source::new(SourceNetwork::Hashiverse, HEX)),
            (format!("https://app.hashiverse.com/#/user_mentioned/{HEX}"), Source::new(SourceNetwork::Hashiverse, HEX)),
            (format!("app.hashiverse.com/#/user/{HEX}"), Source::new(SourceNetwork::Hashiverse, HEX)),
            (format!("https://hashiverse.eu/#/user/{HEX}"), Source::new(SourceNetwork::Hashiverse, HEX)),
            (format!("https://my-node.example/#/user/{HEX}"), Source::new(SourceNetwork::Hashiverse, HEX)),
            (format!("Volg me op Hashiverse op https://app.hashiverse.com/#/user/{HEX}"), Source::new(SourceNetwork::Hashiverse, HEX)),
            // Nostr — any host carrying an npub, plus a bare npub in prose.
            (format!("https://njump.me/{NPUB_B}"), Source::new(SourceNetwork::Nostr, NPUB_B)),
            (format!("https://jumble.social/users/{NPUB_A}"), Source::new(SourceNetwork::Nostr, NPUB_A)),
            (format!("follow me: {NPUB_A}"), Source::new(SourceNetwork::Nostr, NPUB_A)),
            // Bluesky — any host with /profile/<x>; drops a trailing /post/…; http or schemeless.
            ("https://bsky.app/profile/bsky.app".to_string(), Source::new(SourceNetwork::Bluesky, "bsky.app")),
            ("https://bsky.app/profile/cooperlund.online".to_string(), Source::new(SourceNetwork::Bluesky, "cooperlund.online")),
            ("https://bsky.app/profile/joshuajfriedman.com/post/3kzwdmzvnf223".to_string(), Source::new(SourceNetwork::Bluesky, "joshuajfriedman.com")),
            ("https://bsky.app/profile/did:plc:abc123".to_string(), Source::new(SourceNetwork::Bluesky, "did:plc:abc123")),
            ("bsky.app/profile/cooperlund.online".to_string(), Source::new(SourceNetwork::Bluesky, "cooperlund.online")),
            ("https://deer.social/profile/jay.bsky.team".to_string(), Source::new(SourceNetwork::Bluesky, "jay.bsky.team")),
            // Mastodon — any fediverse host with /@user, /@user@host, or /users/<user>; status dropped.
            ("https://mastodon.social/@Gargron".to_string(), Source::new(SourceNetwork::Mastodon, "@Gargron@mastodon.social")),
            ("https://mastodon.social/@Gargron/116734045693755022".to_string(), Source::new(SourceNetwork::Mastodon, "@Gargron@mastodon.social")),
            ("https://hachyderm.io/@user".to_string(), Source::new(SourceNetwork::Mastodon, "@user@hachyderm.io")),
            ("https://mastodon.social/users/Gargron".to_string(), Source::new(SourceNetwork::Mastodon, "@Gargron@mastodon.social")),
            ("https://mastodon.social/@paul@tapbots.social".to_string(), Source::new(SourceNetwork::Mastodon, "@paul@tapbots.social")),
            ("mastodon.social/@matt1corey@iosdev.space".to_string(), Source::new(SourceNetwork::Mastodon, "@matt1corey@iosdev.space")),
            ("https://mastodon.social/@matt1corey@iosdev.space/116734653159593295".to_string(), Source::new(SourceNetwork::Mastodon, "@matt1corey@iosdev.space")),
        ];
        for (address, expected) in cases {
            assert_eq!(parse_source_address(&address).expect(&address), expected, "address {address:?}");
        }
    }

    #[test]
    fn decodes_nprofile_to_its_npub() {
        use nostr_sdk::prelude::{Nip19Profile, PublicKey, RelayUrl, ToBech32};
        let public_key = PublicKey::parse(NPUB_A).expect("parse npub");
        let nprofile = Nip19Profile::new(public_key, Vec::<RelayUrl>::new()).to_bech32().expect("encode nprofile");
        // An nprofile in a URL, and via the explicit nostr: prefix, both resolve to the npub.
        assert_eq!(parse_source_address(&format!("https://njump.me/{nprofile}")).expect("url"), Source::new(SourceNetwork::Nostr, NPUB_A));
        assert_eq!(parse_source_address(&format!("nostr:{nprofile}")).expect("prefix"), Source::new(SourceNetwork::Nostr, NPUB_A));
    }

    #[test]
    fn find_hex64_requires_exactly_64() {
        assert_eq!(find_hex64(&format!("/#/user/{HEX}")), Some(HEX));
        assert_eq!(find_hex64(&format!("/x/{HEX}/post")), Some(HEX));
        // A 63- or 65-char run is not a Hashiverse id.
        assert_eq!(find_hex64(&HEX[..63]), None);
        assert_eq!(find_hex64(&format!("{HEX}a")), None);
    }

    #[test]
    fn finds_nostr_tokens() {
        assert_eq!(find_nostr_token(&format!("https://njump.me/{NPUB_B}")), Some(NPUB_B));
        assert_eq!(find_nostr_token("no token here"), None);
    }

    #[test]
    fn mastodon_candidate_builds_the_handle() {
        assert_eq!(mastodon_candidate("/@Gargron", "mastodon.social"), Some("@Gargron@mastodon.social".to_string()));
        assert_eq!(mastodon_candidate("/@Gargron/12345", "mastodon.social"), Some("@Gargron@mastodon.social".to_string()));
        assert_eq!(mastodon_candidate("/@paul@tapbots.social", "mastodon.social"), Some("@paul@tapbots.social".to_string()));
        assert_eq!(mastodon_candidate("/users/Gargron", "mastodon.social"), Some("@Gargron@mastodon.social".to_string()));
        assert_eq!(mastodon_candidate("/explore", "mastodon.social"), None);
    }

    #[test]
    fn pure_id_rejects_url_like_strings() {
        // A '/' or whitespace means it's not a bare id; let the URL layers handle it.
        assert!(match_pure_id("mastodon.social/@user").is_none());
        assert!(match_pure_id("has space").is_none());
    }
}
