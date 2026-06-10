import { describe, expect, it } from "vitest";
import { DEFAULT_THEME_ID, MUX_THEMES, resolveTheme } from "./themes.ts";

describe("theme registry", () => {
	it("ships the three built-in themes", () => {
		const ids = MUX_THEMES.map((theme) => theme.id);
		expect(ids).toEqual(["light", "dark", "electric"]);
	});

	it("gives every theme a valid color scheme, fonts, and label", () => {
		for (const theme of MUX_THEMES) {
			expect(["light", "dark"]).toContain(theme.colorScheme);
			expect(theme.fonts.length).toBeGreaterThan(0);
			expect(theme.label.length).toBeGreaterThan(0);
			expect(theme.appBackground.length).toBeGreaterThan(0);
		}
	});

	it("uses a different font for the electric theme", () => {
		const electric = resolveTheme("electric");
		const light = resolveTheme("light");
		expect(electric.fonts[0].family).toBe("Chakra Petch");
		expect(light.fonts[0].family).not.toBe(electric.fonts[0].family);
	});
});

describe("resolveTheme", () => {
	it("returns the requested theme when it exists", () => {
		expect(resolveTheme("electric").id).toBe("electric");
	});

	it("falls back to the default theme for unknown or missing ids", () => {
		expect(resolveTheme("nonsense").id).toBe(DEFAULT_THEME_ID);
		expect(resolveTheme(null).id).toBe(DEFAULT_THEME_ID);
		expect(resolveTheme(undefined).id).toBe(DEFAULT_THEME_ID);
	});
});
