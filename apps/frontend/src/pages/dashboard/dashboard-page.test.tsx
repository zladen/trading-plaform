import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { setApiBaseUrlForTesting } from "@/shared/api";
import { DashboardPage } from "@/pages/dashboard";

// The page mounts a real Lightweight Charts instance, which needs canvas and
// layout measurement that jsdom does not provide. The chart's own test covers
// its lifecycle; here only the page composition matters.
vi.mock("lightweight-charts", () => {
	const series = { setData: vi.fn() };
	const chart = { addSeries: vi.fn(() => series), remove: vi.fn() };

	return { createChart: vi.fn(() => chart), LineSeries: "Line" };
});

function renderWithClient(ui: ReactElement): ReturnType<typeof render> {
	const queryClient = new QueryClient({
		defaultOptions: { queries: { retry: false, gcTime: 0 } },
	});

	return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
}

function jsonResponse(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { "content-type": "application/json", "x-request-id": "req-dash" },
	});
}

const healthBody = {
	success: true,
	request_id: "req-dash",
	data: { status: "up", uptime_seconds: 61, checked_at: "2026-01-01T00:01:00Z" },
};

const infoBody = {
	success: true,
	request_id: "req-dash",
	data: {
		name: "platform-backend",
		version: "0.1.0",
		environment: "development",
		rust_version: "1.97.1",
		started_at: "2026-01-01T00:00:00Z",
		uptime_seconds: 61,
		websocket_path: "/api/v1/ws",
	},
};

function readinessBody(ready: boolean) {
	return {
		success: true,
		request_id: "req-dash",
		data: {
			status: ready ? "ready" : "not_ready",
			ready,
			checked_at: "2026-01-01T00:01:00Z",
			checks: [
				{ dependency: "postgres", status: ready ? "up" : "down", latency_ms: 3 },
				{
					dependency: "redis",
					status: ready ? "up" : "down",
					latency_ms: 2,
					...(ready ? {} : { error: "connection refused" }),
				},
			],
		},
	};
}

function mockApi(ready: boolean): ReturnType<typeof vi.fn> {
	const fetchMock = vi.fn().mockImplementation((input: string) => {
		const url = String(input);

		if (url.endsWith("/api/v1/health")) {
			return Promise.resolve(jsonResponse(healthBody));
		}

		if (url.endsWith("/api/v1/ready")) {
			return Promise.resolve(jsonResponse(readinessBody(ready), ready ? 200 : 503));
		}

		if (url.endsWith("/api/v1/info")) {
			return Promise.resolve(jsonResponse(infoBody));
		}

		return Promise.resolve(new Response("not found", { status: 404 }));
	});

	vi.stubGlobal("fetch", fetchMock);
	return fetchMock;
}

afterEach(() => {
	setApiBaseUrlForTesting(null);
	vi.unstubAllGlobals();
});

describe("DashboardPage", () => {
	it("shows a loading state before the backend answers", () => {
		vi.stubGlobal("fetch", vi.fn().mockReturnValue(new Promise(() => undefined)));

		renderWithClient(<DashboardPage />);

		expect(screen.getByText(/contacting the backend/i)).toBeInTheDocument();
	});

	it("renders live backend, PostgreSQL and Redis state", async () => {
		mockApi(true);
		renderWithClient(<DashboardPage />);

		expect(await screen.findByText(/All systems operational/i)).toBeInTheDocument();
		expect(await screen.findByText("PostgreSQL")).toBeInTheDocument();
		expect(await screen.findByText("Redis")).toBeInTheDocument();
		expect(screen.getAllByText("Operational").length).toBeGreaterThanOrEqual(3);
		expect(await screen.findByText(/Version 0\.1\.0/i)).toBeInTheDocument();
		expect(await screen.findByText(/uptime 1m 1s/i)).toBeInTheDocument();
	});

	it("mounts the chart in its explicitly empty state", async () => {
		mockApi(true);
		renderWithClient(<DashboardPage />);

		// The chart is part of the required stack, but V0.1 has no market data:
		// the page must show the placeholder instead of a fabricated series.
		expect(await screen.findByText("Price chart")).toBeInTheDocument();
		expect(screen.getAllByText("Market data is not connected yet").length).toBeGreaterThan(0);
	});

	it("surfaces a dependency failure reported with HTTP 503", async () => {
		mockApi(false);
		renderWithClient(<DashboardPage />);

		// The headline state and the backend row both report degradation.
		expect((await screen.findAllByText(/Degraded/i)).length).toBeGreaterThan(0);
		expect(await screen.findByText("connection refused")).toBeInTheDocument();
		expect(screen.getAllByText("Unavailable").length).toBeGreaterThan(0);
	});

	it("renders an error panel when the backend is unreachable", async () => {
		vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("Failed to fetch")));

		renderWithClient(<DashboardPage />);

		expect(await screen.findByText(/System status unavailable/i)).toBeInTheDocument();
		expect(screen.getByRole("alert")).toBeInTheDocument();
	});

	it("refetches when the operator asks for a new check", async () => {
		const fetchMock = mockApi(true);
		const user = userEvent.setup();

		renderWithClient(<DashboardPage />);
		await screen.findByText(/All systems operational/i);

		const callsBefore = fetchMock.mock.calls.length;
		await user.click(screen.getByRole("button", { name: /check system status again/i }));

		await waitFor(() => {
			expect(fetchMock.mock.calls.length).toBeGreaterThan(callsBefore);
		});
	});
});
