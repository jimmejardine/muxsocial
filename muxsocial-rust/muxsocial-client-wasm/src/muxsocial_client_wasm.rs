use std::collections::HashMap;
use std::sync::Arc;

use muxsocial_lib::config_storage::ConfigStorage;
use muxsocial_lib::greeting::compose_greeting_message;
use muxsocial_lib::post::AggregatedPost;
use muxsocial_lib::timeline::{MultiTimeline, NetworkPager, SharedSourceClients};
use muxsocial_lib::timeline_registry::{TimelineRegistry, TimelineView};
use wasm_bindgen::prelude::*;

use crate::indexed_db_config_storage::IndexedDbConfigStorage;

/// The WASM-facing client owned by the MuxsocialWorker. All UI access goes through
/// the MuxsocialClientWasmProxy on the main thread, never through this type
/// directly.
///
/// It owns the [`TimelineRegistry`] — the single source of truth for *which*
/// timelines exist and their sources — over the one [`IndexedDbConfigStorage`]
/// created at startup. The GUI is a pure view: it reads the list and every
/// mutation returns the new snapshot.
///
/// It also owns the live, in-memory pagination state: one
/// [`MultiTimeline`](muxsocial_lib::timeline::MultiTimeline) per timeline id
/// (built lazily on first `get_more_posts`), over a shared set of source clients.
/// Unlike the registry, this state is *not* persisted — a page reload drops the
/// worker and rebuilds it empty (only the registry is reloaded from IndexedDB).
///
/// The worker serialises calls, so the `&mut self` commands never re-enter.
#[wasm_bindgen]
pub struct MuxsocialClientWasm {
    timeline_registry: TimelineRegistry,
    shared_clients: SharedSourceClients,
    /// Live pagination state keyed by timeline id. Dropped (and rebuilt on next
    /// use) whenever the timeline's sources change.
    trackers: HashMap<String, MultiTimeline<NetworkPager>>,
}

#[wasm_bindgen]
impl MuxsocialClientWasm {
    /// Construct the client: open IndexedDB once and load the saved timelines.
    pub async fn create_new() -> Result<MuxsocialClientWasm, JsValue> {
        let storage: Arc<dyn ConfigStorage> = IndexedDbConfigStorage::new().await.map_err(anyhow_to_js)?;
        let mut timeline_registry = TimelineRegistry::new(storage);
        timeline_registry.load().await.map_err(anyhow_to_js)?;
        Ok(MuxsocialClientWasm {
            timeline_registry,
            shared_clients: SharedSourceClients::new(),
            trackers: HashMap::new(),
        })
    }

    pub async fn compose_greeting(&self, recipient_name: String) -> String {
        compose_greeting_message(&recipient_name)
    }

    /// The current timeline snapshot (used by the GUI to seed its view).
    pub async fn list_timelines(&self) -> Result<JsValue, JsValue> {
        snapshot_to_js(&self.timeline_registry.list())
    }

    /// Add a new empty timeline (Rust mints its GUID). Returns the new snapshot.
    pub async fn add_timeline(&mut self) -> Result<JsValue, JsValue> {
        let snapshot = self.timeline_registry.add_timeline().await.map_err(anyhow_to_js)?;
        snapshot_to_js(&snapshot)
    }

    /// Remove the timeline addressed by `id`. Returns the new snapshot.
    pub async fn remove_timeline(&mut self, id: String) -> Result<JsValue, JsValue> {
        let snapshot = self.timeline_registry.remove_timeline(&id).await.map_err(anyhow_to_js)?;
        // The timeline is gone; drop any live pagination state for it.
        self.trackers.remove(&id);
        snapshot_to_js(&snapshot)
    }

    /// Parse `address` and add it as a source of the timeline addressed by `id`.
    /// Returns the new snapshot.
    pub async fn add_source_to_timeline(&mut self, id: String, address: String) -> Result<JsValue, JsValue> {
        let snapshot = self.timeline_registry.add_source_to_timeline(&id, &address).await.map_err(anyhow_to_js)?;
        // The source set changed; drop the tracker so it rebuilds over the new sources.
        self.trackers.remove(&id);
        snapshot_to_js(&snapshot)
    }

    /// Set (or, with an empty string, clear) the custom name of the timeline
    /// addressed by `id`. Returns the new snapshot.
    pub async fn set_timeline_name(&mut self, id: String, name: String) -> Result<JsValue, JsValue> {
        let snapshot = self.timeline_registry.set_timeline_name(&id, &name).await.map_err(anyhow_to_js)?;
        snapshot_to_js(&snapshot)
    }

    /// Pull the next page of posts for the timeline `id`, fetching up to
    /// `per_source_limit` posts from each source. Returns only the **newly-added
    /// batch** (newest-first); the caller accumulates it into its own view and
    /// can reseed the full list via [`Self::timeline_posts`]. Builds the
    /// timeline's live pager on first use.
    pub async fn get_more_posts(&mut self, id: String, per_source_limit: usize) -> Result<JsValue, JsValue> {
        self.ensure_tracker(&id).await.map_err(anyhow_to_js)?;
        let tracker = self.trackers.get_mut(&id).expect("tracker just ensured");
        let new_posts = tracker.get_more(per_source_limit).await.map_err(anyhow_to_js)?;
        posts_to_js(&new_posts)
    }

    /// The full accumulated post list for the timeline `id`, newest-first — used
    /// by the GUI to reseed its view on (re)mount without re-fetching from the
    /// networks. Empty if the timeline has not been paged yet this session.
    pub async fn timeline_posts(&self, id: String) -> Result<JsValue, JsValue> {
        match self.trackers.get(&id) {
            Some(tracker) => posts_to_js(tracker.posts()),
            None => posts_to_js(&[]),
        }
    }

    /// Build and cache the live `MultiTimeline` for `id` if not already present.
    async fn ensure_tracker(&mut self, id: &str) -> anyhow::Result<()> {
        if self.trackers.contains_key(id) {
            return Ok(());
        }
        let sources = self.timeline_registry.sources_for(id).ok_or_else(|| anyhow::anyhow!("no timeline with id {id:?}"))?.to_vec();
        let multi_timeline = self.shared_clients.build_multi_timeline(&sources).await?;
        self.trackers.insert(id.to_string(), multi_timeline);
        Ok(())
    }
}

fn snapshot_to_js(snapshot: &[TimelineView]) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(snapshot).map_err(|serialize_error| JsValue::from_str(&format!("serializing timelines: {serialize_error}")))
}

fn posts_to_js(posts: &[AggregatedPost]) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(posts).map_err(|serialize_error| JsValue::from_str(&format!("serializing posts: {serialize_error}")))
}

fn anyhow_to_js(error: anyhow::Error) -> JsValue {
    JsValue::from_str(&format!("{error:#}"))
}
