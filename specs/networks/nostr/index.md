# nostr

## Library

[`nostr-sdk`](https://crates.io/crates/nostr-sdk) (the rust-nostr project),
pinned at `0.44`. It builds for both native and wasm32 and brings its own
WebSocket relay transport, so nostr does not use the shared HTTP transport.

We pull with `default-features = false`; reading kind-1 notes needs only the core
client + relay pool. nostr's nip19 (bech32) support is used to parse `npub` keys
and to encode each post's permalink as an `nevent` (see below).

## Reading posts

`muxsocial-lib/src/sources/nostr.rs`:

1. Parse the author `PublicKey` (hex or `npub`).
2. `Client::default()`, `add_relay(..)` for each relay, `connect()`.
3. `fetch_events(Filter::new().author(pk).kind(Kind::TextNote).limit(n), timeout)`.
4. Map each `Event` → `AggregatedPost` (id hex; author as the `npub` bech32 form —
   matching what the user added as the source — with hex as a fallback if encoding
   fails; `created_at` seconds → millis; content).

Note content is plain text, so `content_html` is produced by
`crate::html::plain_text_to_html` (HTML-escape + newlines → `<br>`) to match the
HTML the other sources emit; URLs in the text are linkified (see Media below).

**Permalink (`post_url`):** the mapper builds an `nevent` (`Nip19Event`: event id +
author pubkey + the queried relays as hints, capped at two) and forms
`https://njump.me/{nevent}`. Embedding the relays we fetched from keeps the link
resolvable on the right relay; `None` if bech32 encoding fails.

**Media:** notes have no structured media, so `render_nostr_content` scans the
text — an `http(s)` URL ending in an image extension (`jpg/jpeg/png/gif/webp/avif`)
becomes a `PostMedia::Image` (and is dropped from the text), and any other URL is
linkified into `<a href>` in `content_html`.

**Pagination:** `NostrPager` bounds the `Filter` by whole-second timestamps —
`.since` (one second past the newest held) for `fetch_newer`, `.until` (one second
before the oldest held) for `fetch_older`. It carries the queried relay URLs so the
permalink hints match.

Default relays: `wss://relay.damus.io`, `wss://nos.lol`.

## Configurable relays

The relay list is **user-configurable** and shared by reading and posting. It is
edited in the GUI as a single `;`-separated string (the Relays dialog, opened from
the hamburger; see [../../ui/compose.md](../../ui/compose.md)) and persisted in
Rust `ConfigStorage` under the `"nostr_relays"` key (JSON `Vec<String>`), defaulting
to the two relays above until changed.

- Helpers `parse_relays_text` / `relays_to_text` (`sources/nostr.rs`) convert
  between the `;`-string and the list.
- Both builders expose `set_relays(Vec<String>)` (which drops the cached
  `nostr_client` so it reconnects to the new relays) and `relays()`:
  `SharedSourceClients` (read, `timeline/builder.rs`) and `SharedSourceWriters`
  (write, `posting/writers.rs`) — they hold independent clients but the same list.
- The wasm bridge (`MuxsocialClientWasm`) loads the saved list on startup and
  applies it to both builders; `get_nostr_relays()` / `set_nostr_relays(text)`
  read/update it. `set_nostr_relays` persists, applies to both builders, and
  clears live read trackers so pagers rebuild over the reconnected client — the
  change takes effect at runtime (no reload). Empty input falls back to the
  defaults (nostr needs at least one relay).

## Test identifier

The integration harness pages a baked-in author,
`NOSTR_AUTHOR_NPUB = npub1wmr34t36fy03m8hvgl96zl3znndyzyaqhwmwdtshwmtkg03fetaqhjg240`.
Any author pubkey (hex or `npub`) works.

## WASM / build notes

- Builds for wasm32, but `nostr-sdk` pulls `secp256k1-sys`, whose bundled C is
  compiled by `clang` for the wasm target — so LLVM/clang must be installed and
  on `PATH` for wasm builds (rust-nostr has no pure-Rust crypto backend). Native
  builds use MSVC and don't need it.
- WebSocket relay connections are not subject to CORS the way `fetch` is.
