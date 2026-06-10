# Hashiverse

## Library

[`hashiverse-lib`](https://crates.io/crates/hashiverse-lib) (`1.0.7`) — the core
protocol crate. Hashiverse is a DHT + proof-of-work network, so a client is
assembled from injected runtime services (transport, PoW, time) plus storage and
a key locker. The ergonomic native defaults (sqlite storage, on-disk key locker,
DNSSEC bootstrap, native parallel PoW) live in
[`hashiverse-client-rust`](https://crates.io/crates/hashiverse-client-rust) via
`HashiverseBuilder`.

`muxsocial-lib` depends only on `hashiverse-lib` (not the native-only
`hashiverse-client-rust`) and takes an already-constructed `HashiverseClient` by
reference, so the heavy native construction stays out of the wasm path.

## Reading posts

`muxsocial-lib/src/sources/hashiverse.rs`:

1. Parse the user `Id` from hex (`Id::from_hex_str`).
2. `client.single_timeline_get_more(BucketType::User, &id)`.
3. Map each `EncodedPostV1` → `AggregatedPost` (`post_id` hex,
   `header.verification_key_bytes` hex, `header.time_millis`, `post` HTML body).

## Identifier

A 32-byte `Id` as 64 hex characters. A real well-known user id on the live
network is still needed as a test target (none is hard-coded in hashiverse-lib).

## Status

A first-class dependency of `muxsocial-lib` (no feature gate). It builds for both
native (via the local moka patch) and wasm32.

### The moka patch

Published `hashiverse-lib 1.0.7` calls `moka::ExternalClock`, a seam that exists
only in hashiverse's vendored moka fork, not upstream crates.io `moka`. Since
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
(no well-known id is baked into hashiverse-lib). The harness `h` command does the
same interactively.

### WASM

`cargo build -p muxsocial-lib --target wasm32-unknown-unknown` passes (with
LLVM/clang installed — see [../index.md](../index.md)). On wasm, `hashiverse-lib`
does not pull `moka` (native-only), so the patch is irrelevant there; `blake3`'s
bundled C is compiled by clang like nostr's.
