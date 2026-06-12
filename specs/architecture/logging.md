# Logging

The `log` crate is the single facade used throughout `muxsocial-lib` (source clients and the HTTP transports emit `log::` records). Each surface installs its own listener for those records:

- **Native** — the `test-harness` binary owns the listener: `configure_logging_listener(level)` in `muxsocial-integration-tests` sets up `tracing-subscriber` (`fmt` layer + `EnvFilter`). tracing-subscriber's default `tracing-log` bridge captures the `log` facade, so muxsocial's and the SDKs' records surface. `--log-level` sets the base level (default `trace`); `RUST_LOG` overrides per-module; noisy infra crates (hyper, reqwest, rustls, h2, hickory, tungstenite, …) are silenced by default. The listener lives in the binary, not the lib — `muxsocial-lib` only emits.
- **GUI / wasm** — `muxsocial-client-wasm::wasm_init` wires `fern` → `console_log`, so the same `muxsocial-lib` `log::` records appear in the browser console. The `MuxsocialWorker` calls `wasm_init(true)` on startup.
