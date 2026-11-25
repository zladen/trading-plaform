/**
 * Runtime configuration resolved once at module load.
 *
 * Vite exposes only variables prefixed with `VITE_`. The API base URL defaults
 * to the same origin so the browser uses the dev proxy or the nginx reverse
 * proxy in containers instead of a hardcoded host.
 */

const DEFAULT_API_BASE_URL = "/";

function readApiBaseUrl(): string {
	const configured: unknown = import.meta.env.VITE_API_URL;

	if (typeof configured !== "string" || configured.trim().length === 0) {
		return DEFAULT_API_BASE_URL;
	}

	const trimmed = configured.trim();

	return trimmed.endsWith("/") ? trimmed : `${trimmed}/`;
}

export const runtimeConfig = {
	apiBaseUrl: readApiBaseUrl(),
	/**
	 * Interval used by the WebSocket client for application level pings.
	 * Kept below the backend heartbeat so a silent connection is detected.
	 */
	webSocketPingIntervalMs: 15_000,
	/**
	 * Delay before the WebSocket client retries a dropped connection.
	 */
	webSocketReconnectDelayMs: 3_000,
	/**
	 * Timeout applied to every REST request, enforced by AbortController.
	 */
	requestTimeoutMs: 10_000,
} as const;
