/**
 * TS mirrors of the Rust `TimelineConfig` / `Source` snapshot shapes returned by
 * the WASM client (`serde_wasm_bindgen`). The Rust layer owns this state; these
 * types just describe what the snapshot looks like on the wire.
 *
 * @module
 */

export type SourceNetwork = "Hashiverse" | "Nostr" | "Mastodon" | "Bluesky";

export interface SourceConfig {
	network: SourceNetwork;
	id: string;
}

export interface TimelineConfig {
	id: string;
	/** The user-set custom name, or null when using the source-derived default. */
	name: string | null;
	/** The effective name to display (custom name, else the default from sources). */
	display_name: string;
	sources: SourceConfig[];
}
