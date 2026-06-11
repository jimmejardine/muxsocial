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

## PostCard / PostBody

`components/PostCard.tsx` renders one post: a thin left **source bar** in the network's color
with the network name written vertically, then the author, the body, and the timestamp. When
the post has a `post_url`, the source bar is a link (`target="_blank" rel="noreferrer"`) to the
original post on its network (see the per-network permalink formats in
[`../networks/index.md`](../networks/index.md)); otherwise it's a plain bar.

`components/PostBody.tsx` renders `content_html`. Bodies are untrusted network HTML, so
`sanitize_post_html` runs them through **DOMPurify** before `dangerouslySetInnerHTML`
(exported separately so the sanitization is unit-tested).

## Relative timestamps

`tools/RelativeTimeAgo.tsx` shows a live "5 minutes ago" after the absolute time. It formats
via `Intl.RelativeTimeFormat` in the active [i18n](localization.md) language (no translation
keys) and self-updates on an adaptive interval (10s when under a minute old, then 30s / 5m /
30m as the post ages), rendered as a semantic `<time>` element. The formatting function takes
`now` as a parameter so it is pure and testable.
