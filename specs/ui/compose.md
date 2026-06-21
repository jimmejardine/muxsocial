# Compose & accounts UI

The GUI for cross-posting, in `muxsocial-client-web/src`.

## Post button

`Toolbar.tsx` shows a **Post** button (quill/pen glyph) to the **left of the
"Add timeline" button**; it opens `ComposeModal`. "My accounts" also lives in the
hamburger (`HeaderMenu.tsx`).

## ComposeModal — `components/ComposeModal.tsx`

A Mantine `Modal` with a `Textarea` and footer buttons **Post**, **Cancel**, and
**My accounts**.

- **Post** → `muxsocial.cross_post(text)`, then renders the returned
  `PostResult[]` inline (one row per account: `network · label` with a green
  "posted" or red "failed — …"). On all-success it clears the draft.
- **Cancel** → closes but **keeps the draft**: the text is persisted to
  `localStorage` under `"muxsocial.compose_draft"` on every change (GUI-owned,
  not secret), so reopening — or a reload — restores it.
- **My accounts** → opens `AccountsModal`.
- **Unlock gate**: on open it calls `is_unlocked`; if accounts exist and the
  session is locked it shows a master-password field (→ `unlock_secrets`). Post is
  disabled until unlocked, and shows a "will broadcast to N accounts" hint once
  unlocked. With no accounts it points the user at "My accounts".

## AccountsModal — `components/AccountsModal.tsx`

- **Four Add buttons**, one per network (icons supplied later). Hashiverse
  (keyphrase) and nostr (nsec) reveal an inline form that pastes the secret and,
  if the session is locked, the master password (to encrypt). Mastodon and
  Bluesky use OAuth and are disabled ("Coming soon") until that flow ships.
- **Connected accounts** list — network + `display_label`, each with a **Remove**
  button (→ `remove_account`).
- Add/remove use success/error toasts (`tools/Toast.ts`); the inline forms show
  errors (e.g. a wrong master password) in an alert.

## Types & i18n

- `tools/PostingTypes.ts` mirrors the Rust serde shapes (`AccountView`,
  `PostResult`, `PostOutcome`, `SourceNetwork`) — the wasm-bindgen `.d.ts` types
  these returns only as `any`.
- Strings are under the `compose.*` and `accounts.*` keys in
  `src/i18n/locales/en.json`; other languages fall back to English until
  translated through the localization pipeline (see [localization.md](localization.md)).

## OAuth callback (planned)

The Mastodon/Bluesky OAuth stages add `tools/OAuthPopup.ts` and an
`/oauth/callback` route: the popup lands on the callback, `postMessage`s the
redirect query to `window.opener`, and closes; the worker does the token work.
</content>
