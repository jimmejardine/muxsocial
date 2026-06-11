# Mastodon

## Approach

There is no WASM-capable Mastodon Rust library — `mastodon-async`, `megalodon`,
and `elefren` are all tokio+reqwest, native-only. The public REST API is plain
JSON over HTTPS, so `muxsocial-lib/src/sources/mastodon.rs` is our own thin,
read-only client built on the shared `HttpTransport`.

## Reading posts

The caller supplies a single concatenated fediverse handle `@user@instance`
(e.g. `@Gargron@mastodon.social`). `split_fediverse_handle`
(`muxsocial-lib/src/sources/mastodon.rs`) splits it into the instance base URL
(`https://instance`) and local username — that split is internal; everywhere
user-facing uses the `@user@instance` form.

Then, unauthenticated, two calls:

1. `GET /api/v1/accounts/lookup?acct={username}` → account id.
2. `GET /api/v1/accounts/{id}/statuses?limit={n}&exclude_reblogs=true` → statuses.

Each status maps to `AggregatedPost` (`id`, `created_at` RFC3339 → millis,
`content` HTML, `account.acct` / `display_name`).

**Permalink (`post_url`):** the status's own canonical `url` field, deserialized
straight from the API response (absent for rare status types → `None`).

**Media:** `media_attachments` map by `type` — `image` → `Image`, `gifv`/`video` →
`Video` (with `preview_url` as poster); audio/unknown are dropped. A status's link
`card` becomes a single `LinkCard` only when it has no attachments of its own.

**Pagination:** `MastodonPager` uses the statuses endpoint's id cursors — `min_id`
(the newest held id) for `fetch_newer`, `max_id` (the oldest held id) for
`fetch_older` — and caches the resolved `(instance base URL, account id)` pair.

## Test identifier

`@Gargron@mastodon.social`.

## CORS — important

Most Mastodon instances (including mastodon.social) do **not** send permissive
`Access-Control-Allow-Origin` headers on `/api/v1/*`, so these calls will be
**blocked from a browser**. Native (integration test / harness) calls are
unaffected. A proxy will be needed for in-browser Mastodon reads; that strategy
is deferred.
