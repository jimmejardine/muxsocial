import { describe, expect, it } from "vitest";
import { compose_config_text, parse_config_text } from "./ConfigTransfer.ts";

const VALID_THEME_IDS = ["light", "dark", "electric"];
const VALID_LANGUAGE_CODES = ["en", "de", "es", "fr"];

const TIMELINES_JSON = JSON.stringify([{ id: "abc", name: null, sources: [{ network: "Nostr", id: "npub1xyz" }], autopoll: true }]);

describe("compose_config_text", () => {
	it("embeds the timelines and settings under the two roots, pretty-printed", () => {
		const config_text = compose_config_text(TIMELINES_JSON, { theme: "electric", language: "en" });
		const parsed = JSON.parse(config_text);
		expect(parsed.timelines).toEqual(JSON.parse(TIMELINES_JSON));
		expect(parsed.settings).toEqual({ theme: "electric", language: "en" });
		// Pretty-printed for human copy/paste editing.
		expect(config_text).toContain("\n");
	});

	it("round-trips through parse_config_text", () => {
		const config_text = compose_config_text(TIMELINES_JSON, { theme: "dark", language: "de" });
		const parsed = parse_config_text(config_text, VALID_THEME_IDS, VALID_LANGUAGE_CODES);
		expect(JSON.parse(parsed.timelines_json)).toEqual(JSON.parse(TIMELINES_JSON));
		expect(parsed.settings).toEqual({ theme: "dark", language: "de" });
	});
});

describe("parse_config_text", () => {
	it("accepts settings with only some preferences set", () => {
		const parsed = parse_config_text(JSON.stringify({ timelines: [], settings: { theme: "light" } }), VALID_THEME_IDS, VALID_LANGUAGE_CODES);
		expect(parsed.settings).toEqual({ theme: "light" });
		expect(parsed.settings.language).toBeUndefined();
	});

	it("accepts empty settings and an empty timeline list", () => {
		const parsed = parse_config_text(JSON.stringify({ timelines: [], settings: {} }), VALID_THEME_IDS, VALID_LANGUAGE_CODES);
		expect(parsed.timelines_json).toBe("[]");
		expect(parsed.settings).toEqual({});
	});

	it("ignores unknown settings keys", () => {
		const parsed = parse_config_text(JSON.stringify({ timelines: [], settings: { theme: "dark", future_pref: 42 } }), VALID_THEME_IDS, VALID_LANGUAGE_CODES);
		expect(parsed.settings).toEqual({ theme: "dark" });
	});

	it("rejects text that is not JSON", () => {
		expect(() => parse_config_text("not json", VALID_THEME_IDS, VALID_LANGUAGE_CODES)).toThrow(/not valid JSON/);
	});

	it("rejects a non-object root", () => {
		expect(() => parse_config_text("[1, 2]", VALID_THEME_IDS, VALID_LANGUAGE_CODES)).toThrow(/config root/);
		expect(() => parse_config_text("null", VALID_THEME_IDS, VALID_LANGUAGE_CODES)).toThrow(/config root/);
	});

	it("rejects a missing or non-array timelines root", () => {
		expect(() => parse_config_text(JSON.stringify({ settings: {} }), VALID_THEME_IDS, VALID_LANGUAGE_CODES)).toThrow(/"timelines" must be a JSON array/);
		expect(() => parse_config_text(JSON.stringify({ timelines: {}, settings: {} }), VALID_THEME_IDS, VALID_LANGUAGE_CODES)).toThrow(/"timelines" must be a JSON array/);
	});

	it("rejects a missing or non-object settings root", () => {
		expect(() => parse_config_text(JSON.stringify({ timelines: [] }), VALID_THEME_IDS, VALID_LANGUAGE_CODES)).toThrow(/"settings" must be a JSON object/);
		expect(() => parse_config_text(JSON.stringify({ timelines: [], settings: [] }), VALID_THEME_IDS, VALID_LANGUAGE_CODES)).toThrow(/"settings" must be a JSON object/);
	});

	it("rejects an unknown theme or language", () => {
		expect(() => parse_config_text(JSON.stringify({ timelines: [], settings: { theme: "neon" } }), VALID_THEME_IDS, VALID_LANGUAGE_CODES)).toThrow(/unknown theme "neon"/);
		expect(() => parse_config_text(JSON.stringify({ timelines: [], settings: { theme: 7 } }), VALID_THEME_IDS, VALID_LANGUAGE_CODES)).toThrow(/unknown theme 7/);
		expect(() => parse_config_text(JSON.stringify({ timelines: [], settings: { language: "xx" } }), VALID_THEME_IDS, VALID_LANGUAGE_CODES)).toThrow(/unknown language "xx"/);
	});
});
