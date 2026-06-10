/**
 * i18next initialisation for mux.social.
 *
 * English is bundled with the app; other languages listed in
 * {@link ./locales/manifest.json} are fetched at runtime from
 * `/locales/<lang>.json` (served from `public/locales/`) via
 * `i18next-http-backend`. Importing this module initialises i18next as a side
 * effect — `src/index.tsx` imports it before the first render.
 *
 * @module
 */

import i18n from "i18next";
import HttpBackend from "i18next-http-backend";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import manifest from "./locales/manifest.json";

/** localStorage key holding the user's explicit language choice. */
export const LANGUAGE_STORAGE_KEY = "muxsocial.language";

/** The set of supported language codes (from the manifest). */
const supported_language_codes = new Set<string>(manifest);

/** The language's own name for itself, e.g. "de" -> "Deutsch". */
function language_autonym(language_code: string): string {
	try {
		const display_name = new Intl.DisplayNames([language_code], { type: "language" }).of(language_code);
		if (!display_name) return language_code;
		return display_name.charAt(0).toLocaleUpperCase(language_code) + display_name.slice(1);
	} catch {
		return language_code;
	}
}

/** `{ value, label }` options for a language switcher, in manifest order. */
export const SUPPORTED_LANGUAGES = manifest.map((language_code: string) => ({ value: language_code, label: language_autonym(language_code) }));

/**
 * Pick the initial language: an explicit stored choice wins, otherwise the
 * first supported browser language (exact then base code), otherwise English.
 * Synchronous (localStorage + navigator are sync) so the app entry stays sync.
 */
function detect_language(): string {
	const stored_language = localStorage.getItem(LANGUAGE_STORAGE_KEY);
	if (stored_language && supported_language_codes.has(stored_language)) {
		return stored_language;
	}

	for (const browser_language of navigator.languages ?? [navigator.language]) {
		if (supported_language_codes.has(browser_language)) return browser_language;
		const base_code = browser_language.split("-")[0];
		if (supported_language_codes.has(base_code)) return base_code;
	}

	return "en";
}

/** Switch language and remember the choice. */
export function set_language(language_code: string): void {
	i18n.changeLanguage(language_code);
	try {
		localStorage.setItem(LANGUAGE_STORAGE_KEY, language_code);
	} catch {
		// Ignore storage failures (e.g. private mode); the switch still applies.
	}
}

i18n
	.use(HttpBackend)
	.use(initReactI18next)
	.init({
		lng: detect_language(),
		fallbackLng: "en",
		// English is bundled below; the rest are fetched, so allow partial bundles.
		partialBundledLanguages: true,
		keySeparator: false,
		resources: {
			en: { translation: en },
		},
		backend: {
			loadPath: "/locales/{{lng}}.json",
		},
		interpolation: { escapeValue: false }, // React already escapes
		// No Suspense boundary: while a newly-selected language loads, the bundled
		// English fallback is shown instead of suspending.
		react: { useSuspense: false },
	});

export default i18n;
