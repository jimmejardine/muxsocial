import type { GoogleFontSpec } from "./types.ts";

/**
 * Build a Google Fonts CSS2 URL for the given families.
 *
 * Pure function (no DOM) so it can be unit-tested directly. Spaces in family
 * names become "+", weights are sorted and joined with ";", and `display=swap`
 * is appended so text renders in a fallback font until the web font arrives
 * (avoids invisible text on theme switch).
 *
 * @example
 * buildGoogleFontsUrl([{ family: "Chakra Petch", weights: [700, 400] }])
 * // => "https://fonts.googleapis.com/css2?family=Chakra+Petch:wght@400;700&display=swap"
 */
export function buildGoogleFontsUrl(fonts: GoogleFontSpec[]): string {
	const families = fonts.map((font) => {
		const family_param = font.family.trim().replace(/\s+/g, "+");
		const weights = [...font.weights].sort((a, b) => a - b).join(";");
		return `family=${family_param}:wght@${weights}`;
	});
	return `https://fonts.googleapis.com/css2?${families.join("&")}&display=swap`;
}

/** Families already requested this session, so re-activating a theme is a no-op. */
const loaded_families = new Set<string>();

/** Insert the one-time `<link rel="preconnect">` hints for the Google Fonts CDNs. */
function ensure_preconnect(): void {
	if (document.querySelector("link[data-mux-font-preconnect]")) {
		return;
	}
	for (const [href, crossorigin] of [
		["https://fonts.googleapis.com", false],
		["https://fonts.gstatic.com", true],
	] as const) {
		const link = document.createElement("link");
		link.rel = "preconnect";
		link.href = href;
		link.setAttribute("data-mux-font-preconnect", "");
		if (crossorigin) {
			link.crossOrigin = "anonymous";
		}
		document.head.appendChild(link);
	}
}

/**
 * Idempotently load the given Google Fonts at runtime.
 *
 * Injects the preconnect hints (once) and a single stylesheet `<link>` covering
 * any not-yet-loaded families. Calling it again with already-loaded families
 * does nothing, so it is safe to call on every theme change.
 */
export function ensureGoogleFonts(fonts: GoogleFontSpec[]): void {
	const pending = fonts.filter((font) => !loaded_families.has(font.family));
	if (pending.length === 0) {
		return;
	}

	ensure_preconnect();

	const link = document.createElement("link");
	link.rel = "stylesheet";
	link.href = buildGoogleFontsUrl(pending);
	link.setAttribute("data-mux-font", pending.map((font) => font.family).join(","));
	document.head.appendChild(link);

	for (const font of pending) {
		loaded_families.add(font.family);
	}
}

/** Test-only: reset the in-memory loaded-families cache. */
export function resetLoadedFontsForTests(): void {
	loaded_families.clear();
}
