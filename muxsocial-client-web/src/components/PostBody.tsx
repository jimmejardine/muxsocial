import DOMPurify from "dompurify";

/**
 * Sanitize a post's `content_html` for safe rendering. Post bodies are HTML from
 * untrusted networks (Mastodon/Hashiverse native HTML, nostr/Bluesky rendered to
 * HTML), so they must be sanitized before they touch the DOM. Exported so the
 * sanitization is unit-testable without rendering.
 */
export function sanitize_post_html(content_html: string): string {
	return DOMPurify.sanitize(content_html);
}

interface PostBodyProps {
	content_html: string;
}

/** Render a post body's HTML after sanitizing it with DOMPurify. */
export function PostBody({ content_html }: PostBodyProps) {
	const sanitized_html = sanitize_post_html(content_html);
	return <div dangerouslySetInnerHTML={{ __html: sanitized_html }} />;
}
