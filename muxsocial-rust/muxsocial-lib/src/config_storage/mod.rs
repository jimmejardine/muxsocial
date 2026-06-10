//! Pluggable key/value configuration storage.
//!
//! The GUI and the test harness both need to persist small configuration
//! (chosen accounts/relays, UI preferences). `ConfigStorage` is the abstract
//! backend they write through; two implementations ship:
//!
//! - [`mem_config_storage::MemConfigStorage`] — in-memory, for the harness and
//!   native tests.
//! - an IndexedDB implementation in the WASM client crate (`muxsocial-client-wasm`),
//!   for the browser GUI.
//!
//! This is a flat string key/value store — simpler than hashiverse-lib's bucketed,
//! LRU-evictable `ClientStorage`, because configuration is small and never evicts.

use serde::Serialize;
use serde::de::DeserializeOwned;

pub mod mem_config_storage;

/// A backend that persists configuration as string key/value pairs.
///
/// `async_trait` is used (rather than native async-fn-in-trait) so a backend can
/// be held as `Arc<dyn ConfigStorage>`. On wasm the generated futures are `!Send`
/// (browser APIs are single-threaded), hence the `?Send` variant there.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait ConfigStorage: Send + Sync {
    /// Return the value for `key`, or `None` if it is not set.
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>>;
    /// Set `key` to `value`, overwriting any existing value.
    async fn set(&self, key: &str, value: &str) -> anyhow::Result<()>;
    /// Remove `key`. A no-op if it is not set.
    async fn remove(&self, key: &str) -> anyhow::Result<()>;
    /// Return all stored keys (in unspecified order).
    async fn keys(&self) -> anyhow::Result<Vec<String>>;
    /// Remove every key/value pair.
    async fn clear(&self) -> anyhow::Result<()>;
}

/// Store `value` under `key` as JSON.
pub async fn set_json<T: Serialize>(storage: &dyn ConfigStorage, key: &str, value: &T) -> anyhow::Result<()> {
    let json = serde_json::to_string(value)?;
    storage.set(key, &json).await
}

/// Read `key` and deserialize it from JSON, or `None` if it is not set.
pub async fn get_json<T: DeserializeOwned>(storage: &dyn ConfigStorage, key: &str) -> anyhow::Result<Option<T>> {
    match storage.get(key).await? {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

/// Backend-agnostic test suite, run against both `MemConfigStorage` (native) and
/// the IndexedDB backend (in-browser). Compiled into tests and behind the
/// `generic-tests` feature so the wasm crate can reuse it.
#[cfg(any(test, feature = "generic-tests"))]
pub mod tests {
    use super::*;
    use std::sync::Arc;

    /// Exercise the full get/set/overwrite/remove/keys/clear contract.
    pub async fn config_storage_contract(storage: Arc<dyn ConfigStorage>) {
        storage.clear().await.expect("clear");
        assert_eq!(storage.get("missing").await.expect("get"), None);
        assert!(storage.keys().await.expect("keys").is_empty());

        // Set + get.
        storage.set("relay", "wss://relay.damus.io").await.expect("set");
        assert_eq!(storage.get("relay").await.expect("get").as_deref(), Some("wss://relay.damus.io"));

        // Overwrite.
        storage.set("relay", "wss://nos.lol").await.expect("set");
        assert_eq!(storage.get("relay").await.expect("get").as_deref(), Some("wss://nos.lol"));

        // A second key, then keys() returns both.
        storage.set("theme", "dark").await.expect("set");
        let mut keys = storage.keys().await.expect("keys");
        keys.sort();
        assert_eq!(keys, vec!["relay".to_string(), "theme".to_string()]);

        // Remove one.
        storage.remove("relay").await.expect("remove");
        assert_eq!(storage.get("relay").await.expect("get"), None);
        assert_eq!(storage.keys().await.expect("keys"), vec!["theme".to_string()]);

        // Remove missing is a no-op.
        storage.remove("relay").await.expect("remove missing");

        // Clear empties everything.
        storage.clear().await.expect("clear");
        assert!(storage.keys().await.expect("keys").is_empty());
    }

    /// Exercise the `set_json` / `get_json` helpers.
    pub async fn config_storage_json_helpers(storage: Arc<dyn ConfigStorage>) {
        storage.clear().await.expect("clear");
        let relays = vec!["wss://relay.damus.io".to_string(), "wss://nos.lol".to_string()];
        set_json(storage.as_ref(), "relays", &relays).await.expect("set_json");
        let loaded: Option<Vec<String>> = get_json(storage.as_ref(), "relays").await.expect("get_json");
        assert_eq!(loaded, Some(relays));
        let missing: Option<Vec<String>> = get_json(storage.as_ref(), "missing").await.expect("get_json");
        assert_eq!(missing, None);
        storage.clear().await.expect("clear");
    }
}
