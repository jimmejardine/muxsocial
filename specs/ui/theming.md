# Theming

mux.social ships **pluggable themes** — colours, fonts, and backgrounds — so the look is
easy to extend and customise. Three themes ship built-in: **Light**, **Dark**, and
**Electric** (a neon, dark-based look on a distinct font).

All theme code lives in `muxsocial-client-web/src/theme/`.

## The mux theme model

In Mantine 8, light/dark is a **color scheme** axis, separate from the **theme object**
(`createTheme`: `primaryColor`, `colors`, `fontFamily`, `headings`, `defaultRadius`, …). A
**mux theme** bundles both, plus the fonts to load and the page background, into one
data object (`src/theme/types.ts`):

```ts
interface MuxTheme {
  id: string;                       // "light" | "dark" | "electric"
  label: string;                    // shown in the switcher
  colorScheme: "light" | "dark";    // forced via MantineProvider
  fonts: GoogleFontSpec[];          // loaded from Google Fonts on activation
  mantineTheme: MantineThemeOverride;
  appBackground: string;            // CSS value for the --mux-app-bg custom property
}
```

## Data-driven registry

`src/theme/themes.ts` exports `MUX_THEMES` (the ordered list), `DEFAULT_THEME_ID`, and
`resolveTheme(id)` (lookup with fallback to the default). **Adding a theme is appending one
`MuxTheme` object** — no other code changes. Electric defines a custom 10-shade
`colors.electric` neon palette set as `primaryColor`, uses Chakra Petch, and renders a
layered radial-gradient background; light/dark share Inter and Mantine's `blue`.

## Runtime Google Fonts loading

Fonts are pulled from Google Fonts at runtime — no build step, no bundled font files
(`src/theme/googleFonts.ts`):

- `buildGoogleFontsUrl(fonts)` — pure function producing a CSS2 URL (spaces → `+`, weights
  joined with `;`, `&display=swap` to avoid invisible text).
- `ensureGoogleFonts(fonts)` — idempotently injects the one-time `<link rel="preconnect">`
  hints and a stylesheet `<link>` for any not-yet-loaded families, tracking loaded families
  so re-activating a theme is a no-op.

## Provider and persistence

`src/theme/MuxThemeProvider.tsx` replaces the bare `<MantineProvider>`. It holds the active
theme id, drives Mantine's `theme` and `forceColorScheme` from the selected `MuxTheme`,
loads that theme's fonts, sets `--mux-app-bg` on `<body>` (painted by an `index.css` rule),
and exposes `useMuxTheme()`. The `ThemeSwitcher` in the toolbar (`src/components/`) lists
the registry and calls `setThemeId`.

The chosen theme persists to `localStorage["mux-theme"]`; with nothing stored, newcomers get
the default theme (`DEFAULT_THEME_ID` — **Electric**, which is also the fallback for unknown
stored ids). When configuration is exposed to the TS GUI
over the worker RPC (see [../architecture/config-storage.md](../architecture/config-storage.md)),
persistence can move to `ConfigStorage` — a one-line swap in the provider.

## Tests

`src/theme/*.test.ts` (Vitest + jsdom) cover the Google Fonts URL builder, idempotent
font injection, and registry resolution/fallback.
