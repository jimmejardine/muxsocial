import { describe, expect, it } from "vitest";
import { truncate_source_id } from "./sourceLabel.ts";

describe("truncate_source_id", () => {
	it("shortens long Nostr and Hashiverse ids to first8…last3", () => {
		expect(truncate_source_id("Nostr", "npub1wmr34t36fy03m8hvgl96zl3znndyzyaqhwmwdtshwmtkg03fetaqhjg240")).toBe("npub1wmr…240");
		expect(truncate_source_id("Hashiverse", "ddd86177f252f0d33f32aa3e59fb6b554969faad48af443347c5b72ac2e186f0")).toBe("ddd86177…6f0");
	});

	it("leaves human-readable Mastodon/Bluesky ids in full", () => {
		expect(truncate_source_id("Mastodon", "@Gargron@mastodon.social")).toBe("@Gargron@mastodon.social");
		expect(truncate_source_id("Bluesky", "jay.bsky.team")).toBe("jay.bsky.team");
	});

	it("leaves a short opaque id unchanged", () => {
		expect(truncate_source_id("Nostr", "npub1abc")).toBe("npub1abc");
	});
});
