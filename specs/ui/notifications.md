# Notifications (toasts)

mux.social surfaces success and error feedback as transient toasts, mirroring hashiverse's
`Toast` utility over Mantine's notifications.

`tools/Toast.ts` exposes two calls:

- `Toast.success(message)` — green, auto-closes after 3s.
- `Toast.error(message)` — red, auto-closes after 5s.

Both use `@mantine/notifications` (the `<Notifications position="bottom-center" />` provider is
mounted once in `index.tsx`). Messages are passed in already-localized — callers use i18n keys
(see [localization.md](localization.md)), e.g. `App.tsx` toasts the outcome of each timeline
command (`toast.timeline_added`, `toast.error_add_source`, …) and `usePosts` toasts paging
failures.
