/**
 * TypeScript mirrors of the cross-post types serialized by the Rust WASM client
 * (`muxsocial-lib::posting`). Kept in sync by hand — the wasm-bindgen `.d.ts`
 * types these methods' returns only as `any`.
 *
 * @module
 */

/** Serialized form of `SourceNetwork` (serde enum, variant names verbatim). */
export type SourceNetwork = "Hashiverse" | "Nostr" | "Mastodon" | "Bluesky";

/** A secret-free account, as returned by `list_accounts` / mutations. */
export interface AccountView {
	account_id: string;
	network: SourceNetwork;
	display_label: string;
}

/** Per-account outcome of a cross-post (discriminated on `status`). */
export type PostOutcome = { status: "published"; post_url: string | null; native_post_id: string | null } | { status: "failed"; error_message: string };

/** One row of `cross_post` results: which account, and how it went. */
export interface PostResult {
	network: SourceNetwork;
	account_label: string;
	outcome: PostOutcome;
}
