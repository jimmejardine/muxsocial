//! The authenticated-account model for cross-posting.
//!
//! An [`AuthenticatedAccount`] is one identity the user has connected on a
//! source network. It is keyed by a minted GUID, so **multiple accounts per
//! network** are first-class — the persisted list just holds several with the
//! same `network`, disambiguated in the UI by `display_label`.
//!
//! [`StoredCredential`] is an **open enum**: today it covers pasted secrets
//! (Hashiverse keyphrase, nostr nsec) and OAuth sessions (Mastodon, Bluesky),
//! all with their secret material encrypted at rest via
//! [`secret_box::EncryptedBlob`]. A future external/protected signer (NIP-07 for
//! nostr, or the Hashiverse team's equivalent) is added as a new variant without
//! disturbing the existing ones.

use serde::{Deserialize, Serialize};

use crate::post::SourceNetwork;
use crate::posting::secret_box::EncryptedBlob;

/// One connected identity on a source network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticatedAccount {
    /// Stable, minted GUID — the key the GUI and `remove_account` address.
    pub account_id: String,
    /// Which network this account posts to.
    pub network: SourceNetwork,
    /// Human-friendly label (npub, handle, `@user@instance`, Hashiverse id).
    /// Disambiguates multiple accounts on the same network.
    pub display_label: String,
    /// How to authenticate/sign for this account (secrets encrypted at rest).
    pub credential: StoredCredential,
}

impl AuthenticatedAccount {
    /// Create an account with a freshly minted GUID id.
    pub fn new(network: SourceNetwork, display_label: impl Into<String>, credential: StoredCredential) -> Self {
        Self {
            account_id: uuid::Uuid::new_v4().to_string(),
            network,
            display_label: display_label.into(),
            credential,
        }
    }

    /// The secret-free projection sent across the wasm boundary to the GUI.
    pub fn view(&self) -> AccountView {
        AccountView {
            account_id: self.account_id.clone(),
            network: self.network,
            display_label: self.display_label.clone(),
        }
    }
}

/// The secret-free account projection for the GUI list. Never carries any
/// credential material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountView {
    pub account_id: String,
    pub network: SourceNetwork,
    pub display_label: String,
}

/// How an account authenticates. Secret material is always an
/// [`EncryptedBlob`]; non-secret context (instance URL, DID, public key) is in
/// the clear so the account can be listed without unlocking.
///
/// Internally tagged on `type` so it round-trips through the string KV config
/// store and is easy to switch on. Open by design (see module docs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StoredCredential {
    /// Hashiverse: the keyphrase that unlocks a write identity in the key locker.
    HashiverseKeyphrase { encrypted_keyphrase: EncryptedBlob },
    /// nostr: the pasted nsec private key; `public_key_bech32` is its npub (clear).
    NostrNsec { encrypted_nsec: EncryptedBlob, public_key_bech32: String },
    /// Mastodon OAuth: a bearer access token for `instance_base_url`.
    MastodonOAuth { instance_base_url: String, encrypted_access_token: EncryptedBlob },
    /// Bluesky (ATProto) OAuth: the whole atrium `Session` (DPoP key + token set),
    /// serialized and encrypted as one blob. `client_id`/`redirect_uri` are the
    /// (origin-derived) OAuth client identity, kept so the session can be restored
    /// and refreshed after a reload. `client_id` empty = the localhost dev client.
    BlueskyOAuth {
        did: String,
        client_id: String,
        redirect_uri: String,
        encrypted_session: EncryptedBlob,
    },
}

impl StoredCredential {
    /// A representative encrypted secret, used to verify the master password on
    /// unlock by attempting to decrypt it. Every variant has at least one secret
    /// blob today; returns `None` for any future no-secret (external-signer)
    /// variant.
    pub fn sample_secret_blob(&self) -> Option<&EncryptedBlob> {
        match self {
            StoredCredential::HashiverseKeyphrase { encrypted_keyphrase } => Some(encrypted_keyphrase),
            StoredCredential::NostrNsec { encrypted_nsec, .. } => Some(encrypted_nsec),
            StoredCredential::MastodonOAuth { encrypted_access_token, .. } => Some(encrypted_access_token),
            StoredCredential::BlueskyOAuth { encrypted_session, .. } => Some(encrypted_session),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_blob(tag: &str) -> EncryptedBlob {
        EncryptedBlob {
            nonce_b64: format!("nonce-{tag}"),
            ciphertext_b64: format!("cipher-{tag}"),
        }
    }

    /// Every credential variant must round-trip through JSON unchanged (this is
    /// the persisted form in the `"accounts"` config key).
    #[test]
    fn credential_variants_round_trip_through_json() {
        let variants = vec![
            StoredCredential::HashiverseKeyphrase { encrypted_keyphrase: dummy_blob("hv") },
            StoredCredential::NostrNsec {
                encrypted_nsec: dummy_blob("nostr"),
                public_key_bech32: "npub1example".to_string(),
            },
            StoredCredential::MastodonOAuth {
                instance_base_url: "https://mastodon.social".to_string(),
                encrypted_access_token: dummy_blob("masto"),
            },
            StoredCredential::BlueskyOAuth {
                did: "did:plc:example".to_string(),
                client_id: "https://mux.social/client-metadata-bluesky.json".to_string(),
                redirect_uri: "https://mux.social/oauth-callback.html".to_string(),
                encrypted_session: dummy_blob("bsky-session"),
            },
        ];

        for credential in variants {
            let json = serde_json::to_string(&credential).expect("serialize");
            let parsed: StoredCredential = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed, credential, "credential must round-trip: {json}");
        }
    }

    #[test]
    fn account_mints_a_unique_id_and_view_drops_secrets() {
        let credential = StoredCredential::NostrNsec {
            encrypted_nsec: dummy_blob("n"),
            public_key_bech32: "npub1abc".to_string(),
        };
        let first = AuthenticatedAccount::new(SourceNetwork::Nostr, "npub1abc", credential.clone());
        let second = AuthenticatedAccount::new(SourceNetwork::Nostr, "npub1abc", credential);
        assert_ne!(first.account_id, second.account_id, "each account gets a fresh GUID");

        let view = first.view();
        assert_eq!(view.account_id, first.account_id);
        assert_eq!(view.network, SourceNetwork::Nostr);
        assert_eq!(view.display_label, "npub1abc");
        // The view type structurally cannot carry a credential.
        let view_json = serde_json::to_string(&view).expect("serialize view");
        assert!(!view_json.contains("nsec") && !view_json.contains("cipher"), "view must not leak secrets: {view_json}");
    }
}
