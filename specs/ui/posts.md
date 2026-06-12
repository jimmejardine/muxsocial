# Posts (UI)

A timeline column's body is a virtualized post list. Rust owns the canonical, accumulated
post list per timeline and each page returns only the newly-added batch (the delta — see
[`../architecture/timelines.md`](../architecture/timelines.md)); React holds the rendered
window and folds deltas in.

## usePosts — the rendered window

`hooks/usePosts.ts` owns one timeline's window: `{ posts, firstItemIndex, reachedOldest }` plus
a `loading` flag and a `getMore()` action.

- On mount it **reseeds** from `timeline_posts(id)` (the tracker's accumulated list) so a
  remount restores what was already fetched without re-hitting the networks.
- `getMore()` calls `get_more_posts(id, PER_SOURCE_LIMIT)` (20/source) and folds the delta in
  with `merge_post_delta`, a **pure reducer**: dedupe by `source_post_id`, prepend posts newer
  than the current head, merge the rest newest-first.
- Scroll stability uses `react-virtuoso`'s `firstItemIndex` pattern: a high base index
  (`VIRTUOSO_START_INDEX`) decremented by the prepended count, so already-visible posts keep
  their position when newer posts arrive. The reducer's purity makes it React-StrictMode-safe
  and unit-testable.

The "Get more posts" button is non-directional — Rust decides newer-vs-older per source — so a
single page can both prepend and append. When a page returns nothing new, `reachedOldest`
flips and the list shows an end marker.

`getMore` also fires automatically: once whenever the timeline's source set changes (a
`sources_signature` string computed in `Timeline.tsx` — so a freshly-added address loads
without a click, and a populated timeline loads on mount), and on a 5-minute interval gated by
the timeline's [autopoll flag](timelines.md) (live refs keep the interval subscription stable
across re-renders). Auto-fetches drive the same `loading` flag, so the "Get more posts" button
shows its spinner as if pressed.

## PostCard / PostBody

`components/PostCard.tsx` renders one post: a **title bar** across the top in the network's
color with the poster on the left (display name, else the identifier shortened by the same
`truncate_source_id` shrinker as the source chips) and the [timestamp](#timestamps) on the
right, then the body and media. When the post has a `post_url`, the title bar is a link
(`target="_blank" rel="noreferrer"`) to the original post on its network (see the per-network
permalink formats in [`../networks/index.md`](../networks/index.md)); otherwise it's a plain
bar. The title bar is
`position: sticky` (top), so while scrolling it pins to the top of the column until its post
scrolls past and the next post's bar takes over — a cascading sticky header within the virtuoso
list (which renders items in normal flow, so plain CSS sticky works without touching the
`firstItemIndex` prepend).

`components/PostBody.tsx` renders `content_html`. Bodies are untrusted network HTML, so
`sanitize_post_html` runs them through **DOMPurify** before `dangerouslySetInnerHTML`
(exported separately so the sanitization is unit-tested). It keeps DOMPurify's broad default
allowlist (Hashiverse bodies are rich HTML), additionally permits inline `<video>`/`<source>`,
and registers an `afterSanitizeAttributes` hook that forces `target="_blank"
rel="noopener noreferrer"` on links (nostr linkifies URLs) and `loading="lazy"` on images.

## Media

`components/PostMedia.tsx` renders the post's structured `media` (see
[`../networks/index.md`](../networks/index.md)) below the body: `image` → a lazy `<img>`
linking to the full image; `video` → a `<video controls poster>` (Bluesky video is HLS, which
plays only in Safari without hls.js — a deferred follow-up; the poster still shows); `link_card`
→ an external-link card with thumbnail, title, description, and host. Media that the source
keeps inline (Hashiverse) renders through `PostBody` instead.

## Timestamps

`tools/RelativeTimeAgo.tsx` renders the single timestamp in the title bar:
`format_timestamp` shows the **relative** form ("5 minutes ago") while the post is under 24h
old, and the **absolute** local date-time (`toLocaleString`) beyond that. The relative form
uses `Intl.RelativeTimeFormat` in the active [i18n](localization.md) language (no translation
keys) and self-updates on an adaptive interval (10s when under a minute old, then 30s / 5m /
30m as the post ages); the exact local date-time is always available as the `<time>` element's
hover tooltip. The formatting functions take `now` as a parameter so they are pure and
testable.
