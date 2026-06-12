# Configuration storage

The GUI and the test harness both persist small key/value **configuration**
(chosen accounts/relays, UI preferences). They write through one trait,
`ConfigStorage`, with a backend chosen per target.

This mirrors the trait-injection pattern of hashiverse-lib's `ClientStorage`, but
is intentionally simpler: a **flat string key/value** store with no buckets, LRU
eviction, or access timestamps — configuration is small and never evicts.

## The trait

`muxsocial-lib/src/config_storage/mod.rs`:

```rust
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait ConfigStorage: Send + Sync {
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>>;
    async fn set(&self, key: &str, value: &str) -> anyhow::Result<()>;
    async fn remove(&self, key: &str) -> anyhow::Result<()>;
    async fn keys(&self) -> anyhow::Result<Vec<String>>;
    async fn clear(&self) -> anyhow::Result<()>;
}
```

- `async_trait` (rather than native async-fn-in-trait) so a backend can be held as
  `Arc<dyn ConfigStorage>`; the `?Send` variant on wasm reflects that browser
  futures are `!Send`.
- Free helpers `set_json` / `get_json` (serde_json) sit alongside the trait for
  typed values.
- A backend-agnostic suite, `config_storage::tests` (behind the `generic-tests`
  feature), validates any backend; both backends run it.

## Backends

| Backend | Crate | Storage | Used by |
|---|---|---|---|
| `MemConfigStorage` | `muxsocial-lib` (`config_storage::mem_config_storage`) | `Mutex<HashMap>` | test harness, native tests |
| `IndexedDbConfigStorage` | `muxsocial-client-wasm` (`indexed_db_config_storage`) | IndexedDB via `indexed_db_futures` 0.6.4 | the browser GUI |

The IndexedDB backend uses a single object store `config` (database
`muxsocial.config`) keyed by `"key"`, holding `{ key, value }` records — following
hashiverse-client-wasm's `wasm_client_storage.rs` API (`Database::open` +
`with_on_upgrade_needed`, `transaction(...).object_store(...).get/put/delete/
get_all/clear`, `.serde()`). The impl is an empty struct that reopens the DB per
call, which keeps it `Send + Sync` while the `!Send` IndexedDB work stays inside
the async methods.

## Status / follow-ups

Delivered: the trait, both backends, and the shared test suite (native runs;
the IndexedDB wasm-bindgen-test is build-verified, needs a headless browser to
run). The timeline list is persisted through it by `TimelineRegistry` (key
`"timelines"`), which also exports/imports that key as JSON for the GUI's
[config-transfer dialog](../ui/config-transfer.md). Not yet done: wiring other
concrete config (network identifiers, prefs) through the trait, and exposing it
over the worker RPC ([worker-rpc.md](worker-rpc.md)) to the TS GUI.
