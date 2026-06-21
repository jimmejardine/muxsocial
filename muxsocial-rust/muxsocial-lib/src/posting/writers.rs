//! Shared, session-scoped state for cross-posting — the write analogue of
//! [`SharedSourceClients`](crate::timeline::SharedSourceClients).
//!
//! Owns the singleton network clients reused across posts (the lazily-connected
//! nostr publish client; on wasm, a cache of keyphrase-unlocked Hashiverse
//! clients) and the **session master key** derived from the user's master
//! password on unlock. [`cross_post`](SharedSourceWriters::cross_post) fans one
//! [`ComposeRequest`] out to every authenticated account, collecting one
//! [`PostResult`] each and never short-circuiting on a failure.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;

use crate::http::{DefaultHttpTransport, default_http_transport};
use crate::post::SourceNetwork;
use crate::posting::account::{AuthenticatedAccount, StoredCredential};
use crate::posting::account_store::AccountStore;
use crate::posting::network_poster::NetworkPoster;
use crate::posting::oauth::bluesky::{self, BlueskyFlow};
use crate::posting::oauth::{self, BeginOauthResult, PendingOauth};
use crate::posting::secret_box::{EncryptedBlob, MasterKey};
use crate::posting::{ComposeRequest, PostResult, PublishedPostReference, SourcePoster};
use crate::sources::hashiverse::{Client as HashiverseClient, HashiversePoster};
use crate::sources::mastodon::MastodonPoster;
use crate::sources::nostr::{self, Client as NostrClient, KeysEventSigner, NostrPoster};

/// Session-scoped cross-post state. One per logical client (the wasm worker).
pub struct SharedSourceWriters {
    /// Shared HTTP transport for the request/response networks (Mastodon, Bluesky).
    http_transport: DefaultHttpTransport,
    /// Shared, lazily-connected nostr client (one relay pool for every nostr post).
    nostr_client: Option<NostrClient>,
    /// Relays nostr posts are published to (also embedded as permalink hints).
    nostr_relays: Vec<String>,
    /// Per-account authenticated Hashiverse clients, built from each keyphrase and
    /// cached for reuse. wasm32 only (native callers can't build one here).
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    hashiverse_clients: HashMap<String, Arc<HashiverseClient>>,
    /// In-flight OAuth flows, keyed by the id returned from `begin_oauth`.
    pending_oauth: HashMap<String, PendingOauth>,
    /// The key derived from the master password on unlock; `None` while locked.
    master_key: Option<MasterKey>,
}

impl SharedSourceWriters {
    /// A fresh, locked set of writers using the default nostr relays.
    pub fn new() -> Self {
        Self {
            http_transport: default_http_transport(),
            nostr_client: None,
            nostr_relays: nostr::DEFAULT_RELAYS.iter().map(|relay_url| relay_url.to_string()).collect(),
            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            hashiverse_clients: HashMap::new(),
            pending_oauth: HashMap::new(),
            master_key: None,
        }
    }

    /// Begin an OAuth flow for `network` (Mastodon today). `instance_or_handle` is
    /// the instance host (Mastodon); `redirect_uri` is the SPA's OAuth callback
    /// URL (the main thread supplies it). Returns the authorize URL to open and an
    /// id to pass back to [`complete_oauth`](Self::complete_oauth).
    pub async fn begin_oauth(&mut self, network: SourceNetwork, instance_or_handle: &str, redirect_uri: &str, client_id: &str) -> anyhow::Result<BeginOauthResult> {
        match network {
            SourceNetwork::Mastodon => {
                let (pending, authorize_url) = oauth::mastodon::begin(&self.http_transport, instance_or_handle, redirect_uri).await?;
                let oauth_flow_id = uuid::Uuid::new_v4().to_string();
                self.pending_oauth.insert(oauth_flow_id.clone(), PendingOauth::Mastodon(pending));
                Ok(BeginOauthResult { authorize_url, oauth_flow_id })
            }
            SourceNetwork::Bluesky => {
                let session_store = bluesky::new_session_store();
                let client = bluesky::build_client(client_id, redirect_uri, session_store.clone())?;
                let authorize_url = bluesky::authorize(&client, instance_or_handle).await?;
                let oauth_flow_id = uuid::Uuid::new_v4().to_string();
                self.pending_oauth.insert(
                    oauth_flow_id.clone(),
                    PendingOauth::Bluesky(Box::new(BlueskyFlow {
                        client,
                        session_store,
                        client_id: client_id.to_string(),
                        redirect_uri: redirect_uri.to_string(),
                        handle_label: instance_or_handle.trim().to_string(),
                    })),
                );
                Ok(BeginOauthResult { authorize_url, oauth_flow_id })
            }
            other => anyhow::bail!("{other:?} does not use OAuth"),
        }
    }

    /// Complete an OAuth flow: parse the callback `redirect_query`, exchange the
    /// code, and build the (encrypted) account. Requires the session to be
    /// unlocked (the caller unlocks first). The caller persists the returned
    /// account.
    pub async fn complete_oauth(&mut self, oauth_flow_id: &str, redirect_query: &str) -> anyhow::Result<AuthenticatedAccount> {
        let pending = self.pending_oauth.remove(oauth_flow_id).ok_or_else(|| anyhow::anyhow!("unknown or expired OAuth flow"))?;
        match pending {
            PendingOauth::Mastodon(pending) => {
                let authorization_code = oauth::extract_authorization_code(redirect_query, &pending.state)?;
                let (instance_base_url, access_token, display_label) = oauth::mastodon::complete(&self.http_transport, &pending, &authorization_code).await?;
                let encrypted_access_token = self.encrypt_secret(access_token.as_bytes())?;
                Ok(AuthenticatedAccount::new(SourceNetwork::Mastodon, display_label, StoredCredential::MastodonOAuth { instance_base_url, encrypted_access_token }))
            }
            PendingOauth::Bluesky(flow) => {
                // Move the flow out of the box so its fields can be moved into the account.
                let flow = *flow;
                if let Some(error) = oauth::query_value(redirect_query, "error") {
                    anyhow::bail!("Bluesky authorization failed: {error}");
                }
                let code = oauth::query_value(redirect_query, "code").ok_or_else(|| anyhow::anyhow!("Bluesky callback missing authorization code"))?;
                let state = oauth::query_value(redirect_query, "state");
                let iss = oauth::query_value(redirect_query, "iss");
                let (did, session) = bluesky::complete(&flow.client, &flow.session_store, code, state, iss).await?;
                let session_bytes = serde_json::to_vec(&session).context("serializing Bluesky session")?;
                let encrypted_session = self.encrypt_secret(&session_bytes)?;
                let display_label = if flow.handle_label.is_empty() { did.clone() } else { flow.handle_label.clone() };
                Ok(AuthenticatedAccount::new(
                    SourceNetwork::Bluesky,
                    display_label,
                    StoredCredential::BlueskyOAuth {
                        did,
                        client_id: flow.client_id,
                        redirect_uri: flow.redirect_uri,
                        encrypted_session,
                    },
                ))
            }
        }
    }

    /// Whether the master key is available (the user has unlocked this session).
    pub fn is_unlocked(&self) -> bool {
        self.master_key.is_some()
    }

    /// Drop the session master key (re-lock). Used after importing accounts, whose
    /// secrets use the source machine's master password.
    pub fn lock(&mut self) {
        self.master_key = None;
    }

    /// Replace the nostr relay list. Drops the cached client so the next post
    /// reconnects to the new relays.
    pub fn set_relays(&mut self, relays: Vec<String>) {
        self.nostr_relays = relays;
        self.nostr_client = None;
    }

    /// The current nostr relay list.
    pub fn relays(&self) -> &[String] {
        &self.nostr_relays
    }

    /// Derive and hold the session master key from `master_password` + `salt`. If
    /// `verification_blob` is given (an existing account's secret), the derived
    /// key must successfully decrypt it — this rejects a wrong master password
    /// instead of silently storing a key that can't read existing credentials.
    pub fn unlock(&mut self, master_password: &str, salt: &[u8], verification_blob: Option<&EncryptedBlob>) -> anyhow::Result<()> {
        let key = MasterKey::derive(master_password, salt)?;
        if let Some(blob) = verification_blob {
            key.decrypt(blob).map_err(|_| anyhow::anyhow!("incorrect master password"))?;
        }
        self.master_key = Some(key);
        Ok(())
    }

    /// Encrypt `plaintext` (a credential secret) under the session key. Errors if
    /// the session is locked.
    pub fn encrypt_secret(&self, plaintext: &[u8]) -> anyhow::Result<EncryptedBlob> {
        self.master_key()?.encrypt(plaintext)
    }

    fn master_key(&self) -> anyhow::Result<&MasterKey> {
        self.master_key.as_ref().ok_or_else(|| anyhow::anyhow!("account credentials are locked; unlock with the master password first"))
    }

    /// Publish `request` to every account in `accounts`, returning one
    /// [`PostResult`] per account (in the same order). Never short-circuits: a
    /// failure on one account still attempts and reports the rest.
    pub async fn cross_post(&mut self, account_store: &mut AccountStore, accounts: &[AuthenticatedAccount], request: &ComposeRequest) -> Vec<PostResult> {
        let mut results: Vec<PostResult> = Vec::with_capacity(accounts.len());
        for account in accounts {
            let publish_result = self.publish_to_account(account_store, account, request).await;
            results.push(PostResult::from_publish(account.network, account.display_label.clone(), publish_result));
        }
        results
    }

    async fn publish_to_account(&mut self, account_store: &mut AccountStore, account: &AuthenticatedAccount, request: &ComposeRequest) -> anyhow::Result<PublishedPostReference> {
        // Bluesky doesn't go through the NetworkPoster enum (its atrium types are
        // generic-heavy); handle it inline so the session can be restored/rotated.
        if matches!(account.credential, StoredCredential::BlueskyOAuth { .. }) {
            return self.publish_bluesky(account_store, account, request).await;
        }
        let mut poster = self.build_poster_for(account).await?;
        poster.publish_post(request).await
    }

    /// Decrypt the stored Bluesky session, restore it, post, and re-persist the
    /// (possibly token-rotated) session so the next post / a reload still works.
    async fn publish_bluesky(&mut self, account_store: &mut AccountStore, account: &AuthenticatedAccount, request: &ComposeRequest) -> anyhow::Result<PublishedPostReference> {
        let StoredCredential::BlueskyOAuth {
            did,
            client_id,
            redirect_uri,
            encrypted_session,
        } = &account.credential
        else {
            anyhow::bail!("publish_bluesky called for a non-Bluesky account");
        };
        let session_bytes = self.master_key()?.decrypt(encrypted_session)?;
        let session: bluesky::BlueskySession = serde_json::from_slice(&session_bytes).context("decoding stored Bluesky session")?;
        let session_store = bluesky::new_session_store();
        let reference = bluesky::create_text_post(client_id, redirect_uri, did, session, session_store.clone(), &request.text, request.created_at_millis).await?;

        // Re-persist the rotated session (ATProto refresh tokens are single-use).
        if let Some(rotated) = bluesky::read_session(&session_store, did).await {
            if let Ok(rotated_bytes) = serde_json::to_vec(&rotated) {
                if let Ok(new_blob) = self.encrypt_secret(&rotated_bytes) {
                    let updated = StoredCredential::BlueskyOAuth {
                        did: did.to_string(),
                        client_id: client_id.to_string(),
                        redirect_uri: redirect_uri.to_string(),
                        encrypted_session: new_blob,
                    };
                    let _ = account_store.update_account_credential(&account.account_id, updated).await;
                }
            }
        }
        Ok(reference)
    }

    /// Decrypt `account`'s credential and build the matching network poster.
    /// Errors clearly for networks whose posting isn't implemented yet (surfaced
    /// as a per-account failure by [`cross_post`](Self::cross_post)).
    async fn build_poster_for(&mut self, account: &AuthenticatedAccount) -> anyhow::Result<NetworkPoster> {
        match &account.credential {
            StoredCredential::NostrNsec { encrypted_nsec, .. } => {
                let nsec_bytes = self.master_key()?.decrypt(encrypted_nsec)?;
                let nsec = String::from_utf8(nsec_bytes).context("decrypted nostr secret is not valid UTF-8")?;
                let signer = KeysEventSigner::from_secret_key(&nsec)?;
                let nostr_client = self.shared_nostr_client().await?;
                Ok(NetworkPoster::Nostr(Box::new(NostrPoster::new(nostr_client, signer, self.nostr_relays.clone()))))
            }
            StoredCredential::HashiverseKeyphrase { encrypted_keyphrase } => {
                let keyphrase_bytes = self.master_key()?.decrypt(encrypted_keyphrase)?;
                let keyphrase = String::from_utf8(keyphrase_bytes).context("decrypted hashiverse keyphrase is not valid UTF-8")?;
                let hashiverse_client = self.authenticated_hashiverse_client(&account.account_id, &keyphrase).await?;
                Ok(NetworkPoster::Hashiverse(HashiversePoster::new(hashiverse_client)))
            }
            StoredCredential::MastodonOAuth { instance_base_url, encrypted_access_token } => {
                let token_bytes = self.master_key()?.decrypt(encrypted_access_token)?;
                let access_token = String::from_utf8(token_bytes).context("decrypted Mastodon token is not valid UTF-8")?;
                Ok(NetworkPoster::Mastodon(MastodonPoster::new(self.http_transport.clone(), instance_base_url.clone(), access_token)))
            }
            StoredCredential::BlueskyOAuth { .. } => anyhow::bail!("Bluesky posting is not implemented yet"),
        }
    }

    /// Connect the shared nostr client once; hand out cheap clones (shared pool).
    async fn shared_nostr_client(&mut self) -> anyhow::Result<NostrClient> {
        if self.nostr_client.is_none() {
            let relay_urls: Vec<&str> = self.nostr_relays.iter().map(String::as_str).collect();
            self.nostr_client = Some(nostr::connect_client(&relay_urls).await?);
        }
        Ok(self.nostr_client.as_ref().expect("nostr client just connected").clone())
    }

    /// The authenticated Hashiverse client for `account_id`, built from
    /// `keyphrase` on first use and cached. wasm32 only.
    async fn authenticated_hashiverse_client(&mut self, account_id: &str, keyphrase: &str) -> anyhow::Result<Arc<HashiverseClient>> {
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            if let Some(existing) = self.hashiverse_clients.get(account_id) {
                return Ok(existing.clone());
            }
            let hashiverse_client = crate::sources::hashiverse::build_authenticated_client(keyphrase).await?;
            self.hashiverse_clients.insert(account_id.to_string(), hashiverse_client.clone());
            Ok(hashiverse_client)
        }
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            let _ = (account_id, keyphrase);
            anyhow::bail!("Hashiverse posting is only supported in the browser client")
        }
    }
}

impl Default for SharedSourceWriters {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_storage::ConfigStorage;
    use crate::config_storage::mem_config_storage::MemConfigStorage;
    use crate::post::SourceNetwork;

    /// Unlocking with the same master password used to encrypt round-trips; a
    /// different password is rejected by the verification-blob check.
    #[tokio::test]
    async fn unlock_round_trips_and_rejects_wrong_password() {
        let storage: Arc<dyn ConfigStorage> = Arc::new(MemConfigStorage::default());
        let account_store = AccountStore::new(storage);
        let salt = account_store.ensure_salt().await.expect("salt");

        // Establish the password and encrypt a secret with it.
        let mut writers = SharedSourceWriters::new();
        writers.unlock("hunter2", &salt, None).expect("first unlock establishes password");
        let blob = writers.encrypt_secret(b"nsec1secret").expect("encrypt");

        // A fresh session with the right password can decrypt; a wrong one cannot.
        let mut good = SharedSourceWriters::new();
        good.unlock("hunter2", &salt, Some(&blob)).expect("right password unlocks");
        assert!(good.is_unlocked());

        let mut bad = SharedSourceWriters::new();
        assert!(bad.unlock("WRONG", &salt, Some(&blob)).is_err(), "wrong password must be rejected");
        assert!(!bad.is_unlocked());
    }

    /// Encrypting a secret while locked is an error (no master key yet).
    #[test]
    fn encrypt_while_locked_errors() {
        let writers = SharedSourceWriters::new();
        assert!(writers.encrypt_secret(b"secret").is_err());
    }

    /// Cross-posting yields one result per account and never short-circuits on a
    /// failure. Uses two Hashiverse accounts: the authenticated client is
    /// wasm-only, so on native they fail deterministically at poster-build time
    /// (no network), exercising the fan-out and multiple-accounts-on-one-network.
    #[tokio::test]
    async fn cross_post_reports_failures_without_short_circuiting() {
        let storage: Arc<dyn ConfigStorage> = Arc::new(MemConfigStorage::default());
        let mut account_store = AccountStore::new(storage);
        let salt = account_store.ensure_salt().await.expect("salt");

        let mut writers = SharedSourceWriters::new();
        writers.unlock("pw", &salt, None).expect("unlock");

        let make_hashiverse = |writers: &SharedSourceWriters, label: &str| {
            AuthenticatedAccount::new(
                SourceNetwork::Hashiverse,
                label,
                StoredCredential::HashiverseKeyphrase {
                    encrypted_keyphrase: writers.encrypt_secret(b"keyphrase").expect("enc"),
                },
            )
        };
        let accounts = vec![make_hashiverse(&writers, "hv-a"), make_hashiverse(&writers, "hv-b")];

        let results = writers.cross_post(&mut account_store, &accounts, &ComposeRequest::new("hi")).await;
        assert_eq!(results.len(), 2, "one result per account, none dropped");
        assert_eq!(results[0].account_label, "hv-a");
        assert_eq!(results[1].account_label, "hv-b");
        assert!(results.iter().all(|result| matches!(result.outcome, crate::posting::PostOutcome::Failed { .. })));
    }
}
