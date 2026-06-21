# Cross-post credentials

How authenticated accounts are modelled, persisted, and protected.

## Account model — `muxsocial-lib/src/posting/account.rs`

- `AuthenticatedAccount { account_id, network, display_label, credential }` — one
  connected identity. `account_id` is a minted GUID, so **multiple accounts per
  network** are first-class (the persisted list just holds several with the same
  `network`); `display_label` disambiguates them in the UI and in `PostResult`.
- `AccountView { account_id, network, display_label }` — the secret-free
  projection sent to the GUI (`list_accounts`). Structurally cannot carry a
  credential.
- `StoredCredential` — an **open enum** (internally tagged on `type`). All secret
  material is an `EncryptedBlob`; non-secret context is in the clear so accounts
  can be listed without unlocking:
  - `HashiverseKeyphrase { encrypted_keyphrase }`
  - `NostrNsec { encrypted_nsec, public_key_bech32 }`
  - `MastodonOAuth { instance_base_url, encrypted_access_token }`
  - `BlueskyOAuth { did, client_id, redirect_uri, encrypted_session }`
  - `sample_secret_blob()` returns one blob per account, used to verify the master
    password on unlock. Returns `None` for any future no-secret (external-signer)
    variant.

### Single source of truth for the pasted secrets

For the pasted-secret networks (Hashiverse keyphrase, nostr nsec), this encrypted
`AccountStore` is the **only** place the secret is persisted. In particular,
Hashiverse is built with an **in-memory key locker**
(`MemKeyLockerManager`, `sources/hashiverse.rs`) rather than hashiverse-lib's
persistent `WasmKeyLockerManager` — so hashiverse-lib writes no key to its own
IndexedDB. On unlock the keyphrase is decrypted and an ephemeral locker is
re-derived for the session (a reload starts again from the encrypted keyphrase),
mirroring how the nostr nsec is held only in memory after decryption. Only the
non-secret Hashiverse post cache (`WasmClientStorage`) stays in IndexedDB.

## Persistence — `account_store.rs`

`AccountStore` is the write-side analogue of `TimelineRegistry`: it owns the
account list and persists every change through `ConfigStorage` (IndexedDB in the
browser). Keys:
- `"accounts"` — JSON array of `AuthenticatedAccount`.
- `"accounts_salt"` — base64 of the store-level Argon2id salt, created once on
  first use and stable thereafter so a later session derives the same key.

## At-rest encryption — `secret_box.rs`

Everything lives in the browser with no server, so the master password is the
only thing protecting the keys.

- KDF: **Argon2id** (pinned params, not the crate default, so a future default
  change can't make existing blobs underivable) over the master password + the
  store-level salt → a 32-byte key (`MasterKey`).
- AEAD: **XChaCha20-Poly1305** with a fresh random 24-byte nonce per secret.
- `EncryptedBlob { nonce_b64, ciphertext_b64 }` — base64 so it round-trips
  through the string KV store; the ciphertext includes the Poly1305 tag.
- Both crates are pure-Rust and build for wasm32 with no clang (unlike nostr's
  secp256k1).

The key is derived **once per session** on unlock and held in memory in
`SharedSourceWriters`; only the master password crosses the JS→WASM boundary.

## Session unlock

`SharedSourceWriters::unlock(master_password, salt, verification_blob?)` derives
the key and, when a `verification_blob` is given (an existing account's secret),
requires it to decrypt — rejecting a wrong password rather than storing a key
that can't read existing credentials. The very first unlock (no accounts yet)
establishes the password. The session starts locked on every reload (nothing
about the key is persisted).

### Honesty note

The protection is at-rest only. A pasted secret (keyphrase, nsec) and OAuth
tokens necessarily pass through JS memory in the browser when entered/used, and
the decrypted key lives in worker memory while unlocked. "Encrypted at rest"
means the persisted IndexedDB records are ciphertext; it does not make the
running browser tab a hardware enclave.

## WASM bridge — `muxsocial-client-wasm`

`MuxsocialClientWasm` owns an `AccountStore` + a `SharedSourceWriters`. Methods:
`list_accounts`, `is_unlocked`, `unlock_secrets(master_password)`,
`add_nostr_account(nsec, master_password)`,
`add_hashiverse_account(keyphrase, label, master_password)`,
`remove_account(account_id)`, and `cross_post(text)`. The OAuth `begin_oauth` /
`complete_oauth` methods are added with the OAuth stages.
</content>
