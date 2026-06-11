# Localization (i18n)

The GUI is localized with [i18next](https://www.i18next.com/) + `react-i18next`. English is
bundled; other languages are fetched at runtime, so adding a language ships no extra JS.

## Setup

`i18n/i18n.ts` initialises i18next as an import side effect (`index.tsx` imports it before the
first render):

- **English** (`i18n/locales/en.json`) is bundled as the in-app resource and the
  `fallbackLng`. The other languages — currently German, Spanish, French — are loaded on demand
  by `i18next-http-backend` from `/locales/{{lng}}.json`. `partialBundledLanguages` + no
  Suspense boundary means the bundled English shows while a newly-selected language loads.
- `i18n/locales/manifest.json` lists the supported language codes. `SUPPORTED_LANGUAGES` maps
  it to `{ value, label }` options where the label is each language's **autonym** (its own name
  for itself, via `Intl.DisplayNames`).
- `detect_language()` picks the initial language: an explicit stored choice wins, else the first
  matching browser language, else English. `set_language(code)` switches and persists the choice
  to `localStorage["muxsocial.language"]`.
- `keySeparator` is off, so keys are flat dotted strings (`"toast.timeline_added"`,
  `"timeline.get_more"`, …).

## Switcher

`components/LanguageSwitcher.tsx` (in the [toolbar](timelines.md)) lists `SUPPORTED_LANGUAGES`
and calls `set_language`.

## Notes

- Translation keys live in `i18n/locales/*.json`; the runtime-fetched copies are served from
  `public/locales/`.
- Relative timestamps are localized separately by `Intl.RelativeTimeFormat` keyed on the active
  language, not by translation keys (see [posts.md](posts.md)).
