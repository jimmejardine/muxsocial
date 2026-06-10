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
	sources: SourceConfig[];
}
