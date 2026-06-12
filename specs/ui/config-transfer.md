# Config transfer (import/export)

The "Config" entry in the hamburger menu (`HeaderMenu.tsx`) opens `ConfigDialog.tsx`: the whole
configuration as one editable JSON document in a large monospace textarea, for copy/paste
transfer between machines. There is no file or server involved — the user copies the text on one
machine and pastes it into the same dialog on another, then presses **Apply**.

## The document shape

Two root elements, mirroring where each half of the configuration lives:

```json
{
  "timelines": [ { "id": "…", "name": null, "sources": [{ "network": "Nostr", "id": "npub1…" }], "autopoll": true } ],
  "settings": { "theme": "electric", "language": "en" }
}
```

- `"timelines"` — the persisted timeline list, exactly the shape `TimelineRegistry` stores under
  its `"timelines"` key in [config storage](../architecture/config-storage.md) (IndexedDB in the
  browser). Owned and validated by Rust.
- `"settings"` — the GUI-selectable preferences, owned by the GUI and persisted in localStorage
  by their providers: `theme` (a [theme id](theming.md)) and `language` (a
  [locale code](localization.md)). New GUI preferences should be added to this object.

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
- **TypeScript** (`src/tools/ConfigTransfer.ts`, pure and unit-tested):
  - `compose_config_text(timelines_json, settings)` — build the pretty-printed document.
  - `parse_config_text(text, valid_theme_ids, valid_language_codes)` — validate the envelope
    (both roots required; `timelines` must be an array but is otherwise opaque here) and the
    settings values; unknown settings keys are ignored for forward compatibility.

## Dialog behaviour

- The textarea is seeded from the current configuration each time the dialog opens.
- **Copy** puts the textbox contents (including any unsaved edits) on the clipboard, with a
  success/error toast — same pattern as the source-chip address copy in `Timeline.tsx`.
- **Apply** parses/validates the text; the Rust import runs first (all-or-nothing), then the
  settings are applied (`setThemeId`, `set_language` — both persist themselves), the app's
  timeline state is replaced with the returned snapshot, a success toast shows, and the dialog
  closes. Any failure shows an error toast and leaves the dialog (and all state) untouched.
- **Revert** reloads the textarea from the current configuration, discarding edits.
