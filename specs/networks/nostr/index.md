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
4. Map each `Event` → `AggregatedPost` (id hex, pubkey hex, `created_at` seconds → millis, content).

Note content is plain text, so `content_html` is produced by
`crate::html::plain_text_to_html` (HTML-escape + newlines → `<br>`) to match the
HTML the other sources emit. Inline body text is not linkified.

**Permalink (`post_url`):** the mapper builds an `nevent` (`Nip19Event`: event id +
author pubkey + the queried relays as hints, capped at two) and forms
`https://njump.me/{nevent}`. Embedding the relays we fetched from keeps the link
resolvable on the right relay; `None` if bech32 encoding fails.

**Pagination:** `NostrPager` bounds the `Filter` by whole-second timestamps —
`.since` (one second past the newest held) for `fetch_newer`, `.until` (one second
before the oldest held) for `fetch_older`. It carries the queried relay URLs so the
permalink hints match.

Default relays: `wss://relay.damus.io`, `wss://nos.lol`.

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
