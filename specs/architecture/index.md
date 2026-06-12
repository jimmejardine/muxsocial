# Architecture

## Overview

mux.social is a serverless SPA. There is no backend server — the only hosted artifact is the static SPA bundle. All aggregation, protocol handling, and storage happens in the browser.

The application is layered:

1. **muxsocial-lib** (Rust) — the bulk of the functionality: protocol clients, aggregation logic, data types. Pure Rust, unit-tested with `cargo test`.
2. **muxsocial-client-wasm** (Rust → WASM) — a thin wrapper exposing selected `muxsocial-lib` functionality to TypeScript via `wasm-bindgen`. Built with `wasm-pack build --release --target bundler` into a gitignored `pkg/` folder. Tested with `wasm-bindgen-test` in headless Chrome.
3. **MuxsocialWorker** (TypeScript, Web Worker) — the WASM module is loaded and owned by a dedicated Web Worker so that no WASM work ever runs on the main thread. See [worker-rpc.md](worker-rpc.md).
4. **muxsocial-client-web** (TypeScript/React/Mantine) — the GUI. UI code talks to a typed async proxy (`MuxsocialClientWasmProxy`) and never touches WASM or the worker directly.
5. **muxsocial-integration-tests** (Rust) — long-running tests broader than unit tests, plus a `test-harness` clap binary for interactive/scripted runs.

The build toolchain (pinned Rust workspace, the WASM bridge, the web client stack, and PWA
packaging) is specified in [toolchain.md](toolchain.md); logging (the `log` facade and the
per-surface listeners) in [logging.md](logging.md).

## Build order

```
wasm-pack build muxsocial-client-wasm --release --target bundler   # from /muxsocial-rust
npm install && npm run build                                       # from /muxsocial-client-web
```

## CI

`run_ci_checks.mjs` (repo root, plain Node, no dependencies) is the single source
of truth for the build/test/lint pipeline, run fail-fast in order: `cargo fmt
--check`, `cargo clippy -p muxsocial-lib --all-targets -- -D warnings`, `cargo test
-p muxsocial-lib`, the wasm-pack build, `npm ci`, Biome (`npm run check:ci`),
vitest, and the web build. Developers run `node run_ci_checks.mjs` locally before
committing; `.github/workflows/ci.yml` runs the same file (its other steps are
environment setup, tag→version stamping, and artifact upload), so CI and local
can't drift. A separate `deploy` job publishes `muxsocial-client-web/dist` to
Cloudflare Pages on `v*` tags or manual dispatch; a separate `check-translations`
workflow is the i18n staleness gate (see [../ui/localization.md](../ui/localization.md)).

## Files

- [toolchain.md](toolchain.md) — the pinned Rust workspace, the WASM bridge, the web client stack, and PWA/installability packaging
- [logging.md](logging.md) — the `log` facade and the per-surface listeners (native tracing-subscriber, wasm fern → console)
- [worker-rpc.md](worker-rpc.md) — the Web Worker / MessageChannel RPC design between the GUI and WASM
- [timelines.md](timelines.md) — the timeline/pagination engine (`SourcePager`, `SourceTimeline`, `MultiTimeline`, `SharedSourceClients`)
- [config-storage.md](config-storage.md) — the `ConfigStorage` key/value trait and its in-memory / IndexedDB backends
