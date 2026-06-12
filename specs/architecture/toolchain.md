# Toolchain

The build toolchain mirrors the one proven in the Hashiverse project.

## Rust workspace (`/muxsocial-rust`)

- Pinned toolchain: `nightly-2026-05-19` (see `rust-toolchain.toml`), edition 2024, resolver 2.
- `default-members = ["muxsocial-lib"]` so bare `cargo test`/`cargo check` stays host-only and fast; the WASM crate and integration tests are reached with `-p`.
- Shared versions live in `[workspace.dependencies]`; lints in `[workspace.lints]`.
- `wasm32-unknown-unknown` builds enable SIMD (`-C target-feature=+simd128`, see `.cargo/config.toml`).
- Formatting via `rustfmt.toml` (max_width 250, 4 spaces, Unix newlines).
- `Cargo.lock` is committed.

## WASM bridge

- `wasm-bindgen` 0.2.108, crate-type `["cdylib", "rlib"]`.
- `wasm_init(verbose)` must be called once per worker before use: it wires `fern` → `console_log` logging and sets `console_error_panic_hook`.
- Deferred dependencies (not yet needed by the skeleton; versions recorded here to stay in lockstep with Hashiverse when added): serde 1.0.228, serde-wasm-bindgen 0.6.5, tsify 0.5.6 (js feature), web-sys 0.3, tokio 1.45, gloo-net 0.6.0, indexed_db_futures 0.6.4.
- The wasm build needs a **clang toolchain on `PATH`**: nostr's `secp256k1-sys` is C and is cross-compiled to wasm by cc-rs/clang. This is a muxsocial-specific requirement — hashiverse is C-free on wasm (pure-Rust crypto, with `ring` gated off the wasm target), but nostr's secp256k1 has no pure-Rust substitute. See the README for the Windows LLVM setup.

## Web client (`/muxsocial-client-web`)

- React 19, Mantine 8 (core/hooks/notifications), react-router 7 (HashRouter).
- Bundler: RSBuild (`@rsbuild/core` + `plugin-react` + `plugin-basic-ssl`); dev server runs on https with a self-signed certificate.
- Lint/format: Biome (tabs, lineWidth 192, double-quoted strings, `noExplicitAny` error).
- TypeScript 5.9 strict, `moduleResolution: "bundler"`; `npm run build` runs `tsc --noEmit` before bundling.
- Docs: TypeDoc.
- The WASM package is imported by **relative path** (`../../muxsocial-rust/muxsocial-client-wasm/pkg`), not as an npm dependency, so the WASM build must run before the web client builds. `pkg/` is gitignored; `package-lock.json` is committed.

## PWA / installability

The web client is an installable PWA (Chrome/Edge show an install icon; "Install
mux.social" opens a standalone desktop window):

- `public/manifest.webmanifest` — `id`/`start_url`/`scope` `/`, `display:
  standalone`, electric-dark theme/background colors, the 256/512 favicon icons,
  and `screenshots` (one `form_factor: wide` desktop shot + one `narrow` mobile
  shot under `public/img/screenshots/`) for the richer install dialog. Linked from
  `public/index.html`.
- `public/sw.js` — a minimal service worker (Chromium requires a fetch handler for
  installability), registered on load in `src/index.tsx`. Same-origin GETs are
  **network-first** (assets are content-hashed) with the response cached, falling
  back to the cache — then `/` — when offline, giving an app-shell offline
  fallback. Cross-origin requests (network APIs, relays) and non-GETs are left to
  the browser. Excluded from Biome (service-worker globals) via `biome.json`.
