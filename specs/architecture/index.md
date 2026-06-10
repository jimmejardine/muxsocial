# Architecture

## Overview

mux.social is a serverless SPA. There is no backend server — the only hosted artifact is the static SPA bundle. All aggregation, protocol handling, and storage happens in the browser.

The application is layered:

1. **muxsocial-lib** (Rust) — the bulk of the functionality: protocol clients, aggregation logic, data types. Pure Rust, unit-tested with `cargo test`.
2. **muxsocial-client-wasm** (Rust → WASM) — a thin wrapper exposing selected `muxsocial-lib` functionality to TypeScript via `wasm-bindgen`. Built with `wasm-pack build --release --target bundler` into a gitignored `pkg/` folder. Tested with `wasm-bindgen-test` in headless Chrome.
3. **MuxsocialWorker** (TypeScript, Web Worker) — the WASM module is loaded and owned by a dedicated Web Worker so that no WASM work ever runs on the main thread. See [worker-rpc.md](worker-rpc.md).
4. **muxsocial-client-web** (TypeScript/React/Mantine) — the GUI. UI code talks to a typed async proxy (`MuxsocialClientWasmProxy`) and never touches WASM or the worker directly.
5. **muxsocial-integration-tests** (Rust) — long-running tests broader than unit tests, plus a `test-harness` clap binary for interactive/scripted runs.

## Toolchain

The build toolchain mirrors the one proven in the Hashiverse project.

### Rust workspace (`/muxsocial-rust`)

- Pinned toolchain: `nightly-2026-05-19` (see `rust-toolchain.toml`), edition 2024, resolver 2.
- `default-members = ["muxsocial-lib"]` so bare `cargo test`/`cargo check` stays host-only and fast; the WASM crate and integration tests are reached with `-p`.
- Shared versions live in `[workspace.dependencies]`; lints in `[workspace.lints]`.
- `wasm32-unknown-unknown` builds enable SIMD (`-C target-feature=+simd128`, see `.cargo/config.toml`).
- Formatting via `rustfmt.toml` (max_width 250, 4 spaces, Unix newlines).
- `Cargo.lock` is committed.

### WASM bridge

- `wasm-bindgen` 0.2.108, crate-type `["cdylib", "rlib"]`.
- `wasm_init(verbose)` must be called once per worker before use: it wires `fern` → `console_log` logging and sets `console_error_panic_hook`.
- Deferred dependencies (not yet needed by the skeleton; versions recorded here to stay in lockstep with Hashiverse when added): serde 1.0.228, serde-wasm-bindgen 0.6.5, tsify 0.5.6 (js feature), web-sys 0.3, tokio 1.45, gloo-net 0.6.0, indexed_db_futures 0.6.4.
- The wasm build needs a **clang toolchain on `PATH`**: nostr's `secp256k1-sys` is C and is cross-compiled to wasm by cc-rs/clang. This is a muxsocial-specific requirement — hashiverse is C-free on wasm (pure-Rust crypto, with `ring` gated off the wasm target), but nostr's secp256k1 has no pure-Rust substitute. See the README for the Windows LLVM setup.

### Web client (`/muxsocial-client-web`)

- React 19, Mantine 8 (core/hooks/notifications), react-router 7 (HashRouter).
- Bundler: RSBuild (`@rsbuild/core` + `plugin-react` + `plugin-basic-ssl`); dev server runs on https with a self-signed certificate.
- Lint/format: Biome (tabs, lineWidth 192, double-quoted strings, `noExplicitAny` error).
- TypeScript 5.9 strict, `moduleResolution: "bundler"`; `npm run build` runs `tsc --noEmit` before bundling.
- Docs: TypeDoc.
- The WASM package is imported by **relative path** (`../../muxsocial-rust/muxsocial-client-wasm/pkg`), not as an npm dependency, so the WASM build must run before the web client builds. `pkg/` is gitignored; `package-lock.json` is committed.

## Logging

The `log` crate is the single facade used throughout `muxsocial-lib` (source clients and the HTTP transports emit `log::` records). Each surface installs its own listener for those records:

- **Native** — the `test-harness` binary owns the listener: `configure_logging_listener(level)` in `muxsocial-integration-tests` sets up `tracing-subscriber` (`fmt` layer + `EnvFilter`). tracing-subscriber's default `tracing-log` bridge captures the `log` facade, so muxsocial's and the SDKs' records surface. `--log-level` sets the base level (default `trace`); `RUST_LOG` overrides per-module; noisy infra crates (hyper, reqwest, rustls, h2, hickory, tungstenite, …) are silenced by default. The listener lives in the binary, not the lib — `muxsocial-lib` only emits.
- **GUI / wasm** — `muxsocial-client-wasm::wasm_init` wires `fern` → `console_log`, so the same `muxsocial-lib` `log::` records appear in the browser console. The `MuxsocialWorker` calls `wasm_init(true)` on startup.

## Build order

```
wasm-pack build muxsocial-client-wasm --release --target bundler   # from /muxsocial-rust
npm install && npm run build                                       # from /muxsocial-client-web
```

## Files

- [worker-rpc.md](worker-rpc.md) — the Web Worker / MessageChannel RPC design between the GUI and WASM
