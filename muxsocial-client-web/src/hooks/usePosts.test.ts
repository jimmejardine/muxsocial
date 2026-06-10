import { describe, expect, it } from "vitest";
import type { Post } from "../tools/Post.ts";
import { EMPTY_POSTS_STATE, merge_post_delta, type PostsState, VIRTUOSO_START_INDEX } from "./usePosts.ts";

function post(source_post_id: string, created_at_millis: number): Post {
	return {
		source: "Bluesky",
		source_post_id,
		author_identifier: "author",
		author_display_name: null,
		created_at_millis,
		content_html: `<p>${source_post_id}</p>`,
		post_url: null,
	};
}

function state_with(posts: Post[], firstItemIndex = VIRTUOSO_START_INDEX): PostsState {
	return { posts, firstItemIndex, reachedOldest: false };
}

describe("merge_post_delta", () => {
	it("seeds an empty window without shifting the index", () => {
		const merged = merge_post_delta(EMPTY_POSTS_STATE, [post("b", 200), post("a", 100)]);
		expect(merged.posts.map((p) => p.source_post_id)).toEqual(["b", "a"]);
		expect(merged.firstItemIndex).toBe(VIRTUOSO_START_INDEX);
		expect(merged.reachedOldest).toBe(false);
	});

	it("appends an older batch without shifting the index", () => {
		const current = state_with([post("c", 300), post("b", 200)]);
		const merged = merge_post_delta(current, [post("a", 100)]);
		expect(merged.posts.map((p) => p.source_post_id)).toEqual(["c", "b", "a"]);
		expect(merged.firstItemIndex).toBe(VIRTUOSO_START_INDEX);
	});

	it("prepends a newer batch and decrements the index by the prepended count", () => {
		const current = state_with([post("b", 200), post("a", 100)]);
		const merged = merge_post_delta(current, [post("d", 400), post("c", 300)]);
		expect(merged.posts.map((p) => p.source_post_id)).toEqual(["d", "c", "b", "a"]);
		expect(merged.firstItemIndex).toBe(VIRTUOSO_START_INDEX - 2);
	});

	it("only the newer part of a mixed batch shifts the index", () => {
		const current = state_with([post("c", 300), post("b", 200)]);
		// "e" is newer than the head (prepend); "a" is older (merge below).
		const merged = merge_post_delta(current, [post("e", 500), post("a", 100)]);
		expect(merged.posts.map((p) => p.source_post_id)).toEqual(["e", "c", "b", "a"]);
		expect(merged.firstItemIndex).toBe(VIRTUOSO_START_INDEX - 1);
	});

	it("dedupes by source_post_id", () => {
		const current = state_with([post("b", 200), post("a", 100)]);
		const merged = merge_post_delta(current, [post("b", 200), post("a", 100)]);
		// Both are duplicates -> nothing fresh -> reached oldest, window unchanged.
		expect(merged.posts.map((p) => p.source_post_id)).toEqual(["b", "a"]);
		expect(merged.firstItemIndex).toBe(VIRTUOSO_START_INDEX);
		expect(merged.reachedOldest).toBe(true);
	});

	it("marks reachedOldest when the delta is empty", () => {
		const current = state_with([post("a", 100)]);
		const merged = merge_post_delta(current, []);
		expect(merged.reachedOldest).toBe(true);
		expect(merged.posts).toEqual(current.posts);
	});

	it("is pure: applying the same delta twice from the same state is idempotent", () => {
		const current = state_with([post("b", 200)]);
		const once = merge_post_delta(current, [post("c", 300)]);
		const twice = merge_post_delta(current, [post("c", 300)]);
		expect(once).toEqual(twice);
	});
});
