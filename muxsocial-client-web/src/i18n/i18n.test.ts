import { describe, expect, it } from "vitest";
import de from "../../public/locales/de.json";
import es from "../../public/locales/es.json";
import fr from "../../public/locales/fr.json";
import i18n, { SUPPORTED_LANGUAGES } from "./i18n.ts";
import en from "./locales/en.json";
import manifest from "./locales/manifest.json";

/** The fetched-at-runtime locales, keyed by code, for parity checks. */
const fetched_locales: Record<string, Record<string, string>> = { de, es, fr };

describe("i18n configuration", () => {
	it("exposes one switcher option per manifest language, each with a non-empty label", () => {
		expect(SUPPORTED_LANGUAGES.map((language) => language.value)).toEqual(manifest);
		for (const language of SUPPORTED_LANGUAGES) {
			expect(language.label.length).toBeGreaterThan(0);
		}
	});

	it("resolves a bundled English key by default", () => {
		expect(i18n.t("toolbar.add_timeline")).toBe("Add timeline");
	});

	it("ships a fetched locale for every non-English manifest language", () => {
		for (const language_code of manifest.filter((code) => code !== "en")) {
			expect(fetched_locales[language_code], `missing locale file for ${language_code}`).toBeDefined();
		}
	});

	it("keeps every fetched locale in key-parity with English", () => {
		const english_keys = Object.keys(en).sort();
		for (const [language_code, locale] of Object.entries(fetched_locales)) {
			const locale_keys = Object.keys(locale).sort();
			expect(locale_keys, `locale ${language_code} keys must match en`).toEqual(english_keys);
		}
	});
});
