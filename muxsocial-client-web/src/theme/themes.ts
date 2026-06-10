import { createTheme, type MantineColorsTuple } from "@mantine/core";
import type { MuxTheme } from "./types.ts";

const INTER = { family: "Inter", weights: [400, 500, 600, 700] };
const CHAKRA_PETCH = { family: "Chakra Petch", weights: [400, 500, 600, 700] };

// A neon cyan ramp (light → dark) for the electric theme's primary color.
const ELECTRIC_CYAN: MantineColorsTuple = ["#e1fcff", "#c8f6ff", "#94ecff", "#5ce2ff", "#30daff", "#16d6ff", "#00d3ff", "#00b6df", "#00a1c8", "#0089b0"];

/**
 * The built-in themes. Order is the order shown in the switcher.
 *
 * Adding a theme is a matter of appending one {@link MuxTheme} object here — no
 * other code changes are required.
 */
export const MUX_THEMES: MuxTheme[] = [
	{
		id: "light",
		label: "Light",
		colorScheme: "light",
		fonts: [INTER],
		mantineTheme: createTheme({
			primaryColor: "blue",
			fontFamily: "Inter, sans-serif",
			headings: { fontFamily: "Inter, sans-serif" },
		}),
		appBackground: "var(--mantine-color-gray-0)",
	},
	{
		id: "dark",
		label: "Dark",
		colorScheme: "dark",
		fonts: [INTER],
		mantineTheme: createTheme({
			primaryColor: "blue",
			fontFamily: "Inter, sans-serif",
			headings: { fontFamily: "Inter, sans-serif" },
		}),
		appBackground: "var(--mantine-color-dark-7)",
	},
	{
		id: "electric",
		label: "Electric",
		colorScheme: "dark",
		fonts: [CHAKRA_PETCH],
		mantineTheme: createTheme({
			primaryColor: "electric",
			primaryShade: { light: 6, dark: 5 },
			colors: { electric: ELECTRIC_CYAN },
			fontFamily: "Chakra Petch, sans-serif",
			headings: { fontFamily: "Chakra Petch, sans-serif", fontWeight: "600" },
			defaultRadius: "xs",
		}),
		appBackground: "radial-gradient(circle at 20% 0%, rgba(0, 211, 255, 0.18), transparent 55%), radial-gradient(circle at 90% 100%, rgba(214, 0, 255, 0.16), transparent 50%), #05060a",
	},
];

/** The theme used when nothing is stored and on unknown ids. */
export const DEFAULT_THEME_ID = "dark";

/** Look up a theme by id, falling back to the default theme. */
export function resolveTheme(id: string | null | undefined): MuxTheme {
	const found = MUX_THEMES.find((theme) => theme.id === id);
	if (found) {
		return found;
	}
	const fallback = MUX_THEMES.find((theme) => theme.id === DEFAULT_THEME_ID);
	if (!fallback) {
		throw new Error(`Default theme "${DEFAULT_THEME_ID}" missing from registry`);
	}
	return fallback;
}
