/**
 * TS mirror of the Rust `AggregatedPost` shape returned by the WASM client's
 * `get_more_posts` / `timeline_posts` (`serde_wasm_bindgen`). The Rust layer owns
 * this state; this type just describes the snapshot on the wire.
 *
 * @module
 */

import type { SourceNetwork } from "./TimelineConfig.ts";

export interface Post {
	/** The network this post came from. */
	source: SourceNetwork;
	/** The network-native, stable identifier (used as the React key and for dedupe). */
	source_post_id: string;
	/** The network-native author identifier (pubkey hex, handle, acct, DID). */
	author_identifier: string;
	/** A human-friendly author name when the network exposes one cheaply. */
	author_display_name: string | null;
	/** Post creation time as Unix epoch milliseconds (UTC). */
	created_at_millis: number;
	/** The post body as HTML (sanitized before rendering — see {@link ../components/PostBody}). */
	content_html: string;
	/** Canonical web permalink to the original post on its network, or null. */
	post_url: string | null;
}
