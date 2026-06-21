//! The registry of authenticated cross-post accounts — the write-side analogue
//! of [`TimelineRegistry`](crate::timeline_registry::TimelineRegistry).
//!
//! It owns the persisted list of [`AuthenticatedAccount`]s and the store-level
//! Argon2id salt (see [`secret_box`](crate::posting::secret_box)). The GUI is a
//! pure view: it lists accounts (secret-free [`AccountView`]s) and every mutation
//! is persisted to [`ConfigStorage`] before returning.

use std::sync::Arc;

use base64::Engine;

use crate::config_storage::{ConfigStorage, get_json, set_json};
use crate::posting::account::{AccountView, AuthenticatedAccount};
use crate::posting::secret_box::{ARGON2_SALT_LEN, random_salt};

/// Storage key for the persisted account list.
const ACCOUNTS_KEY: &str = "accounts";
/// Storage key for the store-level Argon2id salt (base64).
const ACCOUNTS_SALT_KEY: &str = "accounts_salt";

/// Owns the authenticated-account list and persists every change.
pub struct AccountStore {
    storage: Arc<dyn ConfigStorage>,
    accounts: Vec<AuthenticatedAccount>,
}

impl AccountStore {
    /// Wrap a storage backend. Call [`load`](Self::load) once to read saved state.
    pub fn new(storage: Arc<dyn ConfigStorage>) -> Self {
        Self { storage, accounts: Vec::new() }
    }

    /// Load the persisted account list (call once at startup).
    pub async fn load(&mut self) -> anyhow::Result<()> {
        self.accounts = get_json::<Vec<AuthenticatedAccount>>(self.storage.as_ref(), ACCOUNTS_KEY).await?.unwrap_or_default();
        Ok(())
    }

    /// The secret-free snapshot for the GUI list.
    pub fn list(&self) -> Vec<AccountView> {
        self.accounts.iter().map(AuthenticatedAccount::view).collect()
    }

    /// The full accounts (with credentials) — used when building posters.
    pub fn accounts(&self) -> &[AuthenticatedAccount] {
        &self.accounts
    }

    /// Whether any persisted account holds an encrypted secret (i.e. unlocking is
    /// required before posting). Today every credential variant does, but this
    /// keeps the call-site honest as future no-secret (external-signer) variants
    /// are added.
    pub fn has_any_accounts(&self) -> bool {
        !self.accounts.is_empty()
    }

    /// Add `account` and persist. Returns the new snapshot.
    pub async fn add(&mut self, account: AuthenticatedAccount) -> anyhow::Result<Vec<AccountView>> {
        self.accounts.push(account);
        self.persist().await?;
        Ok(self.list())
    }

    /// Remove the account addressed by `account_id` and persist. Returns the new
    /// snapshot. A no-op (but still re-persists) if no such account exists.
    pub async fn remove(&mut self, account_id: &str) -> anyhow::Result<Vec<AccountView>> {
        self.accounts.retain(|account| account.account_id != account_id);
        self.persist().await?;
        Ok(self.list())
    }

    /// Replace the credential of the account addressed by `account_id` and
    /// persist (used to re-persist a rotated Bluesky OAuth session). A no-op if no
    /// such account exists.
    pub async fn update_account_credential(&mut self, account_id: &str, credential: crate::posting::account::StoredCredential) -> anyhow::Result<()> {
        if let Some(account) = self.accounts.iter_mut().find(|account| account.account_id == account_id) {
            account.credential = credential;
            self.persist().await?;
        }
        Ok(())
    }

    /// The store-level Argon2id salt, creating and persisting a fresh random one
    /// on first use. Stable thereafter so a later session derives the same key.
    pub async fn ensure_salt(&self) -> anyhow::Result<[u8; ARGON2_SALT_LEN]> {
        if let Some(existing) = self.load_salt().await? {
            return Ok(existing);
        }
        let salt = random_salt()?;
        self.storage.set(ACCOUNTS_SALT_KEY, &base64::engine::general_purpose::STANDARD.encode(salt)).await?;
        Ok(salt)
    }

    /// The persisted salt, if one has been created.
    pub async fn load_salt(&self) -> anyhow::Result<Option<[u8; ARGON2_SALT_LEN]>> {
        let Some(salt_b64) = self.storage.get(ACCOUNTS_SALT_KEY).await? else {
            return Ok(None);
        };
        let salt_bytes = base64::engine::general_purpose::STANDARD.decode(salt_b64).map_err(|decode_error| anyhow::anyhow!("decoding accounts salt: {decode_error}"))?;
        let salt: [u8; ARGON2_SALT_LEN] = salt_bytes.try_into().map_err(|_| anyhow::anyhow!("stored accounts salt has wrong length"))?;
        Ok(Some(salt))
    }

    async fn persist(&self) -> anyhow::Result<()> {
        set_json(self.storage.as_ref(), ACCOUNTS_KEY, &self.accounts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_storage::mem_config_storage::MemConfigStorage;
    use crate::post::SourceNetwork;
    use crate::posting::account::StoredCredential;
    use crate::posting::secret_box::EncryptedBlob;

    fn nostr_account(label: &str) -> AuthenticatedAccount {
        AuthenticatedAccount::new(
            SourceNetwork::Nostr,
            label,
            StoredCredential::NostrNsec {
                encrypted_nsec: EncryptedBlob {
                    nonce_b64: "n".to_string(),
                    ciphertext_b64: "c".to_string(),
                },
                public_key_bech32: label.to_string(),
            },
        )
    }

    #[tokio::test]
    async fn add_remove_persists_and_reloads() {
        let storage: Arc<dyn ConfigStorage> = Arc::new(MemConfigStorage::default());

        let mut store = AccountStore::new(storage.clone());
        store.load().await.expect("load");
        assert!(!store.has_any_accounts());

        let first = nostr_account("npub1aaa");
        let second = nostr_account("npub1bbb");
        let first_id = first.account_id.clone();
        store.add(first).await.expect("add");
        let snapshot = store.add(second).await.expect("add");
        assert_eq!(snapshot.len(), 2);

        // A fresh store over the same backing storage sees the persisted accounts.
        let mut reloaded = AccountStore::new(storage.clone());
        reloaded.load().await.expect("reload");
        assert_eq!(reloaded.list().len(), 2);

        reloaded.remove(&first_id).await.expect("remove");
        assert_eq!(reloaded.list().len(), 1);
        assert_eq!(reloaded.list()[0].display_label, "npub1bbb");
    }

    #[tokio::test]
    async fn salt_is_created_once_and_stable() {
        let storage: Arc<dyn ConfigStorage> = Arc::new(MemConfigStorage::default());
        let store = AccountStore::new(storage);
        assert_eq!(store.load_salt().await.expect("load"), None);
        let first = store.ensure_salt().await.expect("ensure");
        let second = store.ensure_salt().await.expect("ensure");
        assert_eq!(first, second, "salt must be stable once created");
        assert_eq!(store.load_salt().await.expect("load"), Some(first));
    }

    #[tokio::test]
    async fn supports_multiple_accounts_on_the_same_network() {
        let storage: Arc<dyn ConfigStorage> = Arc::new(MemConfigStorage::default());
        let mut store = AccountStore::new(storage);
        store.load().await.expect("load");
        store.add(nostr_account("npub1aaa")).await.expect("add");
        store.add(nostr_account("npub1bbb")).await.expect("add");
        let nostr_accounts: Vec<_> = store.accounts().iter().filter(|account| account.network == SourceNetwork::Nostr).collect();
        assert_eq!(nostr_accounts.len(), 2, "two nostr accounts coexist");
    }
}
