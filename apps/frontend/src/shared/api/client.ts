/**
 * Typed HTTP client for the platform API.
 *
 * The client is deliberately thin: it owns base URL resolution, request
 * cancellation, timeouts, status handling and envelope unwrapping, and nothing
 * else. Request and response shapes come from the generated OpenAPI schema, so
 * the backend contract cannot drift from the frontend types.
 *
 * Every endpoint is wrapped in the unified envelope
 * `{ success: true, data, request_id }`. The client returns the unwrapped
 * payload plus the request identifier, which the UI shows for support purposes.
 */

import { runtimeConfig } from "@/shared/config";
import { ApiError } from "./errors";
import type { components, paths } from "./generated/schema";

type HealthPayload = components["schemas"]["HealthDto"];
type ReadyPayload = components["schemas"]["ReadyDto"];
type InfoPayload = components["schemas"]["InfoDto"];

/** Typed operation identifiers exposed by the generated schema. */
export type ApiOperation = keyof paths;

/** Result of a successful call: unwrapped payload plus correlation identifier. */
export interface ApiResult<TData> {
	data: TData;
	requestId: string;
}

export interface RequestOptions {
	/** Caller owned cancellation signal, for example from React Query. */
	signal?: AbortSignal;
	/** Overrides the configured request deadline. */
	timeoutMs?: number;
	/**
	 * Statuses that carry a valid payload even though they are not 2xx.
	 *
	 * `GET /api/v1/ready` answers `503` with the full dependency report when a
	 * dependency is down, so that response must be parsed instead of raised.
	 */
	acceptedStatuses?: readonly number[];
}

const REQUEST_ID_HEADER = "x-request-id";

/** Reads the base URL at call time so tests can replace it without reloading. */
let baseUrlOverride: string | null = null;

/**
 * Overrides the API base URL. Intended for tests, where the transport is
 * replaced with a stub and the URL only needs to be stable.
 */
export function setApiBaseUrlForTesting(baseUrl: string | null): void {
	baseUrlOverride = baseUrl;
}

function resolveUrl(path: string): string {
	const base = baseUrlOverride ?? runtimeConfig.apiBaseUrl;

	// A relative base such as `/` means "same origin". `new URL` requires an
	// absolute base, so resolve against the document origin first.
	const absoluteBase = new URL(base, globalThis.location.origin);

	return new URL(path.replace(/^\//, ""), absoluteBase).toString();
}

function isErrorBody(value: unknown): value is components["schemas"]["ErrorResponse"] {
	if (typeof value !== "object" || value === null) {
		return false;
	}

	const candidate = value as { success?: unknown; error?: unknown };

	return (
		candidate.success === false &&
		typeof candidate.error === "object" &&
		candidate.error !== null
	);
}

function isSuccessBody<TData>(value: unknown): value is { data: TData; request_id: string } {
	if (typeof value !== "object" || value === null) {
		return false;
	}

	const candidate = value as { success?: unknown; data?: unknown; request_id?: unknown };

	return (
		candidate.success === true &&
		candidate.data !== undefined &&
		typeof candidate.request_id === "string"
	);
}

async function readBody(response: Response): Promise<unknown> {
	const text = await response.text();

	if (text.length === 0) {
		return null;
	}

	try {
		return JSON.parse(text) as unknown;
	} catch (cause) {
		throw new ApiError({
			kind: "parse",
			status: response.status,
			message: "The server returned a response that is not valid JSON.",
			cause,
		});
	}
}

/**
 * Performs a request and unwraps the unified envelope.
 *
 * @param path API path beginning with `/api/v1`.
 * @param options Cancellation and timeout overrides.
 * @returns The unwrapped payload and the request identifier.
 * @throws {ApiError} For network failures, timeouts, non-success statuses and
 * responses that violate the contract.
 */
export async function apiRequest<TData>(
	path: string,
	options: RequestOptions = {},
): Promise<ApiResult<TData>> {
	const timeoutMs = options.timeoutMs ?? runtimeConfig.requestTimeoutMs;

	const timeoutSignal = AbortSignal.timeout(timeoutMs);
	const signal =
		options.signal === undefined
			? timeoutSignal
			: AbortSignal.any([options.signal, timeoutSignal]);

	let response: Response;

	try {
		response = await fetch(resolveUrl(path), {
			method: "GET",
			headers: { Accept: "application/json" },
			signal,
			// Credentials are intentionally omitted: V0.1 has no authentication,
			// and cookies must not be sent to a cross-origin backend by default.
			credentials: "same-origin",
		});
	} catch (cause) {
		throw normaliseTransportError(cause, options.signal);
	}

	const body = await readBody(response);
	const requestId = response.headers.get(REQUEST_ID_HEADER) ?? undefined;

	const accepted = options.acceptedStatuses ?? [];

	if (!response.ok && !accepted.includes(response.status)) {
		throw buildHttpError(response, body, requestId);
	}

	if (!isSuccessBody<TData>(body)) {
		throw new ApiError({
			kind: "parse",
			status: response.status,
			message: "The server response does not match the documented envelope.",
			...(requestId === undefined ? {} : { requestId }),
		});
	}

	return {
		data: body.data,
		requestId: body.request_id,
	};
}

function normaliseTransportError(cause: unknown, callerSignal: AbortSignal | undefined): ApiError {
	if (callerSignal?.aborted === true) {
		return new ApiError({
			kind: "network",
			message: "The request was cancelled.",
			cause,
		});
	}

	if (
		cause instanceof DOMException &&
		(cause.name === "TimeoutError" || cause.name === "AbortError")
	) {
		return new ApiError({
			kind: "timeout",
			message: "The backend did not respond in time.",
			cause,
		});
	}

	return new ApiError({
		kind: "network",
		message: "The backend is unreachable.",
		cause,
	});
}

function buildHttpError(
	response: Response,
	body: unknown,
	requestId: string | undefined,
): ApiError {
	if (isErrorBody(body)) {
		return new ApiError({
			kind: "http",
			status: response.status,
			code: body.error.code,
			message: body.error.message,
			requestId: body.request_id,
		});
	}

	return new ApiError({
		kind: "http",
		status: response.status,
		message: `The backend responded with HTTP ${response.status}.`,
		...(requestId === undefined ? {} : { requestId }),
	});
}

/** Process liveness from `GET /api/v1/health`. */
export function fetchHealth(options?: RequestOptions): Promise<ApiResult<HealthPayload>> {
	return apiRequest<HealthPayload>("/api/v1/health", options);
}

/** Dependency readiness from `GET /api/v1/ready`. */
export function fetchReadiness(options?: RequestOptions): Promise<ApiResult<ReadyPayload>> {
	return apiRequest<ReadyPayload>("/api/v1/ready", {
		...options,
		// A `503` still carries the complete report, which is exactly what the
		// dashboard needs to show which dependency is down.
		acceptedStatuses: options?.acceptedStatuses ?? [503],
	});
}

/** Build and environment metadata from `GET /api/v1/info`. */
export function fetchInfo(options?: RequestOptions): Promise<ApiResult<InfoPayload>> {
	return apiRequest<InfoPayload>("/api/v1/info", options);
}

export type { HealthPayload, InfoPayload, ReadyPayload };
