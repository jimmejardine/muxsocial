import { describe, expect, it } from "vitest";
import { sanitize_post_html } from "./PostBody.tsx";

describe("sanitize_post_html", () => {
	it("strips <script> tags", () => {
		const sanitized = sanitize_post_html("<p>hi</p><script>alert(1)</script>");
		expect(sanitized).toContain("<p>hi</p>");
		expect(sanitized).not.toContain("<script>");
		expect(sanitized).not.toContain("alert(1)");
	});

	it("strips inline event handlers like onerror", () => {
		const sanitized = sanitize_post_html('<img src="x" onerror="alert(1)">');
		expect(sanitized).not.toContain("onerror");
		expect(sanitized).not.toContain("alert(1)");
	});

	it("keeps benign markup", () => {
		const sanitized = sanitize_post_html('<p>hello <a href="https://example.com">link</a></p>');
		expect(sanitized).toContain("<a");
		expect(sanitized).toContain("https://example.com");
	});
});
