//! In-memory [`ConfigStorage`] backend for the test harness and native tests.

use std::collections::HashMap;
use std::sync::Mutex;

use super::ConfigStorage;

/// A non-persistent [`ConfigStorage`] backed by a `HashMap`. The lock is never
/// held across an `.await`, so the futures stay `Send` on native.
#[derive(Default)]
pub struct MemConfigStorage {
    entries: Mutex<HashMap<String, String>>,
}

impl MemConfigStorage {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        // A poisoned lock only happens if another thread panicked mid-write;
        // recover the map rather than propagating the poison.
        self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl ConfigStorage for MemConfigStorage {
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        Ok(self.lock().get(key).cloned())
    }

    async fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.lock().insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn remove(&self, key: &str) -> anyhow::Result<()> {
        self.lock().remove(key);
        Ok(())
    }

    async fn keys(&self) -> anyhow::Result<Vec<String>> {
        Ok(self.lock().keys().cloned().collect())
    }

    async fn clear(&self) -> anyhow::Result<()> {
        self.lock().clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_storage::tests::{config_storage_contract, config_storage_json_helpers};
    use std::sync::Arc;

    #[tokio::test]
    async fn passes_the_config_storage_contract() {
        config_storage_contract(Arc::new(MemConfigStorage::new())).await;
    }

    #[tokio::test]
    async fn passes_the_json_helper_contract() {
        config_storage_json_helpers(Arc::new(MemConfigStorage::new())).await;
    }
}
