/**
 * Per-timeline post state for the GUI's pull-based view.
 *
 * The Rust layer is the source of truth: each `get_more_posts` returns only the
 * newly-added batch (the delta), and keeps the full accumulated list internally.
 * This hook holds the rendered window in React and applies each delta, which is
 * what `react-virtuoso`'s scroll-stable prepend (`firstItemIndex`) needs and what
 * keeps each call from re-serializing an ever-growing list across the worker.
 *
 * `get_more_posts` is non-directional (Rust pulls newer-first, else older per
 * source), so the returned batch can contain newer and/or older posts. We insert
 * by timestamp: posts newer than the current head prepend (and shift
 * `firstItemIndex` to hold scroll position), the rest merge into the list.
 *
 * @module
 */

import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMuxsocial } from "../tools/MuxsocialContext.tsx";
import type { Post } from "../tools/Post.ts";
import { Toast } from "../tools/Toast.ts";

/** Posts fetched per source on each "get more" call. */
const PER_SOURCE_LIMIT = 20;

/**
 * `react-virtuoso`'s anchor: the virtual index of `posts[0]`. Starts high and is
 * decremented by the number of items prepended, so already-visible items keep
 * their virtual index (and the scroll position) across a prepend.
 */
export const VIRTUOSO_START_INDEX = 1_000_000;

function error_message(err: unknown): string {
	return err instanceof Error ? err.message : String(err);
}

/** Sort newest-first, tie-breaking on id — mirrors the Rust ordering. */
export function sort_posts_newest_first(posts: Post[]): Post[] {
	return [...posts].sort((a, b) => b.created_at_millis - a.created_at_millis || a.source_post_id.localeCompare(b.source_post_id));
}

/** The window of posts shown for one timeline, plus its virtuoso anchor. */
export interface PostsState {
	posts: Post[];
	firstItemIndex: number;
	/** True once a `get_more` returned nothing new (every source exhausted). */
	reachedOldest: boolean;
}

export const EMPTY_POSTS_STATE: PostsState = { posts: [], firstItemIndex: VIRTUOSO_START_INDEX, reachedOldest: false };

/**
 * Pure reducer: fold a delta batch into the current window. Dedupes by
 * `source_post_id`, prepends posts newer than the current head (shifting
 * `firstItemIndex` to hold scroll), and merges the rest newest-first. An empty
 * fresh set means every source is exhausted. Pure (no side effects), so it is
 * safe inside a React state updater and trivially unit-testable.
 */
export function merge_post_delta(current: PostsState, delta: Post[]): PostsState {
	const seen_ids = new Set(current.posts.map((post) => post.source_post_id));
	const fresh = delta.filter((post) => !seen_ids.has(post.source_post_id));
	if (fresh.length === 0) {
		return { ...current, reachedOldest: true };
	}
	const old_head = current.posts[0];
	const prepended_count = old_head ? fresh.filter((post) => post.created_at_millis > old_head.created_at_millis).length : 0;
	return {
		posts: sort_posts_newest_first([...fresh, ...current.posts]),
		firstItemIndex: current.firstItemIndex - prepended_count,
		reachedOldest: false,
	};
}

export interface UsePostsResult {
	posts: Post[];
	firstItemIndex: number;
	loading: boolean;
	reachedOldest: boolean;
	getMore: () => Promise<void>;
}

/**
 * Holds and pages the posts of the timeline `timeline_id`. On (re)mount it
 * reseeds from the live tracker's accumulated posts (`timeline_posts`) without
 * re-fetching from the networks; `getMore` pulls and folds in the next delta.
 */
export function usePosts(timeline_id: string): UsePostsResult {
	const { t } = useTranslation();
	const muxsocial = useMuxsocial();
	const [state, set_state] = useState<PostsState>(EMPTY_POSTS_STATE);
	const [loading, set_loading] = useState(false);

	useEffect(() => {
		if (!muxsocial) return;
		let cancelled = false;
		(async () => {
			try {
				const seeded = (await muxsocial.timeline_posts(timeline_id)) as Post[];
				if (!cancelled && seeded.length > 0) {
					set_state({ posts: sort_posts_newest_first(seeded), firstItemIndex: VIRTUOSO_START_INDEX, reachedOldest: false });
				}
			} catch (err) {
				if (!cancelled) {
					Toast.error(t("toast.error_get_more", { message: error_message(err) }));
				}
			}
		})();
		return () => {
			cancelled = true;
		};
	}, [muxsocial, timeline_id, t]);

	const getMore = useCallback(async () => {
		if (!muxsocial) return;
		set_loading(true);
		try {
			const delta = (await muxsocial.get_more_posts(timeline_id, PER_SOURCE_LIMIT)) as Post[];
			set_state((current) => merge_post_delta(current, delta));
		} catch (err) {
			Toast.error(t("toast.error_get_more", { message: error_message(err) }));
		} finally {
			set_loading(false);
		}
	}, [muxsocial, timeline_id, t]);

	return { posts: state.posts, firstItemIndex: state.firstItemIndex, loading, reachedOldest: state.reachedOldest, getMore };
}
