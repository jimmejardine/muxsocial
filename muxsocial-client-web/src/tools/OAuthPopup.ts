/**
 * Drives the OAuth authorize step in a popup window. The worker builds the
 * authorize URL and does all token work; this only opens the popup, waits for the
 * static `/oauth-callback.html` page to `postMessage` back the redirect query
 * (`?code=…&state=…`), and resolves with it.
 *
 * @module
 */

/** The redirect URI registered for OAuth flows — a static same-origin page. */
export function oauth_redirect_uri(): string {
	return `${window.location.origin}/oauth-callback.html`;
}

/**
 * Open `authorize_url` in a popup and resolve with the callback query string.
 * Rejects if the popup is blocked or closed before completing.
 */
export function open_oauth_popup(authorize_url: string): Promise<string> {
	return new Promise<string>((resolve, reject) => {
		const popup = window.open(authorize_url, "muxsocial_oauth", "width=600,height=760");
		if (!popup) {
			reject(new Error("Popup blocked. Allow popups for this site and try again."));
			return;
		}

		let settled = false;

		const cleanup = () => {
			window.removeEventListener("message", on_message);
			window.clearInterval(closed_timer);
			try {
				popup.close();
			} catch {
				// Already closed.
			}
		};

		const on_message = (event: MessageEvent) => {
			if (event.origin !== window.location.origin) return;
			const data = event.data as { type?: string; query?: string } | null;
			if (data?.type !== "muxsocial-oauth-callback") return;
			settled = true;
			cleanup();
			resolve(data.query ?? "");
		};

		// If the user closes the popup without completing, fail rather than hang.
		const closed_timer = window.setInterval(() => {
			if (popup.closed && !settled) {
				cleanup();
				reject(new Error("Authorization window was closed before finishing."));
			}
		}, 500);

		window.addEventListener("message", on_message);
	});
}
