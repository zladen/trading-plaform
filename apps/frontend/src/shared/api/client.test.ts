import { describe, expect, it, vi } from "vitest";

import { ApiError } from "./errors";
import { apiRequest, fetchReadiness, setApiBaseUrlForTesting } from "./client";

const BASE_URL = "http://backend.test/";

function jsonResponse(body: unknown, init: ResponseInit = {}): Response {
	return new Response(JSON.stringify(body), {
		status: init.status ?? 200,
		headers: { "content-type": "application/json", "x-request-id": "req-1", ...init.headers },
	});
}

describe("apiRequest", () => {
	it("unwraps the unified success envelope", async () => {
		setApiBaseUrlForTesting(BASE_URL);
		const fetchMock = vi.fn().mockResolvedValue(
			jsonResponse({
				success: true,
				data: { status: "up", uptime_seconds: 12, checked_at: "2026-01-01T00:00:00Z" },
				request_id: "req-1",
			}),
		);
		vi.stubGlobal("fetch", fetchMock);

		const result = await apiRequest<{ status: string }>("/api/v1/health");

		expect(result.data.status).toBe("up");
		expect(result.requestId).toBe("req-1");
		expect(fetchMock).toHaveBeenCalledOnce();

		const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
		expect(url).toBe("http://backend.test/api/v1/health");
		expect(init.method).toBe("GET");
		expect(init.credentials).toBe("same-origin");

		setApiBaseUrlForTesting(null);
	});

	it("raises a typed error for a documented error envelope", async () => {
		vi.stubGlobal(
			"fetch",
			vi.fn().mockResolvedValue(
				jsonResponse(
					{
						success: false,
						error: { code: "not_found", message: "No such route", details: [] },
						request_id: "req-err",
					},
					{ status: 404 },
				),
			),
		);

		await expect(apiRequest("/api/v1/missing")).rejects.toMatchObject({
			name: "ApiError",
			kind: "http",
			status: 404,
			code: "not_found",
			message: "No such route",
			requestId: "req-err",
		});
	});

	it("marks server failures as retryable and client failures as not", async () => {
		vi.stubGlobal(
			"fetch",
			vi.fn().mockResolvedValue(
				jsonResponse(
					{
						success: false,
						error: { code: "internal_error", message: "boom" },
						request_id: "r",
					},
					{ status: 500 },
				),
			),
		);

		const error = await apiRequest("/api/v1/health").catch((caught: unknown) => caught);

		expect(error).toBeInstanceOf(ApiError);
		expect((error as ApiError).isRetryable).toBe(true);
	});

	it("reports a network failure when fetch rejects", async () => {
		vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("Failed to fetch")));

		await expect(apiRequest("/api/v1/health")).rejects.toMatchObject({
			kind: "network",
			message: "The backend is unreachable.",
		});
	});

	it("reports a timeout when the deadline signal aborts", async () => {
		vi.stubGlobal(
			"fetch",
			vi.fn().mockRejectedValue(new DOMException("The operation timed out", "TimeoutError")),
		);

		await expect(apiRequest("/api/v1/health")).rejects.toMatchObject({
			kind: "timeout",
			isRetryable: true,
		});
	});

	it("propagates caller cancellation as a cancelled request", async () => {
		const controller = new AbortController();
		controller.abort();

		vi.stubGlobal(
			"fetch",
			vi.fn().mockImplementation(() => {
				throw new DOMException("The operation was aborted", "AbortError");
			}),
		);

		await expect(
			apiRequest("/api/v1/health", { signal: controller.signal }),
		).rejects.toMatchObject({
			kind: "network",
			message: "The request was cancelled.",
		});
	});

	it("rejects a response that violates the documented envelope", async () => {
		vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({ unexpected: true })));

		await expect(apiRequest("/api/v1/health")).rejects.toMatchObject({
			kind: "parse",
			message: "The server response does not match the documented envelope.",
		});
	});

	it("rejects a body that is not valid JSON", async () => {
		vi.stubGlobal(
			"fetch",
			vi.fn().mockResolvedValue(new Response("<html>oops</html>", { status: 200 })),
		);

		await expect(apiRequest("/api/v1/health")).rejects.toMatchObject({ kind: "parse" });
	});
});

describe("fetchReadiness", () => {
	it("treats 503 as a valid report instead of a transport failure", async () => {
		vi.stubGlobal(
			"fetch",
			vi.fn().mockResolvedValue(
				jsonResponse(
					{
						success: true,
						data: {
							status: "not_ready",
							ready: false,
							checked_at: "2026-01-01T00:00:00Z",
							checks: [
								{ dependency: "postgres", status: "up", latency_ms: 3 },
								{
									dependency: "redis",
									status: "down",
									latency_ms: 200,
									error: "connection refused",
								},
							],
						},
						request_id: "req-503",
					},
					{ status: 503 },
				),
			),
		);

		const result = await fetchReadiness();

		expect(result.data.ready).toBe(false);
		expect(result.data.checks).toHaveLength(2);
		expect(result.data.checks[1]).toMatchObject({ dependency: "redis", status: "down" });
	});
});
