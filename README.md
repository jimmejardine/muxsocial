<img align="right" width="150" src="muxsocial-client-web/public/img/favicon/muxsocial-icon-256.png" alt="mux.social icon">

# mux.social

An open source social media tool that aggregates posts from the top tier open source social media networks: Hashiverse, nostr, Mastodon, and Bluesky.

mux.social is a serverless SPA (single page application). There is no server other than the hosting of the SPA — everything lives in the browser. The heavy lifting is done in Rust compiled to WASM; the GUI is React/Mantine.

[![CI](https://github.com/jimmejardine/muxsocial/actions/workflows/ci.yml/badge.svg)](https://github.com/jimmejardine/muxsocial/actions/workflows/ci.yml)
[![check-translations](https://github.com/jimmejardine/muxsocial/actions/workflows/check-translations.yml/badge.svg)](https://github.com/jimmejardine/muxsocial/actions/workflows/check-translations.yml)

The `check-translations` badge goes red when a locale falls behind the English source — run `node muxsocial-client-web/translations/check-translations.mjs` and feed its JSON output into a Claude Code session to update the flagged strings.

## Repository layout

```
/specs                                        # The hierarchical specs folder
/muxsocial-client-web                         # The Typescript/React/Mantine GUI consuming muxsocial-client-wasm
/muxsocial-rust                               # Rust workspace
/muxsocial-rust/muxsocial-lib                 # Rust library containing the bulk of functionality
/muxsocial-rust/muxsocial-client-wasm         # Rust wrapper exposing some of the muxsocial-lib to WASM/Typescript
/muxsocial-rust/muxsocial-integration-tests   # Long-running tests that are broader than unit tests
```

## Development

Prerequisites: rustup (the pinned nightly in `muxsocial-rust/rust-toolchain.toml` plus the `wasm32-unknown-unknown` target), [wasm-pack](https://rustwasm.github.io/wasm-pack/), and Node.js.

The WASM build also needs **LLVM/clang on `PATH`**: it cross-compiles nostr's `secp256k1-sys` C code to wasm, and cc-rs uses clang for the `wasm32-unknown-unknown` target. Linux/macOS usually have clang already; on Windows install [LLVM](https://releases.llvm.org/) and ensure `C:\Program Files\LLVM\bin` is on `PATH`. (Native `cargo` builds use the platform's default C toolchain and don't need this. Unlike the hashiverse toolchain, which is C-free on wasm, muxsocial requires clang because nostr's secp256k1 has no pure-Rust substitute.)

The web client imports the WASM package by relative path from `muxsocial-rust/muxsocial-client-wasm/pkg`, which is gitignored — so the WASM build must run before anything in the web client will type-check or build:

```
cd muxsocial-rust
wasm-pack build muxsocial-client-wasm --release --target bundler
cd ../muxsocial-client-web
npm install
npm run dev
```

### Tests

```
cd muxsocial-rust
cargo test                                                  # muxsocial-lib unit tests
cargo test -p muxsocial-integration-tests                   # integration smoke tests
cargo run -p muxsocial-integration-tests --bin test-harness # long-running test harness
wasm-pack test --chrome --headless muxsocial-client-wasm    # WASM tests in headless Chrome
```

### Running CI locally

Before committing, run the full CI pipeline (rust fmt/clippy/tests, wasm build, web lint/tests/build)
from the repo root:

```
node run_ci_checks.mjs
```

GitHub Actions runs this same file, so a green local run mirrors CI. It needs the same toolchain as a
manual build (rust, the `wasm32-unknown-unknown` target, clang/LLVM, wasm-pack, and Node).

## Deployment

The site is hosted on **Cloudflare Pages**. CI (`.github/workflows/ci.yml`) builds the web
client and the `deploy` job direct-uploads `muxsocial-client-web/dist` to the Pages project
**`muxsocial`** (production branch `main`) via `wrangler pages deploy … --branch=main`.

One-time setup:

1. Create the Pages project `muxsocial` with production branch `main` (Cloudflare dashboard →
   Workers & Pages → create a Pages project via direct upload, or
   `wrangler pages project create muxsocial --production-branch=main`).
2. Create a Cloudflare API token with the **Cloudflare Pages: Edit** permission, and note your
   **Account ID**.
3. Add them as GitHub repository secrets: `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`.

Deploys to production then run on a **`v*` tag push** (`git tag v1.0.9 && git push origin v1.0.9`)
or **manually** via Actions → CI → "Run workflow". (A manual run from a non-tag ref leaves the
baked-in version at `0.0.0`; dispatch from a tag to stamp it.)

## Install as a desktop app

mux.social is a PWA: in Chrome/Edge (and on the deployed HTTPS site) an install icon appears in the
address bar — "Install mux.social" adds it to the desktop and launches it in its own standalone
window. This is driven by `public/manifest.webmanifest` and the service worker `public/sw.js`
(registered in `src/index.tsx`), which also provides a basic offline fallback.

## License

Dual-licensed under either of

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
