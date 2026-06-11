/**
 * Display helpers for a timeline source's id in the chip.
 *
 * @module
 */

import type { SourceNetwork } from "./TimelineConfig.ts";

/**
 * Shorten long opaque ids (Nostr `npub…`, Hashiverse 64-hex) to `first8…last3`
 * (e.g. `npub1xxx…xxx`). Mastodon `@user@instance` and Bluesky handles are
 * human-readable and returned unchanged. The full id stays available as the chip's
 * tooltip.
 */
export function truncate_source_id(network: SourceNetwork, id: string): string {
	const is_opaque = network === "Nostr" || network === "Hashiverse";
	if (is_opaque && id.length > 11) {
		return `${id.slice(0, 8)}…${id.slice(-3)}`;
	}
	return id;
}
