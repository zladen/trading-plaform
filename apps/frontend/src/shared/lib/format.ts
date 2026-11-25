/**
 * Formats a duration in seconds as a compact human readable string.
 *
 * @param seconds Non-negative number of seconds.
 * @returns For example `3d 4h`, `12m 8s` or `45s`.
 */
export function formatUptime(seconds: number): string {
	const safe = Math.max(0, Math.floor(seconds));

	const days = Math.floor(safe / 86_400);
	const hours = Math.floor((safe % 86_400) / 3_600);
	const minutes = Math.floor((safe % 3_600) / 60);
	const remainder = safe % 60;

	if (days > 0) {
		return `${days}d ${hours}h`;
	}

	if (hours > 0) {
		return `${hours}h ${minutes}m`;
	}

	if (minutes > 0) {
		return `${minutes}m ${remainder}s`;
	}

	return `${remainder}s`;
}

/**
 * Formats an RFC 3339 timestamp for display in the user's local timezone.
 *
 * @param value Timestamp produced by the backend, in UTC.
 * @param locale Locale used for formatting.
 * @returns A short date and time, or the input when it cannot be parsed.
 */
export function formatTimestamp(value: string, locale = "en-GB"): string {
	const parsed = new Date(value);

	if (Number.isNaN(parsed.getTime())) {
		return value;
	}

	return new Intl.DateTimeFormat(locale, {
		dateStyle: "medium",
		timeStyle: "medium",
	}).format(parsed);
}

/**
 * Formats a latency in milliseconds for status rows.
 *
 * @param milliseconds Measured duration.
 * @returns For example `12 ms` or `1.4 s`.
 */
export function formatLatency(milliseconds: number): string {
	if (!Number.isFinite(milliseconds) || milliseconds < 0) {
		return "—";
	}

	if (milliseconds < 1_000) {
		return `${Math.round(milliseconds)} ms`;
	}

	return `${(milliseconds / 1_000).toFixed(1)} s`;
}
