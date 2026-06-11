# Timelines (UI)

The main view is a row of independent **timeline columns**. Each column is one Rust
`MultiTimeline` (see [`../architecture/timelines.md`](../architecture/timelines.md)); the GUI is
a pull-based view over the Rust-owned state (see
[`../architecture/worker-rpc.md`](../architecture/worker-rpc.md)).

## Shell

`App.tsx` is the Mantine `AppShell`: a header (`Toolbar`), the timeline area
(`TimelineArea`), and a footer (`StatusBar`). It creates the worker-backed client once, seeds
`list_timelines()`, holds the `TimelineConfig[]` snapshot, and provides the client to
descendants via `MuxsocialContext` (`tools/MuxsocialContext.tsx`, `useMuxsocial()`). Each
mutation callback (`add_timeline` / `remove_timeline` / `add_source` / `set_name`) calls the
WASM command and replaces state with the returned snapshot, firing a [toast](notifications.md)
on success/failure.

- `Toolbar` (`components/Toolbar.tsx`) — the app logo + "mux.social" title (left) plus the
  [language](localization.md) and [theme](theming.md) switchers and an **Add timeline** button.
  As an onboarding cue, the Add-timeline button pulses (the global `.mux-throb` glow,
  reduced-motion aware) while there are no timelines; the address box pulses the same way when
  the lone timeline still has no sources.
- There is no footer/status bar. The app version (from the WASM `version()`) is the first item
  in the hamburger menu — a button linking to the GitHub releases page.
- `TimelineArea` (`components/TimelineArea.tsx`) — lays the columns side by side (each at least
  500px, horizontally scrolling when they overflow), with an empty state (the `muxsocial.jpg`
  hero above the "add a timeline" hint) when there are
  no timelines. Columns are keyed by timeline id so per-column state survives re-renders.

## A timeline column

`Timeline.tsx` renders one column:

- An editable **name** input (commits on blur/Enter; with no custom name it shows a
  source-derived default — the source ids joined with the same `truncate_source_id` shortener
  as the chips — falling back to "Timeline N").
- A **Get more posts** button → pages this timeline (see [posts.md](posts.md)).
- A **remove** (✕) button that opens a [`ConfirmModal`](#confirm-before-remove) before deleting.
- An **address bar**: paste an identifier and press Enter to add a source. The Rust
  `parse_source_address` (`timeline_registry.rs`) detects the network — an explicit
  `network:identifier` prefix (`nostr:` / `bluesky:`|`bsky:` / `mastodon:`|`masto:` /
  `hashiverse:`|`hash:`) always wins, otherwise `@user@host` → Mastodon, `npub1…` → nostr,
  `did:plc:…` or a bare dotted handle → Bluesky, and a bare 64-hex string → Hashiverse.
- A compact row of **source chips** — one per source, colored by network
  ([`theme/networkColors.ts`](theming.md)) and showing `network: id`. Long opaque ids
  (Nostr `npub…`, Hashiverse hex) are shortened to `first8…last3` via
  `truncate_source_id` (full id in the tooltip); Mastodon/Bluesky handles show in full.
  Each chip has a `×` that removes the source (via `remove_source_from_timeline`) after the
  same `ConfirmModal`; clicking the chip label copies the full source address to the clipboard
  (with a toast).
- The post list itself (see [posts.md](posts.md)).

## Confirm before remove

Destructive actions go through `ConfirmModal` (`components/ConfirmModal.tsx`), a small wrapper
over Mantine `Modal` with a message and a Cancel / red-confirm button pair (focus trap +
Escape/backdrop close). Removing a timeline opens it; confirming runs the existing
`remove_timeline` path.
