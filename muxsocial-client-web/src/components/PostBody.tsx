import DOMPurify from "dompurify";

// Harden links (nostr linkifies URLs) and lazy-load inline images (Hashiverse
// bodies can carry <img>/<video>). Registered once at module load.
DOMPurify.addHook("afterSanitizeAttributes", (node) => {
	if (node.tagName === "A") {
		node.setAttribute("target", "_blank");
		node.setAttribute("rel", "noopener noreferrer");
	}
	if (node.tagName === "IMG") {
		node.setAttribute("loading", "lazy");
	}
});

// Keep DOMPurify's broad default allowlist (Hashiverse bodies are rich HTML) and
// additionally permit inline media + the attributes the hook sets.
const SANITIZE_CONFIG = {
	ADD_TAGS: ["video", "source"],
	ADD_ATTR: ["target", "rel", "loading", "controls", "poster"],
};

/**
 * Sanitize a post's `content_html` for safe rendering. Post bodies are HTML from
 * untrusted networks (Mastodon/Hashiverse native HTML, nostr/Bluesky rendered to
 * HTML), so they must be sanitized before they touch the DOM. Exported so the
 * sanitization is unit-testable without rendering.
 */
export function sanitize_post_html(content_html: string): string {
	return DOMPurify.sanitize(content_html, SANITIZE_CONFIG);
}

interface PostBodyProps {
	content_html: string;
}

/** Render a post body's HTML after sanitizing it with DOMPurify. */
export function PostBody({ content_html }: PostBodyProps) {
	const sanitized_html = sanitize_post_html(content_html);
	return <div dangerouslySetInnerHTML={{ __html: sanitized_html }} />;
}
