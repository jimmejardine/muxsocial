# mux.social Specification

mux.social is an open source social media tool that aggregates posts from the top tier open source social media networks: Hashiverse, nostr, Mastodon, and Bluesky. It is a serverless SPA — everything lives in the browser.

## How this specification is organised

This folder is a hierarchy of folders containing markdown files. Each subfolder contains an `index.md` that describes the specification covered in that subfolder and links to the other markdown files in it. If any individual markdown file gets too long, it is replaced with a subfolder of the same name, and the too-long file is splintered into an `index.md` plus sub-markdown files inside that child subfolder. This keeps each file small enough that AI agents can navigate subsections of the spec without loading the entire spec into context.

## Sections

- [architecture/](architecture/index.md) — system architecture, toolchain, the worker RPC, the timeline/pagination engine, and config storage
- [networks/](networks/index.md) — source-network integration (Hashiverse, nostr, Mastodon, Bluesky), per-network pagination and permalinks, the shared HTTP transport, and the normalized post model
- [ui/](ui/index.md) — the GUI: timeline columns, the post list, notifications, localization, and the pluggable theme system
