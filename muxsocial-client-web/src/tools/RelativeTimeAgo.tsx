/**
 * A live post timestamp: the relative form ("5 minutes ago") while the post is
 * under 24h old, otherwise the absolute local date-time. Localized via
 * `Intl.RelativeTimeFormat` / `toLocaleString` (no translation keys), and
 * self-updating on an adaptive interval so the relative form stays current
 * without re-rendering the whole timeline.
 *
 * @module
 */

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

/** Units largest-first, with their length in seconds. */
const RELATIVE_TIME_UNITS: Array<[Intl.RelativeTimeFormatUnit, number]> = [
	["year", 365 * 24 * 60 * 60],
	["month", 30 * 24 * 60 * 60],
	["week", 7 * 24 * 60 * 60],
	["day", 24 * 60 * 60],
	["hour", 60 * 60],
	["minute", 60],
	["second", 1],
];

/**
 * Format `date_millis` relative to `now_millis` as a localized string like
 * "5 minutes ago" or "in 3 hours". Picks the largest unit that fits; anything
 * under a minute reads as "now" (via `numeric: "auto"`). Pure (takes `now`), so
 * it is unit-testable.
 */
export function format_relative_time(date_millis: number, now_millis: number, locale?: string): string {
	const delta_seconds = Math.round((date_millis - now_millis) / 1000);
	const absolute_delta_seconds = Math.abs(delta_seconds);
	const formatter = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
	for (const [unit, threshold_seconds] of RELATIVE_TIME_UNITS) {
		if (absolute_delta_seconds >= threshold_seconds) {
			return formatter.format(Math.round(delta_seconds / threshold_seconds), unit);
		}
	}
	return formatter.format(0, "second");
}

/** Under this age a post reads as a relative time; at or beyond it, an absolute date. */
const DAY_MILLIS = 24 * 60 * 60 * 1000;

/**
 * The post's displayed time: the relative form ("5 minutes ago") when under 24h
 * old, otherwise the absolute local date-time (matching `Date.toLocaleString`).
 * Pure (takes `now`), so it is unit-testable.
 */
export function format_timestamp(date_millis: number, now_millis: number, locale?: string): string {
	if (Math.abs(now_millis - date_millis) < DAY_MILLIS) {
		return format_relative_time(date_millis, now_millis, locale);
	}
	return new Date(date_millis).toLocaleString(locale);
}

/** How often to refresh, scaled to age: recent posts tick faster than old ones. */
export function refresh_interval_millis(age_millis: number): number {
	const age_seconds = Math.abs(age_millis) / 1000;
	if (age_seconds < 60) return 10_000;
	if (age_seconds < 3600) return 30_000;
	if (age_seconds < 86400) return 300_000;
	return 1_800_000;
}

/** A self-updating localized timestamp for `date_millis` (Unix epoch ms): relative
 *  under 24h, absolute date-time beyond. */
export function RelativeTimeAgo({ date_millis }: { date_millis: number }) {
	const { i18n } = useTranslation();
	const [text, set_text] = useState(() => format_timestamp(date_millis, Date.now(), i18n.language));

	useEffect(() => {
		set_text(format_timestamp(date_millis, Date.now(), i18n.language));
		const interval_id = setInterval(
			() => {
				set_text(format_timestamp(date_millis, Date.now(), i18n.language));
			},
			refresh_interval_millis(Date.now() - date_millis),
		);
		return () => clearInterval(interval_id);
	}, [date_millis, i18n.language]);

	// Always expose the exact local date-time on hover, even when showing the relative form.
	return (
		<time dateTime={new Date(date_millis).toISOString()} title={new Date(date_millis).toLocaleString(i18n.language)}>
			{text}
		</time>
	);
}
