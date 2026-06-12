# Timelines (UI)

The main view is a row of independent **timeline columns**. Each column is one Rust
`MultiTimeline` (see [`../architecture/timelines.md`](../architecture/timelines.md)); the GUI is
a pull-based view over the Rust-owned state (see
[`../architecture/worker-rpc.md`](../architecture/worker-rpc.md)).

## Shell

`App.tsx` is the Mantine `AppShell`: a header (`Toolbar`) and the timeline area
(`TimelineArea`) — there is no footer. It creates the worker-backed client once, seeds
`list_timelines()`, holds the `TimelineConfig[]` snapshot, and provides the client to
descendants via `MuxsocialContext` (`tools/MuxsocialContext.tsx`, `useMuxsocial()`). Each
mutation callback (`add_timeline` / `remove_timeline` / `add_source` / `remove_source` /
`set_name` / `set_autopoll`) calls the WASM command and replaces state with the returned
snapshot, firing a [toast](notifications.md) on success/failure. It also owns the
[help wizard](help-wizard.md)'s open state.

- `Toolbar` (`components/Toolbar.tsx`) — the app logo + "mux.social" title, then a smaller
  dimmed **tagline** ("your singular dashboard for Hashiverse, Nostr, Mastodon & Bluesky",
  each network an `<a target="_blank">` to its homepage via `tools/networks.ts`). The tagline
  is the only flexible element: it truncates with `…` when the header is narrow while the
  title and the right-side controls stay fixed. On the right: an **Add timeline** button and a
  **hamburger** (`HeaderMenu.tsx`, a Burger-triggered popover) holding the
  [Getting started wizard](help-wizard.md) launcher, the [language](localization.md) and
  [theme](theming.md) switchers, a Networks menu (homepage links), the
  [Config dialog](config-transfer.md) launcher, a GitHub link, and the app version (from the
  WASM `version()`) linking to the releases page. As an onboarding cue, the Add-timeline
  button pulses (the global `.mux-throb` glow, reduced-motion aware) while there are no
  timelines; the lone empty timeline's "+" add-source button pulses the same way.
- `TimelineArea` (`components/TimelineArea.tsx`) — lays the columns side by side (each at least
  500px, horizontally scrolling when they overflow; `scroll-snap-type: x proximity` with each
  column a `scroll-snap-align: start` point, so a sideways scroll that settles near a column
  boundary snaps its left edge flush). With no timelines it shows an empty state: the
  `muxsocial.jpg` hero above a **Getting started** button that opens the
  [help wizard](help-wizard.md). Columns are keyed by timeline id so per-column state survives
  re-renders.

## A timeline column

`Timeline.tsx` renders one column:

- An editable **name** input (commits on blur/Enter; with no custom name it shows a
  source-derived default — the source ids joined with the same `truncate_source_id` shortener
  as the chips — falling back to "Timeline N").
- A **Get more posts** button → pages this timeline (see [posts.md](posts.md)), with an
  **autopoll** toggle next to it (a depressable reload-icon `ActionIcon`, default on/`filled`):
  while on, the timeline auto-refreshes on the recurring 5-minute tick; while off it no-ops.
  The initial pull (and the pull when a source changes) is automatic regardless. The flag is
  persisted per timeline (`TimelineConfig.autopoll`); toggling it toasts
  "Autorefresh ON"/"Autorefresh OFF".
- A **remove** (✕) button that opens a [`ConfirmModal`](#confirm-before-remove) before
  deleting — unless the timeline has no sources, in which case there is nothing to lose and it
  is removed immediately.
- A compact, single-line **source row** (scrolls horizontally when it overflows) that holds a
  **paste button** + a **"+" add button** as its first items, followed by one **source chip**
  per source. The paste button reads the clipboard and adds it as a source via the parse-and-add
  path — a valid id is added (success toast), an invalid one or a clipboard-read failure raises an
  error toast. The "+" button opens an [`AddSourceModal`](#add-source-dialog) (a small dialog with
  the "Paste an address" input and OK/Cancel) that adds the source via the same path; it pulses
  (`.mux-throb`) while the lone timeline has no sources. The Rust
  `parse_source_address` (`timeline_registry.rs`) detects the network through four fall-through
  layers, each reusing the previous one to classify: **(1)** an explicit `network:identifier`
  prefix (`nostr:` / `bluesky:`|`bsky:` / `mastodon:`|`masto:` / `hashiverse:`|`hash:`); **(2)** a
  bare id — `@user@host` → Mastodon, `npub1…`/`nprofile1…` → nostr, `did:plc:…` or a dotted handle
  → Bluesky, a 64-hex string → Hashiverse; **(3)** a **profile URL** (scheme optional, any host —
  networks self-host on any domain — matched by *structure*: an `npub`/`nprofile` token → nostr, a
  64-hex path segment → Hashiverse, a `/profile/<handle|did>` path → Bluesky, a `/@user[@host]` or
  `/users/<user>` path → Mastodon, with the URL host filling in a Mastodon home instance when the
  path omits it); and **(4)** a URL (or bare `npub`) **embedded in surrounding text**. Layer 2
  ignores anything containing whitespace or `/`, so a schemeless URL falls through to layer 3. An
  `nprofile` is decoded (via nostr-sdk NIP-19) to its `npub`.
  Each source chip is colored by network ([`theme/networkColors.ts`](theming.md)) and shows
  `network: id`. Long opaque ids (Nostr `npub…`, Hashiverse hex) are shortened to `first8…last3`
  via `truncate_source_id` (full id in the tooltip); Mastodon/Bluesky handles show in full.
  Each chip has a `×` that removes the source (via `remove_source_from_timeline`) after the
  same `ConfirmModal`; clicking the chip label copies the full source address to the clipboard
  (with a toast).
- The post list itself (see [posts.md](posts.md)).

## Add-source dialog

The "+" button opens `AddSourceModal` (`components/AddSourceModal.tsx`), a small Mantine `Modal`
with a single auto-focused "Paste an address" input and an OK / Cancel button pair (focus trap +
Escape/backdrop close). OK (or Enter) submits the trimmed address to the same parse-and-add path
as the paste button; the modal owns the draft value and clears it on close so each open starts
empty.

## Confirm before remove

Destructive actions go through `ConfirmModal` (`components/ConfirmModal.tsx`), a small wrapper
over Mantine `Modal` with a message and a Cancel / red-confirm button pair (focus trap +
Escape/backdrop close). Removing a timeline that has sources opens it (an empty timeline skips
straight to removal); confirming runs the existing `remove_timeline` path.
