# Networks

mux.social aggregates posts from four source networks. Each has a client in
`muxsocial-lib/src/sources/` that maps recent posts for a well-known identifier
into the normalized [`AggregatedPost`](#the-aggregated-post-model). Clients expose
both a one-shot async `fetch_recent_posts(...)` and a per-network `SourcePager`
(`fetch_newer`/`fetch_older`) that the timeline engine pages through — see
[`../architecture/timelines.md`](../architecture/timelines.md).

Everything runs in the browser with no backend, so every client must build for
both native (`x86_64`, for `cargo test` and the integration harness) and
`wasm32-unknown-unknown`.

## The aggregated post model

`muxsocial-lib/src/post.rs` defines the lowest-common-denominator post type that
every network maps into:

- `source: SourceNetwork` — `Hashiverse | Nostr | Mastodon | Bluesky`
- `source_post_id: String` — network-native stable id (hex id, AT-URI, event id, status id)
- `author_identifier: String` — the author in the same user-facing form a source is
  added as, so the post header matches the source chip: nostr `npub` (bech32; hex
  fallback), Bluesky handle, Mastodon acct, Hashiverse user id hex
- `author_display_name: Option<String>`
- `created_at_millis: i64` — Unix epoch milliseconds (UTC); `parse_rfc3339_to_epoch_millis` normalizes ISO timestamps
- `content_html: String` — HTML for every source (Mastodon/Hashiverse native HTML;
  Bluesky rendered from text + facets; nostr plain text escaped + wrapped via
  `crate::html::plain_text_to_html`)
- `post_url: Option<String>` — a canonical web permalink to the original post,
  built by each mapper where the native ids allow it (`None` otherwise). The post's
  title bar links to it; the per-network formats are njump `nevent` (nostr),
  `bsky.app/profile/{did}/post/{rkey}` (Bluesky), the status `url` (Mastodon), and
  the `app.hashiverse.com` post route (Hashiverse).
- `media: Vec<PostMedia>` — structured attached media the GUI renders below the
  body: `PostMedia` is an enum (internally tagged on `kind`) of `Image { url, alt }`,
  `Video { url, poster, alt }`, and `LinkCard { url, title, description, thumbnail_url }`.
  Each mapper fills it from native data (see the per-network "Media" notes); empty
  when the source keeps media inline in `content_html` (Hashiverse).

This is deliberately minimal — enough to prove the pull-and-display pipeline.
Richer versioned wire/bundle types come later.

`muxsocial-lib/src/html.rs` holds the shared HTML helpers (`escape_text`,
`escape_attribute`, `plain_text_to_html`) used by the plain-text sources.

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
- Hashiverse: a first-class dependency (no feature gate). Builds natively via a
  local moka patch, and **reads in the browser** via a wasm guest client built from
  `hashiverse-lib`'s own runtime services. Live native test in
  `hashiverse_pull_smoke.rs` runs with a real user id in
  `MUXSOCIAL_HASHIVERSE_TEST_USER_ID`. See [hashiverse/index.md](hashiverse/index.md).
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
