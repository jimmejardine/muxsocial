# Config transfer (import/export)

The "Config" entry in the hamburger menu (`HeaderMenu.tsx`) opens `ConfigDialog.tsx`: the whole
configuration as one editable JSON document in a large monospace textarea, for copy/paste
transfer between machines. There is no file or server involved — the user copies the text on one
machine and pastes it into the same dialog on another, then presses **Apply**.

## The document shape

Three root elements, mirroring where each part of the configuration lives:

```json
{
  "timelines": [ { "id": "…", "name": null, "sources": [{ "network": "Nostr", "id": "npub1…" }], "autopoll": true } ],
  "settings": { "theme": "electric", "language": "en" },
  "accounts": { "accounts": [ { "account_id": "…", "network": "Nostr", "display_label": "npub1…", "credential": { "type": "nostr_nsec", "encrypted_nsec": { "nonce_b64": "…", "ciphertext_b64": "…" }, "public_key_bech32": "npub1…" } } ], "salt_b64": "…" }
}
```

- `"timelines"` — the persisted timeline list, exactly the shape `TimelineRegistry` stores under
  its `"timelines"` key in [config storage](../architecture/config-storage.md) (IndexedDB in the
  browser). Owned and validated by Rust.
- `"settings"` — the GUI-selectable preferences, owned by the GUI and persisted in localStorage
  by their providers: `theme` (a [theme id](theming.md)) and `language` (a
  [locale code](localization.md)). New GUI preferences should be added to this object.
- `"accounts"` — the connected cross-post accounts bundle (`AccountsBundle`): the accounts with
  their **secrets still encrypted**, plus the store-level Argon2id `salt_b64` so the same master
  password decrypts them on the target. The master password itself is never exported. Owned and
  validated by Rust (`AccountStore`). **Optional** — older configs omit it, in which case Apply
  leaves the existing accounts untouched. See [../networks/compose/credentials.md](../networks/compose/credentials.md).

## Split ownership

The split mirrors the architecture, so each layer validates what it owns:

- **Rust** (`TimelineRegistry`, `muxsocial-lib/src/timeline_registry.rs`):
  - `export_timelines_json()` — the timeline list as a JSON string.
  - `import_timelines_json(json)` — parse, validate (non-empty unique timeline ids, known
    networks, non-empty source ids), **replace the whole list all-or-nothing**, persist, and
    return the new snapshot. Serde defaults match `load` (missing `name` → `None`, missing
    `autopoll` → `true`).
  - Both are exposed on `MuxsocialClientWasm`; the wasm import additionally drops all live
    pagination trackers so every timeline rebuilds over its imported sources.
- **Rust** (`AccountStore`, `muxsocial-lib/src/posting/account_store.rs`):
  - `export_json()` — the accounts + salt as an `AccountsBundle` JSON string.
  - `import_json(json)` — persist the salt, **replace the whole account list all-or-nothing**,
    persist, and return the new snapshot. The wasm import additionally **re-locks the session**
    (drops the master key) so the user re-enters the imported master password before posting.
- **TypeScript** (`src/tools/ConfigTransfer.ts`, pure and unit-tested):
  - `compose_config_text(timelines_json, settings, accounts_json?)` — build the pretty-printed
    document; `accounts` is included only when an accounts bundle is given.
  - `parse_config_text(text, valid_theme_ids, valid_language_codes)` — validate the envelope
    (`timelines` array + `settings` object required; `accounts`, if present, must be an object —
    both opaque here) and the settings values; unknown settings keys are ignored for forward
    compatibility. Returns `accounts_json` (`""` when the config omits accounts).

## Dialog behaviour

- The textarea is seeded from the current configuration each time the dialog opens.
- **Copy** puts the textbox contents (including any unsaved edits) on the clipboard, with a
  success/error toast — same pattern as the source-chip address copy in `Timeline.tsx`.
- **Apply** parses/validates the text; the Rust timeline import runs first (all-or-nothing), then
  the accounts import if the config carried an `accounts` root (which re-locks the session), then
  the settings are applied (`setThemeId`, `set_language` — both persist themselves), the app's
  timeline state is replaced with the returned snapshot, a success toast shows, and the dialog
  closes. Any failure shows an error toast and leaves the dialog (and all state) untouched.
- **Revert** reloads the textarea from the current configuration, discarding edits.
