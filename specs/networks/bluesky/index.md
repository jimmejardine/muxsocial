# Bluesky

## Library

[ATrium](https://github.com/atrium-rs/atrium): `atrium-api` (`0.25`,
`default-features = false`, `namespace-appbsky`) for the typed lexicon, and
`atrium-xrpc` (`0.12`) for the XRPC client traits. Builds native + wasm32.

ATrium abstracts its HTTP transport behind `atrium_xrpc::HttpClient`. Rather than
pull in atrium's own reqwest client, `muxsocial-lib/src/sources/bluesky.rs`
implements `HttpClient`/`XrpcClient` over our shared `HttpTransport`
(`TransportXrpcClient`), so Bluesky rides the same transport as Mastodon and is
wasm-ready.

## Reading posts

Unauthenticated reads from the public AppView `https://public.api.bsky.app`:

1. Build an `XrpcRequest` for `app.bsky.feed.getAuthorFeed` with
   `actor`, `filter = "posts_no_replies"`, `limit`.
2. `send_xrpc::<Parameters, (), Output, Error>(..)`.
3. For each `FeedViewPost`, read `post.uri`, `post.author.handle` /
   `display_name`, and the loosely-typed `post.record` (serialize to JSON, pull
   `text` and `createdAt`).

**Permalink (`post_url`):** built from the post's AT-URI and the author's DID as
`https://bsky.app/profile/{did}/post/{rkey}`, where `rkey` is the last segment of
the `at://…/app.bsky.feed.post/{rkey}` URI. The DID is used (not the handle) because
it is stable across handle changes.

**Media:** `embed_to_media` maps `post.embed` — `images#view` → `Image` (fullsize +
alt), `video#view` → `Video` (HLS `playlist` + `thumbnail` poster), `external#view`
→ `LinkCard`, and `recordWithMedia#view` → its media side. Quote-only `record#view`
embeds carry no media of their own and are skipped for now.

**Pagination:** `BlueskyPager` walks backward with the feed's opaque `cursor`
(`None` for the top page, then the previous response's cursor for older pages);
there is no "since", so `fetch_newer` re-fetches the top.

## Test identifier

`bsky.app` (the official account). Any handle or DID works.

## WASM / CORS notes

- The public AppView sends permissive CORS headers, so unauthenticated reads
  work from a browser.
- atrium-api/atrium-xrpc are pure Rust and wasm-clean; the only wasm build
  prerequisite is the workspace-wide LLVM/clang one (for nostr's C crypto).
