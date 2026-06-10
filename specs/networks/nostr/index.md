# nostr

## Library

[`nostr-sdk`](https://crates.io/crates/nostr-sdk) (the rust-nostr project),
pinned at `0.44`. It builds for both native and wasm32 and brings its own
WebSocket relay transport, so nostr does not use the shared HTTP transport.

We pull with `default-features = false`; reading kind-1 notes needs only the
core client + relay pool. (Parsing an `npub` bech32 key needs nostr's nip19
support — pass a hex pubkey to avoid depending on it.)

## Reading posts

`muxsocial-lib/src/sources/nostr.rs`:

1. Parse the author `PublicKey` (hex or `npub`).
2. `Client::default()`, `add_relay(..)` for each relay, `connect()`.
3. `fetch_events(Filter::new().author(pk).kind(Kind::TextNote).limit(n), timeout)`.
4. Map each `Event` → `AggregatedPost` (id hex, pubkey hex, `created_at` seconds → millis, content).

Note content is plain text, so `content_text` is produced by
`crate::html::plain_text_to_html` (HTML-escape + newlines → `<br>`) to match the
HTML the other sources emit. No URL/`nostr:` linkification is done.

Default relays: `wss://relay.damus.io`, `wss://nos.lol`.

## Test identifier

jack — pubkey hex
`82341f882b6eabcd2ba7f1ef90aad961cf074af15b9ef44a09f9d2a8fbfbe6a2`
(`npub1sg6plzptd64u62a878hep2kev88swjh3tw00gjsfl8f237lmu63q0uf63m`).

## WASM / build notes

- Builds for wasm32, but `nostr-sdk` pulls `secp256k1-sys`, whose bundled C is
  compiled by `clang` for the wasm target — so LLVM/clang must be installed and
  on `PATH` for wasm builds (rust-nostr has no pure-Rust crypto backend). Native
  builds use MSVC and don't need it.
- WebSocket relay connections are not subject to CORS the way `fetch` is.
