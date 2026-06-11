/**
 * The source networks and their homepage URLs, shared by the toolbar tagline
 * links and the header "Networks" menu. Order matches the tagline's `<0>…<3>`
 * placeholders (Hashiverse, Nostr, Mastodon, Bluesky).
 *
 * @module
 */

export interface NetworkLink {
	/** The network's display name (a proper noun — not translated). */
	name: string;
	/** The network's homepage, opened in a new tab. */
	url: string;
}

export const NETWORK_LINKS: NetworkLink[] = [
	{ name: "Hashiverse", url: "https://www.hashiverse.com" },
	{ name: "Nostr", url: "https://nostr.org" },
	{ name: "Mastodon", url: "https://joinmastodon.org" },
	{ name: "Bluesky", url: "https://bsky.social" },
];
