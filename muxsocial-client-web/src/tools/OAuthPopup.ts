/**
 * Drives the OAuth authorize step in a popup window. The worker builds the
 * authorize URL and does all token work; this only opens the popup, waits for the
 * static `/oauth-callback.html` page to hand back the redirect query
 * (`?code=…&state=…`), and resolves with it.
 *
 * The callback result arrives over a same-origin `BroadcastChannel` rather than
 * `window.opener` / `popup.closed`: some auth servers (Bluesky) send
 * `Cross-Origin-Opener-Policy: same-origin`, which severs the opener↔popup
 * relationship — nulling `window.opener` in the callback and making `popup.closed`
 * read `true` while the window is still open. The BroadcastChannel is origin-scoped
 * and immune to COOP. A `window` `"message"` listener is kept as a fallback for
 * providers that don't sever the opener.
 *
 * @module
 */

/** How long to wait for the callback before giving up (real logins can be slow:
 * password managers, 2FA, account pickers). */
const OAUTH_TIMEOUT_MS = 5 * 60 * 1000;

/** The redirect URI registered for OAuth flows — a static same-origin page. */
export function oauth_redirect_uri(): string {
	return `${window.location.origin}/oauth-callback.html`;
}

/**
 * Open `authorize_url` in a popup and resolve with the callback query string.
 * Rejects if the popup is blocked or the authorization times out.
 */
export function open_oauth_popup(authorize_url: string): Promise<string> {
	return new Promise<string>((resolve, reject) => {
		const popup = window.open(authorize_url, "muxsocial_oauth", "width=600,height=760");
		if (!popup) {
			reject(new Error("Popup blocked. Allow popups for this site and try again."));
			return;
		}

		const channel = new BroadcastChannel("muxsocial-oauth");
		let settled = false;

		const cleanup = () => {
			window.clearTimeout(timeout);
			channel.close();
			window.removeEventListener("message", on_message);
			try {
				popup.close();
			} catch {
				// Already closed (or severed by COOP) — nothing to do.
			}
		};

		// Accept the callback from either the BroadcastChannel (primary) or a
		// same-origin window message (fallback); whichever arrives first wins.
		const handle_callback = (data: { type?: string; query?: string } | null) => {
			if (settled || data?.type !== "muxsocial-oauth-callback") return;
			settled = true;
			cleanup();
			resolve(data.query ?? "");
		};

		const on_channel_message = (event: MessageEvent) => handle_callback(event.data as { type?: string; query?: string } | null);
		const on_message = (event: MessageEvent) => {
			if (event.origin !== window.location.origin) return;
			handle_callback(event.data as { type?: string; query?: string } | null);
		};

		// Under COOP the opener can't observe the popup, so a closed popup can't be
		// detected; fall back to an overall timeout instead.
		const timeout = window.setTimeout(() => {
			if (settled) return;
			settled = true;
			cleanup();
			reject(new Error("Authorization timed out — please try again."));
		}, OAUTH_TIMEOUT_MS);

		channel.onmessage = on_channel_message;
		window.addEventListener("message", on_message);
	});
}
