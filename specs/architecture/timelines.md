# Timelines and pagination

The per-network clients in [`networks/`](../networks/index.md) can pull a one-shot
`fetch_recent_posts`, but the app pages timelines through a small stateful engine in
`muxsocial-lib/src/timeline/`. It mirrors hashiverse's `SingleTimeline` / `MultipleTimeline`:
each timeline accumulates a deduped, newest-first post list and advances its position on each
`get_more`, hiding the per-network pagination differences behind one seam.

## The pager seam

`SourcePager` (`timeline/mod.rs`) is the per-network pagination trait — the analogue of an
injected I/O backend:

- `fetch_newer(newest_known, limit)` — posts newer than what we hold (`None` = initial latest).
- `fetch_older(oldest_known, limit)` — posts older than what we hold.
- `reset()` — restart paging at "now".

Each network paginates differently, so each provides its own pager (see the per-network specs):
nostr by `Filter` `.since`/`.until` timestamps, Bluesky by opaque cursor, Mastodon by status
`min_id`/`max_id`, Hashiverse by forwarding both directions to `single_timeline_get_more`.
`NetworkPager` (`timeline/network_pager.rs`) is an enum over the four so heterogeneous sources
share one `SourceTimeline<NetworkPager>` / `MultiTimeline<NetworkPager>` type; unit tests use a
network-free `StubPager` instead (`timeline::test_support`).

## SourceTimeline — one source

`SourceTimeline<P>` (`timeline/source_timeline.rs`) owns one source's accumulated posts
(newest-first), a `seen_post_ids` set for dedupe, and a `reached_oldest` flag. `get_more(limit)`
tries `fetch_newer` first and, only if nothing new arrives, `fetch_older`; it returns the batch
**added** this call (the delta) and sets `reached_oldest` once a backward fetch yields nothing.

## MultiTimeline — many sources merged

`MultiTimeline<P>` (`timeline/multi_timeline.rs`) holds several `SourceTimeline`s and a merged
newest-first list. `get_more(per_source_limit)` pulls from every child, merges the new posts
into the canonical list, and returns the merged delta. Cross-source duplicates can't occur
(distinct networks), so per-child dedupe suffices. `posts()` exposes the full ordered list;
`reached_oldest()` is true once every child is exhausted.

This is the delta model the GUI relies on: Rust keeps the canonical accumulated list, and each
`get_more` returns only the newly-added batch (see [`ui/posts.md`](../ui/posts.md) and
[`worker-rpc.md`](worker-rpc.md)).

## Building timelines — SharedSourceClients

`SharedSourceClients` (`timeline/builder.rs`) constructs `SourceTimeline`/`MultiTimeline`s from a
list of [`Source`](../networks/index.md)s while sharing one set of singleton clients: one
`HttpTransport` (Bluesky + Mastodon), one lazily-connected nostr `Client` (shared relay pool,
default relays `wss://relay.damus.io` / `wss://nos.lol`, 20s timeout), and one Hashiverse client.

- `build_multi_timeline(sources)` / `build_source_timeline(source)` switch on `source.network`
  to construct the right `NetworkPager`.
- Hashiverse needs a `HashiverseClient`: native callers inject one
  (`with_hashiverse_client`, from `hashiverse-client-rust`); in the browser the builder lazily
  builds a read-only **guest** client from `hashiverse-lib` (`ensure_hashiverse_client` →
  `sources::hashiverse::build_guest_client`, wasm32 only — see
  [`networks/hashiverse/index.md`](../networks/hashiverse/index.md)). With neither, a Hashiverse
  source errors clearly rather than failing the whole timeline.

The WASM client owns one `SharedSourceClients` and a `MultiTimeline` per timeline id; see
[`worker-rpc.md`](worker-rpc.md).
