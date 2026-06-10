/**
 * Per-network colors for the post source bar and the timeline source chips.
 *
 * Each network maps to a built-in Mantine color name (not a raw hex) so both the
 * source bar and the chips can use Mantine's `light` variant — a tinted background
 * with a darker same-hue text — which Mantine derives per color scheme. This keeps
 * the bar and the chips visually identical across the light/dark/electric themes.
 *
 * The chosen Mantine ramps are the pastel-brand hues confirmed with the user:
 * Hashiverse → teal, Nostr → violet, Mastodon → indigo, Bluesky → blue.
 *
 * @module
 */

import type { SourceNetwork } from "../tools/TimelineConfig.ts";

/** Per-network Mantine color names. */
export const NETWORK_COLORS: Record<SourceNetwork, string> = {
	Hashiverse: "teal",
	Nostr: "violet",
	Mastodon: "indigo",
	Bluesky: "blue",
};

/** The Mantine color name for a given network. */
export function networkColor(network: SourceNetwork): string {
	return NETWORK_COLORS[network];
}
