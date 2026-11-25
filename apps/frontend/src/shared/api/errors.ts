/**
 * Transport level error raised by the API client.
 *
 * The client normalises three distinct failure modes into one type so that UI
 * code does not need to police `fetch` internals:
 *
 * - `network`  — the request never reached the server (DNS, connection refused).
 * - `timeout`  — the request exceeded the configured deadline.
 * - `http`     — the server answered with a non-success status.
 * - `parse`    — the response was not the JSON contract the client expects.
 */
export type ApiErrorKind = "network" | "timeout" | "http" | "parse";

export interface ApiErrorOptions {
	kind: ApiErrorKind;
	message: string;
	status?: number;
	code?: string;
	requestId?: string;
	cause?: unknown;
}

/**
 * Error surfaced to the UI layer.
 *
 * `code` mirrors the backend `error.code` value when the server answered, so
 * callers can branch on a stable identifier instead of a message string.
 */
export class ApiError extends Error {
	readonly kind: ApiErrorKind;
	readonly status: number | undefined;
	readonly code: string | undefined;
	readonly requestId: string | undefined;

	constructor(options: ApiErrorOptions) {
		super(options.message, options.cause === undefined ? undefined : { cause: options.cause });
		this.name = "ApiError";
		this.kind = options.kind;
		this.status = options.status;
		this.code = options.code;
		this.requestId = options.requestId;
	}

	/** `true` when retrying the same request may succeed. */
	get isRetryable(): boolean {
		if (this.kind === "network" || this.kind === "timeout") {
			return true;
		}

		return this.status !== undefined && this.status >= 500;
	}
}
