use std::collections::HashMap;
use std::sync::Arc;

use muxsocial_lib::config_storage::ConfigStorage;
use muxsocial_lib::greeting::compose_greeting_message;
use muxsocial_lib::post::{AggregatedPost, SourceNetwork};
use muxsocial_lib::posting::account::{AccountView, AuthenticatedAccount, StoredCredential};
use muxsocial_lib::posting::{AccountStore, ComposeRequest, PostResult, SharedSourceWriters};
use muxsocial_lib::sources::nostr::{EventSigner, KeysEventSigner};
use muxsocial_lib::timeline::{MultiTimeline, NetworkPager, SharedSourceClients, Source};
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
    /// The authenticated cross-post accounts (persisted; the single source of
    /// truth for "which identities can post").
    account_store: AccountStore,
    /// Session-scoped write state: the master key (after unlock) and the shared
    /// network publish clients. Not persisted — a reload starts locked.
    writers: SharedSourceWriters,
}

#[wasm_bindgen]
impl MuxsocialClientWasm {
    /// Construct the client: open IndexedDB once and load the saved timelines.
    pub async fn create_new() -> Result<MuxsocialClientWasm, JsValue> {
        let storage: Arc<dyn ConfigStorage> = IndexedDbConfigStorage::new().await.map_err(anyhow_to_js)?;
        let mut timeline_registry = TimelineRegistry::new(storage.clone());
        timeline_registry.load().await.map_err(anyhow_to_js)?;
        let mut account_store = AccountStore::new(storage);
        account_store.load().await.map_err(anyhow_to_js)?;
        Ok(MuxsocialClientWasm {
            timeline_registry,
            shared_clients: SharedSourceClients::new(),
            trackers: HashMap::new(),
            account_store,
            writers: SharedSourceWriters::new(),
        })
    }

    pub async fn compose_greeting(&self, recipient_name: String) -> String {
        compose_greeting_message(&recipient_name)
    }

    /// The app version — the workspace `Cargo.toml` version, baked in at build
    /// time (CI rewrites it from the release tag before building).
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
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

    /// Remove the source (`network` + `source_id`) from the timeline addressed by
    /// `id`. Returns the new snapshot.
    pub async fn remove_source_from_timeline(&mut self, id: String, network: String, source_id: String) -> Result<JsValue, JsValue> {
        let network = SourceNetwork::from_name(&network).ok_or_else(|| JsValue::from_str(&format!("unknown network {network:?}")))?;
        let snapshot = self.timeline_registry.remove_source_from_timeline(&id, &Source::new(network, source_id)).await.map_err(anyhow_to_js)?;
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

    /// Set whether the timeline addressed by `id` auto-polls on the recurring
    /// tick. Returns the new snapshot.
    pub async fn set_timeline_autopoll(&mut self, id: String, autopoll: bool) -> Result<JsValue, JsValue> {
        let snapshot = self.timeline_registry.set_timeline_autopoll(&id, autopoll).await.map_err(anyhow_to_js)?;
        snapshot_to_js(&snapshot)
    }

    /// The whole timeline list as a JSON string — the `"timelines"` root element
    /// of the GUI's config-transfer dialog.
    pub fn export_timelines_json(&self) -> Result<String, JsValue> {
        self.timeline_registry.export_timelines_json().map_err(anyhow_to_js)
    }

    /// Replace the whole timeline list with one parsed from `timelines_json` (the
    /// export shape). All-or-nothing: on any validation error nothing changes.
    /// Returns the new snapshot.
    pub async fn import_timelines_json(&mut self, timelines_json: String) -> Result<JsValue, JsValue> {
        let snapshot = self.timeline_registry.import_timelines_json(&timelines_json).await.map_err(anyhow_to_js)?;
        // Any timeline's sources may have changed; drop all live pagination state
        // so each timeline rebuilds over its imported sources.
        self.trackers.clear();
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

    // --- Cross-posting (authenticated accounts + compose) ---------------------

    /// The authenticated cross-post accounts, secret-free, for the GUI list.
    pub async fn list_accounts(&self) -> Result<JsValue, JsValue> {
        accounts_to_js(&self.account_store.list())
    }

    /// Whether the credential store is unlocked this session (the master password
    /// has been entered). The GUI gates posting on this.
    pub fn is_unlocked(&self) -> bool {
        self.writers.is_unlocked()
    }

    /// Derive and hold the session master key from `master_password`. On the very
    /// first unlock (no accounts yet) this establishes the password; otherwise it
    /// is verified against an existing credential and rejected if wrong. Returns
    /// the account snapshot.
    pub async fn unlock_secrets(&mut self, master_password: String) -> Result<JsValue, JsValue> {
        self.ensure_unlocked(&master_password).await.map_err(anyhow_to_js)?;
        accounts_to_js(&self.account_store.list())
    }

    /// Add a nostr account from a pasted `nsec` (bech32 or hex). The npub is
    /// derived for the label; the nsec is encrypted under the session key.
    /// `master_password` unlocks the session if it is not already unlocked.
    pub async fn add_nostr_account(&mut self, nsec: String, master_password: String) -> Result<JsValue, JsValue> {
        self.ensure_unlocked(&master_password).await.map_err(anyhow_to_js)?;
        let signer = KeysEventSigner::from_secret_key(&nsec).map_err(anyhow_to_js)?;
        let public_key_bech32 = signer.public_key_bech32().map_err(anyhow_to_js)?;
        let encrypted_nsec = self.writers.encrypt_secret(nsec.as_bytes()).map_err(anyhow_to_js)?;
        let account = AuthenticatedAccount::new(
            SourceNetwork::Nostr,
            public_key_bech32.clone(),
            StoredCredential::NostrNsec { encrypted_nsec, public_key_bech32 },
        );
        let snapshot = self.account_store.add(account).await.map_err(anyhow_to_js)?;
        accounts_to_js(&snapshot)
    }

    /// Add a Hashiverse account from a pasted `keyphrase`. `label` is the GUI
    /// display name (defaults to "Hashiverse" if blank); the keyphrase is
    /// encrypted under the session key. `master_password` unlocks if needed.
    pub async fn add_hashiverse_account(&mut self, keyphrase: String, label: String, master_password: String) -> Result<JsValue, JsValue> {
        self.ensure_unlocked(&master_password).await.map_err(anyhow_to_js)?;
        let encrypted_keyphrase = self.writers.encrypt_secret(keyphrase.as_bytes()).map_err(anyhow_to_js)?;
        let display_label = if label.trim().is_empty() { "Hashiverse".to_string() } else { label };
        let account = AuthenticatedAccount::new(SourceNetwork::Hashiverse, display_label, StoredCredential::HashiverseKeyphrase { encrypted_keyphrase });
        let snapshot = self.account_store.add(account).await.map_err(anyhow_to_js)?;
        accounts_to_js(&snapshot)
    }

    /// Begin an OAuth flow for `network` ("Mastodon" / "Bluesky").
    /// `instance_or_handle` is the Mastodon instance host or Bluesky handle (empty
    /// for the Bluesky entryway). `redirect_uri` is the SPA's OAuth callback URL.
    /// `client_id` is the Bluesky hosted client-metadata URL (empty → the
    /// localhost dev client; ignored by Mastodon). Returns `{ authorize_url,
    /// oauth_flow_id }` — the GUI opens `authorize_url` in a popup and passes
    /// `oauth_flow_id` + the callback query to `complete_oauth`.
    pub async fn begin_oauth(&mut self, network: String, instance_or_handle: String, redirect_uri: String, client_id: String) -> Result<JsValue, JsValue> {
        let network = SourceNetwork::from_name(&network).ok_or_else(|| JsValue::from_str(&format!("unknown network {network:?}")))?;
        let result = self.writers.begin_oauth(network, &instance_or_handle, &redirect_uri, &client_id).await.map_err(anyhow_to_js)?;
        serde_wasm_bindgen::to_value(&result).map_err(|serialize_error| JsValue::from_str(&format!("serializing oauth result: {serialize_error}")))
    }

    /// Complete an OAuth flow with the callback `redirect_query` (the popup's
    /// `location.search`). Unlocks with `master_password` if needed, exchanges the
    /// code, encrypts and stores the account. Returns the new account snapshot.
    pub async fn complete_oauth(&mut self, oauth_flow_id: String, redirect_query: String, master_password: String) -> Result<JsValue, JsValue> {
        self.ensure_unlocked(&master_password).await.map_err(anyhow_to_js)?;
        let account = self.writers.complete_oauth(&oauth_flow_id, &redirect_query).await.map_err(anyhow_to_js)?;
        let snapshot = self.account_store.add(account).await.map_err(anyhow_to_js)?;
        accounts_to_js(&snapshot)
    }

    /// Remove the account addressed by `account_id`. Returns the new snapshot.
    pub async fn remove_account(&mut self, account_id: String) -> Result<JsValue, JsValue> {
        let snapshot = self.account_store.remove(&account_id).await.map_err(anyhow_to_js)?;
        accounts_to_js(&snapshot)
    }

    /// Publish `text` to every authenticated account at once. Returns one
    /// `PostResult` per account (success or failure); never short-circuits.
    /// Requires the session to be unlocked.
    pub async fn cross_post(&mut self, text: String) -> Result<JsValue, JsValue> {
        if !self.writers.is_unlocked() {
            return Err(JsValue::from_str("locked: unlock with the master password before posting"));
        }
        // Stamp the post time here (the lib's chrono has no `clock` on wasm); only
        // Bluesky uses it, for the record `createdAt`.
        let request = ComposeRequest {
            text,
            created_at_millis: js_sys::Date::now() as i64,
        };
        let accounts = self.account_store.accounts().to_vec();
        let results = self.writers.cross_post(&mut self.account_store, &accounts, &request).await;
        results_to_js(&results)
    }

    /// Unlock the session if not already unlocked: get/create the store salt, and
    /// verify `master_password` against an existing credential (or establish it if
    /// there are none yet).
    async fn ensure_unlocked(&mut self, master_password: &str) -> anyhow::Result<()> {
        if self.writers.is_unlocked() {
            return Ok(());
        }
        let salt = self.account_store.ensure_salt().await?;
        let verification_blob = self.account_store.accounts().first().and_then(|account| account.credential.sample_secret_blob()).cloned();
        self.writers.unlock(master_password, &salt, verification_blob.as_ref())
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

fn accounts_to_js(accounts: &[AccountView]) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(accounts).map_err(|serialize_error| JsValue::from_str(&format!("serializing accounts: {serialize_error}")))
}

fn results_to_js(results: &[PostResult]) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(results).map_err(|serialize_error| JsValue::from_str(&format!("serializing post results: {serialize_error}")))
}

fn anyhow_to_js(error: anyhow::Error) -> JsValue {
    JsValue::from_str(&format!("{error:#}"))
}
