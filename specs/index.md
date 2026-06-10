# mux.social Specification

mux.social is an open source social media tool that aggregates posts from the top tier open source social media networks: Hashiverse, nostr, Mastodon, and Bluesky. It is a serverless SPA — everything lives in the browser.

## How this specification is organised

This folder is a hierarchy of folders containing markdown files. Each subfolder contains an `index.md` that describes the specification covered in that subfolder and links to the other markdown files in it. If any individual markdown file gets too long, it is replaced with a subfolder of the same name, and the too-long file is splintered into an `index.md` plus sub-markdown files inside that child subfolder. This keeps each file small enough that AI agents can navigate subsections of the spec without loading the entire spec into context.

## Sections

- [architecture/](architecture/index.md) — system architecture, toolchain, and repository structure

## Planned future sections

These do not exist yet; they will be created as the corresponding functionality is specified:

- `networks/hashiverse/` — Hashiverse network integration
- `networks/nostr/` — nostr network integration
- `networks/mastodon/` — Mastodon network integration
- `networks/bluesky/` — Bluesky network integration
