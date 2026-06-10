# muxsocial

An open source social media tool that aggregates posts from the top tier open source social media networks: Hashiverse, nostr, Mastodon, and Bluesky.

Muxsocial is a serverless SPA (single page application). There is no server other than the hosting of the SPA — everything lives in the browser. The heavy lifting is done in Rust compiled to WASM; the GUI is React/Mantine.

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

## License

Dual-licensed under either of

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
