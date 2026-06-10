# Networks

mux.social aggregates posts from four source networks. Each has a client in
`muxsocial-lib/src/sources/` exposing an async `fetch_recent_posts(...)` that
pulls recent posts for a well-known identifier and maps them into the normalized
[`AggregatedPost`](#the-aggregated-post-model).

Everything runs in the browser with no backend, so every client must build for
both native (`x86_64`, for `cargo test` and the integration harness) and
`wasm32-unknown-unknown`.

## The aggregated post model

`muxsocial-lib/src/post.rs` defines the lowest-common-denominator post type that
every network maps into:

- `source: SourceNetwork` — `Hashiverse | Nostr | Mastodon | Bluesky`
- `source_post_id: String` — network-native stable id (hex id, AT-URI, event id, status id)
- `author_identifier: String` — pubkey hex, handle, acct, or DID
- `author_display_name: Option<String>`
- `created_at_millis: i64` — Unix epoch milliseconds (UTC); `parse_rfc3339_to_epoch_millis` normalizes ISO timestamps
- `content_text: String` — plain text or HTML depending on source

This is deliberately minimal — enough to prove the pull-and-display pipeline.
Richer versioned wire/bundle types come later.

## Shared HTTP transport

`muxsocial-lib/src/http/` provides a tiny cross-platform HTTP capability behind
the `HttpTransport` trait, with a per-target `DefaultHttpTransport`:

- native → `reqwest` (rustls)
- wasm → `gloo-net` (browser Fetch API)

Mastodon (our own REST client) and Bluesky (the atrium XRPC adapter) go through
it. nostr and Hashiverse do **not** — they use their libraries' own transports
(WebSocket relays / DHT respectively).

## CORS reality

In the browser every request is subject to CORS. The public Bluesky AppView
sends permissive CORS headers; most Mastodon instances do not and will need a
proxy. nostr (WebSocket) and Hashiverse (its own transport) are not affected the
same way. A proxy strategy for CORS-hostile networks is deferred.

## Library choices

| Network | Library / approach | Transport | Identifier |
|---|---|---|---|
| [nostr](nostr/index.md) | `nostr-sdk` (rust-nostr) | WebSocket relays | pubkey (hex or `npub`) |
| [Bluesky](bluesky/index.md) | ATrium (`atrium-api` + `atrium-xrpc`) over our transport | XRPC/HTTP | handle or DID |
| [Mastodon](mastodon/index.md) | our own thin REST client | HTTP | `acct` + instance URL |
| [Hashiverse](hashiverse/index.md) | `hashiverse-lib` | DHT + proof-of-work | 32-byte `Id` (hex) |

## Status (first feasibility step)

- nostr, Bluesky, Mastodon: implemented; live native pull-tests pass in
  `muxsocial-integration-tests/tests/network_pull_smoke.rs`.
- Hashiverse: a first-class dependency (no feature gate); builds natively via a
  local moka patch. Live test in `hashiverse_pull_smoke.rs` runs with a real user
  id in `MUXSOCIAL_HASHIVERSE_TEST_USER_ID`. See
  [hashiverse/index.md](hashiverse/index.md).
- WASM compile gate: **passes**. `muxsocial-lib` (all four networks) and
  `muxsocial-client-wasm` build for `wasm32-unknown-unknown`.

### WASM build prerequisite — LLVM/clang

nostr's `secp256k1`/`secp256k1-sys` (and, under the hashiverse feature,
`blake3`) compile bundled C via the `cc` crate, which needs `clang` to target
wasm32. rust-nostr has no pure-Rust crypto backend, so clang is required for any
wasm build. Native (MSVC) builds don't need it. Install LLVM and ensure
`clang`/`llvm-ar` are on `PATH` (e.g. `C:\Program Files\LLVM\bin`) before running
`cargo build --target wasm32-unknown-unknown`.

## Files

- [nostr/index.md](nostr/index.md)
- [bluesky/index.md](bluesky/index.md)
- [mastodon/index.md](mastodon/index.md)
- [hashiverse/index.md](hashiverse/index.md)
