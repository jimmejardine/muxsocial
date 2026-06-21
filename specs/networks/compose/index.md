# Compose & cross-post

The write side. A user connects their accounts on each network, types a message
in the compose dialog, and on send mux.social **cross-posts it to every
authenticated account at once**, reporting a per-account success/failure result.
Text-only in v1.

This mirrors the read side: reading hides per-network I/O behind
`SourcePager` (`muxsocial-lib/src/timeline`); writing hides it behind
`SourcePoster` (`muxsocial-lib/src/posting`).

## Module layout — `muxsocial-lib/src/posting/`

- `mod.rs` — the seam and shared types:
  - `ComposeRequest { text }` — a composed message (text-only in v1).
  - `SourcePoster` trait — `async fn publish_post(&mut self, &ComposeRequest) -> Result<PublishedPostReference>`. The write analogue of `SourcePager`; static dispatch only.
  - `PublishedPostReference { native_post_id, post_url }`.
  - `PostResult { network, account_label, outcome }` and `PostOutcome` (serde-tagged on `status`: `published { post_url, native_post_id }` / `failed { error_message }`) — the per-account result serialized to the GUI. `PostResult::from_publish` maps a publish `Result` into one.
- `network_poster.rs` — `NetworkPoster` enum (the write analogue of `NetworkPager`) dispatching `SourcePoster` over the concrete per-network posters. Variants are added per stage; building a poster for an unimplemented network errors (surfaced as a per-account failure, never a panic).
- `account.rs` / `account_store.rs` — the account model and its persistence; see [credentials.md](credentials.md).
- `secret_box.rs` — at-rest credential encryption; see [credentials.md](credentials.md).
- `writers.rs` — `SharedSourceWriters`, the session-scoped state (master key + singleton publish clients) and the `cross_post` fan-out.

## Per-network posters

Each lives beside its reader in `muxsocial-lib/src/sources/`:

| Network | Poster | How it signs / authenticates | Status |
|---|---|---|---|
| Hashiverse | `HashiversePoster` | authenticated `HashiverseClient` from the user's keyphrase (key locker); `submit_post` signs + does PoW in-worker | implemented |
| nostr | `NostrPoster<S: EventSigner>` | `EventSigner` seam; v1 `KeysEventSigner` from a pasted nsec signs a kind-1 note locally | implemented |
| Mastodon | `MastodonPoster` | OAuth bearer token → `POST /api/v1/statuses` | implemented |
| Bluesky | `posting::oauth::bluesky` (inline, not in the enum) | OAuth + DPoP via `atrium-oauth` → `com.atproto.repo.createRecord` (`app.bsky.feed.post`) | implemented |

nostr (WebSocket relays) and Hashiverse (its own transport) bypass the shared
HTTP transport, exactly as on the read side; Mastodon and Bluesky go through it.

## The fan-out — `SharedSourceWriters::cross_post`

`cross_post(&[AuthenticatedAccount], &ComposeRequest) -> Vec<PostResult>` builds a
poster per account, calls `publish_post`, and collects one `PostResult` each. It
**never short-circuits**: one account's failure still attempts and reports the
rest. Multiple accounts on the same network are first-class (each is a separate
`AuthenticatedAccount`), so a cross-post can hit e.g. two Mastodon accounts;
results are disambiguated by `account_label`.

`SharedSourceWriters` also owns the singleton publish clients reused across posts
(the lazily-connected nostr client; on wasm, a cache of keyphrase-unlocked
Hashiverse clients keyed by account id) and the **session master key** (see
[credentials.md](credentials.md)).

## Signing stays in the worker (no signer bridge in v1)

The WASM client runs in a Web Worker. v1 keeps all signing **in the worker**:
nostr signs with the pasted key, Hashiverse signs via its key locker, and (when
built) Bluesky DPoP proofs are signed by a pure-Rust `p256` key held in the
worker — so no per-request main-thread signer bridge is needed. The only
main-thread step is the interactive OAuth popup (Mastodon/Bluesky). A future
external/protected signer (NIP-07 for nostr; the Hashiverse team's eventual
equivalent) is purely additive: `StoredCredential` is an open enum and posters
take a signer abstraction, so such a signer slots in as a new variant — and may
reintroduce a worker↔main bridge then — without disturbing v1.

## OAuth (Mastodon & Bluesky)

Worker-driven: the worker builds the authorize URL and performs all token work;
the main thread only opens the popup and returns the redirect `code`.
- Mastodon: dynamic app registration (`POST /api/v1/apps`) + PKCE + token
  exchange — fully client-side, no hosted metadata. Most instances are
  CORS-permissive for the API and OAuth endpoints, and `/oauth/authorize` is a
  navigation (CORS-exempt).
- Bluesky (ATProto OAuth): handle→PDS resolution, a hosted
  `client-metadata-bluesky.json` (a static file served beside the SPA — named
  with the network suffix so a future `client-metadata-mastodon.json` can sit
  beside it), PAR, DPoP-bound tokens + refresh.

### Bluesky implementation

Built on `atrium-oauth` 0.1.7 (`default-features = false`, driven over our own
`HttpClient`) in `posting/oauth/bluesky.rs`. The library owns PAR + DPoP + PKCE +
refresh. We provide: a `Send + Sync` `OwnedHttpClient` over the shared transport;
`CommonDidResolver` (PLC directory) + `AppViewHandleResolver` (HTTP `resolveHandle`
— no browser DNS); and the client metadata. `BlueskyOAuthClient` aliases away the
five generics; `BlueskyFlow` (held in `pending_oauth`) carries the client +
session-store clone between `begin_oauth` and `complete_oauth`. The atrium
`Session { dpop_key, token_set }` is serialized + encrypted into
`StoredCredential::BlueskyOAuth { did, client_id, redirect_uri, encrypted_session }`.
Posting restores the session into a fresh client and calls
`com.atproto.repo.createRecord` (`app.bsky.feed.post`) via an `Agent`; the
rotated session is re-persisted after each post (ATProto refresh tokens are
single-use). The post `createdAt` is built from a JS-sourced epoch-millis
timestamp on `ComposeRequest` (chrono has no `clock` on wasm).

**Deployment requirement:** the auth server fetches the client metadata over real
HTTPS, so Bluesky OAuth works when the app is served at a public HTTPS origin with
a matching `public/client-metadata-bluesky.json` (the shipped file targets
`https://mux.social`; edit `client_id`/`redirect_uris`/`client_uri` to match your
origin). For local dev, serve at `http://127.0.0.1` (an http loopback) — the web
client then sends an empty `client_id`, selecting the ATProto localhost dev client
(no hosted metadata). The default `https://localhost:4000` rsbuild dev server
satisfies neither (https + self-signed, non-loopback host), so test Bluesky from a
deployed origin or a `127.0.0.1` http server.

**Also fixed for this:** the wasm `gloo` transport previously rejected request
bodies; it now sends them (Uint8Array), which the Mastodon OAuth POSTs needed too.

## Testing

`cargo test -p muxsocial-lib posting` covers: the fan-out (one result per
account, failures don't short-circuit, multiple-same-network), `secret_box`
round-trip / wrong-password / tamper, the account store persistence, credential
serde round-trips, and the nostr signer building a verifiable kind-1 note.
Hashiverse posting against a live bucket and the OAuth round-trips are verified
in-browser / in the integration harness.
