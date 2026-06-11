# Hashiverse

## Library

[`hashiverse-lib`](https://crates.io/crates/hashiverse-lib) — the core protocol
crate. Hashiverse is a DHT + proof-of-work network, so a client is assembled from
injected runtime services (transport, PoW, time) plus storage and a key locker.
The ergonomic **native** defaults (sqlite storage, on-disk key locker, DNSSEC
bootstrap, native parallel PoW) live in
[`hashiverse-client-rust`](https://crates.io/crates/hashiverse-client-rust) via
`HashiverseBuilder`. `hashiverse-lib` itself also carries **wasm** implementations
of those services (IndexedDB key locker + storage, fetch transport,
single-threaded PoW).

`muxsocial-lib` depends only on `hashiverse-lib`, never the native-only
`hashiverse-client-rust`. Native callers (the integration harness) inject an
already-constructed `HashiverseClient`; in the browser, muxsocial builds its own
read-only **guest** client from `hashiverse-lib` (see below).

## Browser guest client

`sources::hashiverse::build_guest_client` (wasm32 only) assembles a guest
`HashiverseClient` from `hashiverse-lib`'s wasm services: `WasmKeyLockerManager`
(empty keyphrase = the guest identity), `WasmClientStorage` (IndexedDB),
`WasmTransportFactory` (fetch), `RealTimeProvider`, and `SingleThreadedPowGenerator`
— reading a timeline needs no PoW workers. The timeline builder calls this lazily
the first time a timeline has a Hashiverse source (see
[`../../architecture/timelines.md`](../../architecture/timelines.md)).

## Reading posts

`muxsocial-lib/src/sources/hashiverse.rs`:

1. Parse the user `Id` from hex (`Id::from_hex_str`).
2. `client.single_timeline_get_more(BucketType::User, &id)`.
3. Map each `EncodedPostV1` → `AggregatedPost` (`post_id` hex,
   `header.verification_key_bytes` hex, `header.time_millis`, `post` HTML body).

**Permalink (`post_url`):** the Hashiverse web app's hash route,
`https://app.hashiverse.com/#/post/{post_id}/{bucket_location}`, with both segments
percent-encoded (the post's `bucket_location` is carried through the mapper).

**Media:** Hashiverse post bodies are native HTML, so any images/video stay inline
in `content_html` (the GUI's sanitizer allows `<img>`/`<video>`); the structured
`media` list is empty.

**Pagination:** `HashiversePager` forwards both `fetch_newer` and `fetch_older` to
`single_timeline_get_more`, which already returns the next latest-then-earlier
deduped batch — so each `get_more` is effectively one call to it.

## Identifier

A 32-byte `Id` as 64 hex characters. A real well-known user id on the live
network is still needed as a test target (none is hard-coded in hashiverse-lib).

## Status

A first-class dependency of `muxsocial-lib` (no feature gate). It builds for both
native (via the local moka patch) and wasm32.

### The moka patch

`hashiverse-lib` calls `moka::ExternalClock`, a seam that exists only in
hashiverse's vendored moka fork, not upstream crates.io `moka`. Since
`moka` is a native-only dependency of `hashiverse-lib`, its native build won't
link against the registry crate. The fork is vendored locally at
`muxsocial-rust/3rdparty/moka` and wired in with:

```toml
[patch.crates-io]
moka = { path = "3rdparty/moka" }
```

With that, `muxsocial-lib` links natively. (moka is also pulled by
`atrium-common`, so the patch is always active.)

### The live test

`muxsocial-integration-tests/tests/hashiverse_pull_smoke.rs` builds a client via
`HashiverseBuilder` and pulls a user's timeline. It needs a real 64-char hex user
id supplied via `MUXSOCIAL_HASHIVERSE_TEST_USER_ID`; without it the test skips
(no well-known id is baked into hashiverse-lib). The harness `th` command does the
same interactively (`tn`/`tb`/`tm`/`th` page each network, `tx` the mixed timeline).

### WASM

`cargo build -p muxsocial-lib --target wasm32-unknown-unknown` passes (with
LLVM/clang installed — see [../index.md](../index.md)). On wasm, `hashiverse-lib`
does not pull `moka` (native-only), so the patch is irrelevant there; `blake3`'s
bundled C is compiled by clang like nostr's.
