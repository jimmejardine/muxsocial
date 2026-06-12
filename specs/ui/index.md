# User interface

The mux.social GUI is a React/Mantine SPA in `muxsocial-client-web`. UI code talks to the
worker-backed `MuxsocialClientWasmProxy` and is a pull-based view over Rust-owned state (see
[../architecture/worker-rpc.md](../architecture/worker-rpc.md)).

## Sections

- [timelines.md](timelines.md) — the app shell and timeline columns: add/remove/rename
  timelines, the paste-address source bar, source chips, and confirm-before-remove.
- [posts.md](posts.md) — the post list: `usePosts` windowing/paging, `PostCard`/`PostBody`
  (sanitized HTML, permalink source bar), and live relative timestamps.
- [notifications.md](notifications.md) — success/error toasts.
- [localization.md](localization.md) — i18next setup, runtime-fetched languages, and the
  language switcher.
- [theming.md](theming.md) — the pluggable theme system: data-driven theme registry,
  forced Mantine color schemes, runtime Google Fonts loading, and per-theme backgrounds.
- [config-transfer.md](config-transfer.md) — the Config dialog: the whole configuration
  (timelines + GUI settings) as one copy/paste JSON document, with Apply/Revert.
