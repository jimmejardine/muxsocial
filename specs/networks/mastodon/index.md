# Mastodon

## Approach

There is no WASM-capable Mastodon Rust library — `mastodon-async`, `megalodon`,
and `elefren` are all tokio+reqwest, native-only. The public REST API is plain
JSON over HTTPS, so `muxsocial-lib/src/sources/mastodon.rs` is our own thin,
read-only client built on the shared `HttpTransport`.

## Reading posts

Unauthenticated, two calls against an instance base URL (e.g.
`https://mastodon.social`):

1. `GET /api/v1/accounts/lookup?acct={acct}` → account id.
2. `GET /api/v1/accounts/{id}/statuses?limit={n}&exclude_reblogs=true` → statuses.

Each status maps to `AggregatedPost` (`id`, `created_at` RFC3339 → millis,
`content` HTML, `account.acct` / `display_name`).

## Test identifier

`@Gargron` on `https://mastodon.social`.

## CORS — important

Most Mastodon instances (including mastodon.social) do **not** send permissive
`Access-Control-Allow-Origin` headers on `/api/v1/*`, so these calls will be
**blocked from a browser**. Native (integration test / harness) calls are
unaffected. A proxy will be needed for in-browser Mastodon reads; that strategy
is deferred.
