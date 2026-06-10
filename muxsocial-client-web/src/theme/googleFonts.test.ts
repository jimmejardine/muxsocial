import { beforeEach, describe, expect, it } from "vitest";
import { buildGoogleFontsUrl, ensureGoogleFonts, resetLoadedFontsForTests } from "./googleFonts.ts";

describe("buildGoogleFontsUrl", () => {
	it("encodes spaces, sorts weights, and appends display=swap", () => {
		const url = buildGoogleFontsUrl([{ family: "Chakra Petch", weights: [700, 400] }]);
		expect(url).toBe("https://fonts.googleapis.com/css2?family=Chakra+Petch:wght@400;700&display=swap");
	});

	it("joins multiple families with &", () => {
		const url = buildGoogleFontsUrl([
			{ family: "Inter", weights: [400] },
			{ family: "Chakra Petch", weights: [500] },
		]);
		expect(url).toBe("https://fonts.googleapis.com/css2?family=Inter:wght@400&family=Chakra+Petch:wght@500&display=swap");
	});
});

describe("ensureGoogleFonts", () => {
	beforeEach(() => {
		document.head.innerHTML = "";
		resetLoadedFontsForTests();
	});

	it("injects a stylesheet link and preconnect hints", () => {
		ensureGoogleFonts([{ family: "Inter", weights: [400] }]);
		const stylesheet = document.head.querySelector("link[rel=stylesheet][data-mux-font]");
		expect(stylesheet?.getAttribute("href")).toContain("family=Inter");
		expect(document.head.querySelectorAll("link[data-mux-font-preconnect]").length).toBe(2);
	});

	it("is idempotent for an already-loaded family", () => {
		ensureGoogleFonts([{ family: "Inter", weights: [400] }]);
		ensureGoogleFonts([{ family: "Inter", weights: [400] }]);
		expect(document.head.querySelectorAll("link[data-mux-font]").length).toBe(1);
	});

	it("only requests not-yet-loaded families on a subsequent call", () => {
		ensureGoogleFonts([{ family: "Inter", weights: [400] }]);
		ensureGoogleFonts([
			{ family: "Inter", weights: [400] },
			{ family: "Chakra Petch", weights: [400] },
		]);
		const links = document.head.querySelectorAll("link[data-mux-font]");
		expect(links.length).toBe(2);
		expect(links[1].getAttribute("href")).toContain("family=Chakra+Petch");
		expect(links[1].getAttribute("href")).not.toContain("family=Inter");
	});
});
