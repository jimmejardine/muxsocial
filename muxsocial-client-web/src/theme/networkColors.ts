/**
 * Fixed per-network colors, used for the post source bar and the timeline source
 * chips. These are intentionally theme-independent (raw hex, not Mantine theme
 * colors) so a network reads as the same color across the light/dark/electric
 * themes. The values are pastel versions of each network's brand hue.
 *
 * @module
 */

import type { SourceNetwork } from "../tools/TimelineConfig.ts";

/** Pastel per-network brand colors. */
export const NETWORK_COLORS: Record<SourceNetwork, string> = {
	Hashiverse: "#63E6BE",
	Nostr: "#B197FC",
	Mastodon: "#91A7FF",
	Bluesky: "#74C0FC",
};

/** The color for a given network. */
export function networkColor(network: SourceNetwork): string {
	return NETWORK_COLORS[network];
}
