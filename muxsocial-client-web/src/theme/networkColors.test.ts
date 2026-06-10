import { describe, expect, it } from "vitest";
import type { SourceNetwork } from "../tools/TimelineConfig.ts";
import { NETWORK_COLORS, networkColor } from "./networkColors.ts";

const ALL_NETWORKS: SourceNetwork[] = ["Hashiverse", "Nostr", "Mastodon", "Bluesky"];

describe("networkColor", () => {
	it("returns the expected hex for each network", () => {
		expect(networkColor("Hashiverse")).toBe("#63E6BE");
		expect(networkColor("Nostr")).toBe("#B197FC");
		expect(networkColor("Mastodon")).toBe("#91A7FF");
		expect(networkColor("Bluesky")).toBe("#74C0FC");
	});
});

describe("NETWORK_COLORS", () => {
	it("maps every network to a hex color", () => {
		for (const network of ALL_NETWORKS) {
			expect(NETWORK_COLORS[network]).toMatch(/^#[0-9A-Fa-f]{6}$/);
		}
	});

	it("has no extra keys beyond the known networks", () => {
		expect(Object.keys(NETWORK_COLORS).sort()).toEqual([...ALL_NETWORKS].sort());
	});
});
