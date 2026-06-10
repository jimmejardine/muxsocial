import { MantineProvider } from "@mantine/core";
import { createContext, type ReactNode, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { ensureGoogleFonts } from "./googleFonts.ts";
import { DEFAULT_THEME_ID, MUX_THEMES, resolveTheme } from "./themes.ts";
import type { MuxTheme } from "./types.ts";

const STORAGE_KEY = "mux-theme";

interface MuxThemeContextValue {
	/** The currently active theme. */
	theme: MuxTheme;
	/** The active theme's id. */
	themeId: string;
	/** Switch to the theme with the given id (persisted to localStorage). */
	setThemeId: (id: string) => void;
	/** All available themes, in switcher order. */
	themes: MuxTheme[];
}

const MuxThemeContext = createContext<MuxThemeContextValue | null>(null);

/** Pick the initial theme id: stored choice, else the OS color-scheme preference. */
function initial_theme_id(): string {
	const stored = localStorage.getItem(STORAGE_KEY);
	if (stored && MUX_THEMES.some((theme) => theme.id === stored)) {
		return stored;
	}
	const prefers_dark = window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
	return prefers_dark ? "dark" : "light";
}

/**
 * Provides the active mux theme and renders the Mantine provider for it.
 *
 * Replaces the bare `<MantineProvider>`: it drives Mantine's `theme` and
 * `forceColorScheme` from the selected {@link MuxTheme}, loads that theme's
 * Google Fonts at runtime, persists the choice, and paints the page background.
 */
export function MuxThemeProvider({ children }: { children: ReactNode }) {
	const [themeId, setThemeIdState] = useState<string>(initial_theme_id);

	const theme = useMemo(() => resolveTheme(themeId), [themeId]);

	const setThemeId = useCallback((id: string) => {
		setThemeIdState(resolveTheme(id).id);
	}, []);

	useEffect(() => {
		ensureGoogleFonts(theme.fonts);
		localStorage.setItem(STORAGE_KEY, theme.id);
		document.body.style.setProperty("--mux-app-bg", theme.appBackground);
	}, [theme]);

	const value = useMemo<MuxThemeContextValue>(() => ({ theme, themeId: theme.id, setThemeId, themes: MUX_THEMES }), [theme, setThemeId]);

	return (
		<MuxThemeContext.Provider value={value}>
			<MantineProvider theme={theme.mantineTheme} forceColorScheme={theme.colorScheme}>
				{children}
			</MantineProvider>
		</MuxThemeContext.Provider>
	);
}

/** Access the active theme and switcher from within the provider. */
export function useMuxTheme(): MuxThemeContextValue {
	const value = useContext(MuxThemeContext);
	if (!value) {
		throw new Error("useMuxTheme must be used within a MuxThemeProvider");
	}
	return value;
}

export { DEFAULT_THEME_ID };
