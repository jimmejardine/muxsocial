//! Browser [`ConfigStorage`] backend on top of IndexedDB.
//!
//! Modeled on hashiverse-client-wasm's `wasm_client_storage.rs`, simplified to a
//! single `config` object store keyed by `"key"` (no metadata store / LRU index —
//! configuration is small and never evicts). The struct holds nothing and reopens
//! the database on each call, which keeps it `Send + Sync` (the non-`Send` work
//! happens inside the async methods).

use std::sync::Arc;

use anyhow::anyhow;
use indexed_db_futures::KeyPath;
use indexed_db_futures::database::Database;
use indexed_db_futures::prelude::*;
use indexed_db_futures::transaction::TransactionMode;
use muxsocial_lib::config_storage::ConfigStorage;
use serde::{Deserialize, Serialize};

const DATABASE_NAME: &str = "muxsocial.config";
const STORE_NAME: &str = "config";

#[derive(Serialize, Deserialize)]
struct Record {
    key: String,
    value: String,
}

/// IndexedDB-backed [`ConfigStorage`] for the GUI.
pub struct IndexedDbConfigStorage {}

/// Open (creating/upgrading if needed) the config database. Every call goes
/// through here so the schema is always present.
async fn open_database() -> anyhow::Result<Database> {
    Database::open(DATABASE_NAME)
        .with_version(1u8)
        .with_on_upgrade_needed(|event, db| {
            let old_version = event.old_version() as u64;
            if old_version < 1 {
                db.create_object_store(STORE_NAME).with_key_path(KeyPath::from("key")).build()?;
            }
            Ok(())
        })
        .build()
        .map_err(|open_error| anyhow!("opening {DATABASE_NAME}: {open_error}"))?
        .await
        .map_err(|open_error| anyhow!("opening {DATABASE_NAME}: {open_error}"))
}

impl IndexedDbConfigStorage {
    pub async fn new() -> anyhow::Result<Arc<Self>> {
        // Open once up front so the store exists before the first read.
        let _database = open_database().await?;
        Ok(Arc::new(Self {}))
    }
}

#[async_trait::async_trait(?Send)]
impl ConfigStorage for IndexedDbConfigStorage {
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        let database = open_database().await?;
        let result: Result<Option<String>, indexed_db_futures::error::Error> = try {
            let transaction = database.transaction(STORE_NAME).with_mode(TransactionMode::Readonly).build()?;
            let object_store = transaction.object_store(STORE_NAME)?;
            let record: Option<Record> = object_store.get(key).serde()?.await?;
            record.map(|record| record.value)
        };
        result.map_err(|storage_error| anyhow!("config get {key:?}: {storage_error}"))
    }

    async fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        let database = open_database().await?;
        let result: Result<(), indexed_db_futures::error::Error> = try {
            let transaction = database.transaction(STORE_NAME).with_mode(TransactionMode::Readwrite).build()?;
            let object_store = transaction.object_store(STORE_NAME)?;
            object_store
                .put(Record {
                    key: key.to_string(),
                    value: value.to_string(),
                })
                .serde()?
                .await?;
            transaction.commit().await?;
        };
        result.map_err(|storage_error| anyhow!("config set {key:?}: {storage_error}"))
    }

    async fn remove(&self, key: &str) -> anyhow::Result<()> {
        let database = open_database().await?;
        let result: Result<(), indexed_db_futures::error::Error> = try {
            let transaction = database.transaction(STORE_NAME).with_mode(TransactionMode::Readwrite).build()?;
            let object_store = transaction.object_store(STORE_NAME)?;
            object_store.delete(key.to_string()).await?;
            transaction.commit().await?;
        };
        result.map_err(|storage_error| anyhow!("config remove {key:?}: {storage_error}"))
    }

    async fn keys(&self) -> anyhow::Result<Vec<String>> {
        let database = open_database().await?;
        let result: Result<Vec<String>, indexed_db_futures::error::Error> = try {
            let transaction = database.transaction(STORE_NAME).with_mode(TransactionMode::Readonly).build()?;
            let object_store = transaction.object_store(STORE_NAME)?;
            let records: Vec<Record> = object_store.get_all().serde()?.await?.collect::<Result<Vec<_>, _>>()?;
            records.into_iter().map(|record| record.key).collect()
        };
        result.map_err(|storage_error| anyhow!("config keys: {storage_error}"))
    }

    async fn clear(&self) -> anyhow::Result<()> {
        let database = open_database().await?;
        let result: Result<(), indexed_db_futures::error::Error> = try {
            let transaction = database.transaction(STORE_NAME).with_mode(TransactionMode::Readwrite).build()?;
            transaction.object_store(STORE_NAME)?.clear()?;
            transaction.commit().await?;
        };
        result.map_err(|storage_error| anyhow!("config clear: {storage_error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use muxsocial_lib::config_storage::tests::{config_storage_contract, config_storage_json_helpers};
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn passes_the_config_storage_contract() {
        config_storage_contract(IndexedDbConfigStorage::new().await.unwrap()).await;
    }

    #[wasm_bindgen_test]
    async fn passes_the_json_helper_contract() {
        config_storage_json_helpers(IndexedDbConfigStorage::new().await.unwrap()).await;
    }
}
