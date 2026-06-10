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

## Test identifier

`bsky.app` (the official account). Any handle or DID works.

## WASM / CORS notes

- The public AppView sends permissive CORS headers, so unauthenticated reads
  work from a browser.
- atrium-api/atrium-xrpc are pure Rust and wasm-clean; the only wasm build
  prerequisite is the workspace-wide LLVM/clang one (for nostr's C crypto).
