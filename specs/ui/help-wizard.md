# Getting-started help wizard

A paged onboarding overlay (`components/HelpWizard.tsx`) that introduces the app. It is always
a Mantine `Modal` — never inline — and is reachable three ways:

- **Auto-open, once**: when the app first loads with zero timelines (`App.tsx` guards with a
  ref so it doesn't re-pop on later renders).
- The **"Getting started"** entry at the top of the toolbar hamburger
  ([timelines.md](timelines.md)).
- The **"Getting started"** button in the empty-timelines state.

## Pages

`WIZARD_PAGES` is a data array of `{ image, textKey }` — four pages, each an image above
centred text:

1. What mux.social is for — the `muxsocial.jpg` welcome banner.
2. The **timelines** concept — `/img/wizard/timelines.jpg`.
3. **Adding an address** (the "+" button / paste button) — `/img/wizard/add-source.jpg`.
4. All done — `/img/wizard/done.jpg`; the copy points the user at the pulsing
   "Add timeline" button and the pulsing "+" button (the wizard takes **no action itself** —
   its final button just closes).

All images share the welcome banner's 1024×512 dimensions and visual style (the robot-octopus
mascot on the dark teal hexagon/neon look). Page copy lives in flat `wizard.*`
[i18n keys](localization.md).

## Behaviour and layout

- Footer nav: **Back** (disabled on the first page) and **Next**, becoming **"Got it"** (close)
  on the last page; a row of step dots shows progress (active dot in the primary color).
- The content is keyed on the modal's `opened` flag so every open resets to page 1.
- The modal renders **no header** (a title bar wastes vertical space): `withCloseButton`
  is off and a `CloseButton` is overlaid in the body's top-right corner (inset 1.5× the body
  padding so it sits on the image), with Escape/backdrop close still working.
- The page image fills the content width so the modal body's uniform padding gives equal
  spacing around it; the text block reserves a min-height so paging doesn't jump the layout
  (`HelpWizard.module.css`).
