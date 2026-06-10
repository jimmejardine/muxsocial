use std::sync::Arc;

use muxsocial_lib::config_storage::ConfigStorage;
use muxsocial_lib::greeting::compose_greeting_message;
use muxsocial_lib::timeline_registry::{TimelineConfig, TimelineRegistry};
use wasm_bindgen::prelude::*;

use crate::indexed_db_config_storage::IndexedDbConfigStorage;

/// The WASM-facing client owned by the MuxsocialWorker. All UI access goes through
/// the MuxsocialClientWasmProxy on the main thread, never through this type
/// directly.
///
/// It owns the [`TimelineRegistry`] — the single source of truth for the
/// timelines — over the one [`IndexedDbConfigStorage`] created at startup. The
/// GUI is a pure view: it reads the list and every mutation returns the new
/// snapshot. The worker serialises calls, so the `&mut self` commands never
/// re-enter.
#[wasm_bindgen]
pub struct MuxsocialClientWasm {
    timeline_registry: TimelineRegistry,
}

#[wasm_bindgen]
impl MuxsocialClientWasm {
    /// Construct the client: open IndexedDB once and load the saved timelines.
    pub async fn create_new() -> Result<MuxsocialClientWasm, JsValue> {
        let storage: Arc<dyn ConfigStorage> = IndexedDbConfigStorage::new().await.map_err(anyhow_to_js)?;
        let mut timeline_registry = TimelineRegistry::new(storage);
        timeline_registry.load().await.map_err(anyhow_to_js)?;
        Ok(MuxsocialClientWasm { timeline_registry })
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
        snapshot_to_js(&snapshot)
    }

    /// Parse `address` and add it as a source of the timeline addressed by `id`.
    /// Returns the new snapshot.
    pub async fn add_source_to_timeline(&mut self, id: String, address: String) -> Result<JsValue, JsValue> {
        let snapshot = self.timeline_registry.add_source_to_timeline(&id, &address).await.map_err(anyhow_to_js)?;
        snapshot_to_js(&snapshot)
    }
}

fn snapshot_to_js(snapshot: &[TimelineConfig]) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(snapshot).map_err(|serialize_error| JsValue::from_str(&format!("serializing timelines: {serialize_error}")))
}

fn anyhow_to_js(error: anyhow::Error) -> JsValue {
    JsValue::from_str(&format!("{error:#}"))
}
