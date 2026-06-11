import { describe, expect, it } from "vitest";
import { format_relative_time, format_timestamp, refresh_interval_millis } from "./RelativeTimeAgo.tsx";

const NOW = 1_700_000_000_000;
const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

describe("format_relative_time (en)", () => {
	it("uses seconds under a minute, and 'now' under a second", () => {
		expect(format_relative_time(NOW - 30_000, NOW, "en")).toBe("30 seconds ago");
		expect(format_relative_time(NOW - 200, NOW, "en")).toBe("now");
	});

	it("picks minutes for a few minutes ago", () => {
		expect(format_relative_time(NOW - 5 * MINUTE, NOW, "en")).toBe("5 minutes ago");
	});

	it("picks hours, then days, as the delta grows", () => {
		expect(format_relative_time(NOW - 3 * HOUR, NOW, "en")).toBe("3 hours ago");
		expect(format_relative_time(NOW - 2 * DAY, NOW, "en")).toBe("2 days ago");
	});

	it("handles future times", () => {
		expect(format_relative_time(NOW + 10 * MINUTE, NOW, "en")).toBe("in 10 minutes");
	});
});

describe("format_timestamp (en)", () => {
	it("uses the relative form under 24h", () => {
		expect(format_timestamp(NOW - 5 * MINUTE, NOW, "en")).toBe("5 minutes ago");
		expect(format_timestamp(NOW - 3 * HOUR, NOW, "en")).toBe("3 hours ago");
	});

	it("uses the absolute local date-time at or beyond 24h", () => {
		expect(format_timestamp(NOW - 2 * DAY, NOW, "en")).toBe(new Date(NOW - 2 * DAY).toLocaleString("en"));
	});
});

describe("refresh_interval_millis", () => {
	it("ticks faster for recent posts and slower for old ones", () => {
		expect(refresh_interval_millis(30_000)).toBe(10_000);
		expect(refresh_interval_millis(30 * MINUTE)).toBe(30_000);
		expect(refresh_interval_millis(5 * HOUR)).toBe(300_000);
		expect(refresh_interval_millis(3 * DAY)).toBe(1_800_000);
	});
});
