import { describe, expect, it } from "vitest";
import type { SourceNetwork } from "../tools/TimelineConfig.ts";
import { NETWORK_COLORS, networkColor } from "./networkColors.ts";

const ALL_NETWORKS: SourceNetwork[] = ["Hashiverse", "Nostr", "Mastodon", "Bluesky"];

describe("networkColor", () => {
	it("returns the expected Mantine color name for each network", () => {
		expect(networkColor("Hashiverse")).toBe("teal");
		expect(networkColor("Nostr")).toBe("violet");
		expect(networkColor("Mastodon")).toBe("indigo");
		expect(networkColor("Bluesky")).toBe("blue");
	});
});

describe("NETWORK_COLORS", () => {
	it("maps every network to a non-empty color name", () => {
		for (const network of ALL_NETWORKS) {
			expect(NETWORK_COLORS[network].length).toBeGreaterThan(0);
		}
	});

	it("has no extra keys beyond the known networks", () => {
		expect(Object.keys(NETWORK_COLORS).sort()).toEqual([...ALL_NETWORKS].sort());
	});
});
