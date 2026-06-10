import type { MantineColorScheme, MantineThemeOverride } from "@mantine/core";

/** A Google Font family and the weights to request from the CSS2 API. */
export interface GoogleFontSpec {
	/** The family name as Google Fonts spells it, e.g. "Chakra Petch". */
	family: string;
	/** Numeric weights to load, e.g. [400, 500, 700]. */
	weights: number[];
}

/**
 * A "mux theme": a self-contained, data-driven theme definition.
 *
 * It bundles everything that distinguishes one look from another — the Mantine
 * theme object, a forced color scheme, the fonts to pull from Google Fonts, and
 * the page background — so a new theme is added by dropping one object into the
 * registry in {@link ./themes}.
 */
export interface MuxTheme {
	/** Stable id used for persistence and lookup, e.g. "electric". */
	id: string;
	/** Human-readable label shown in the theme switcher. */
	label: string;
	/** The color scheme this theme forces on Mantine. */
	colorScheme: Exclude<MantineColorScheme, "auto">;
	/** Fonts loaded from Google Fonts when the theme activates. */
	fonts: GoogleFontSpec[];
	/** Mantine theme overrides (colors, fontFamily, radius, …). */
	mantineTheme: MantineThemeOverride;
	/** CSS value applied to the `--mux-app-bg` custom property on `<body>`. */
	appBackground: string;
}
