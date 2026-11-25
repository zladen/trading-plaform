import { describe, expect, it } from "vitest";

import {
	buildSnapshot,
	deriveSystemState,
	mapHealth,
	mapInfo,
	mapReadiness,
	type SystemSnapshot,
} from "./model";

const health = {
	status: "up" as const,
	uptime_seconds: 120,
	checked_at: "2026-01-01T00:02:00Z",
};

const readiness = {
	status: "ready" as const,
	ready: true,
	checked_at: "2026-01-01T00:02:00Z",
	checks: [
		{ dependency: "postgres", status: "up" as const, latency_ms: 4 },
		{ dependency: "redis", status: "up" as const, latency_ms: 2 },
	],
};

const info = {
	name: "platform-backend",
	version: "0.1.0",
	environment: "development",
	rust_version: "1.97.1",
	started_at: "2026-01-01T00:00:00Z",
	uptime_seconds: 120,
	websocket_path: "/api/v1/ws",
};

function snapshotWith(overrides: Partial<SystemSnapshot> = {}): SystemSnapshot {
	return {
		...buildSnapshot({ health, readiness, info, requestId: "req-1" }),
		...overrides,
	};
}

describe("mapReadiness", () => {
	it("maps known dependencies to display components", () => {
		const result = mapReadiness(readiness);

		expect(result).toEqual([
			{ id: "postgres", label: "PostgreSQL", state: "up", detail: "4 ms" },
			{ id: "redis", label: "Redis", state: "up", detail: "2 ms" },
		]);
	});

	it("keeps an unknown dependency visible instead of dropping it", () => {
		const result = mapReadiness({
			...readiness,
			ready: false,
			status: "not_ready",
			checks: [{ dependency: "kafka", status: "down", latency_ms: 200, error: "timed out" }],
		});

		expect(result).toEqual([
			{ id: "backend", label: "kafka", state: "down", detail: "timed out" },
		]);
	});

	it("falls back to a generic detail when the probe reports no error", () => {
		const result = mapReadiness({
			...readiness,
			ready: false,
			status: "not_ready",
			checks: [{ dependency: "redis", status: "down", latency_ms: 200 }],
		});

		expect(result[0]?.detail).toBe("probe reported the dependency as unavailable");
	});
});

describe("mapHealth", () => {
	it("reports the process as up", () => {
		expect(mapHealth(health).state).toBe("up");
	});

	it("surfaces a non-up status verbatim", () => {
		const result = mapHealth({ ...health, status: "draining" });

		expect(result.state).toBe("down");
		expect(result.detail).toBe("reported status: draining");
	});
});

describe("mapInfo", () => {
	it("normalises a missing rust version to null", () => {
		const result = mapInfo({ ...info, rust_version: null });

		expect(result.rustVersion).toBeNull();
		expect(result.version).toBe("0.1.0");
	});
});

describe("buildSnapshot", () => {
	it("is operational when every component is up", () => {
		const snapshot = snapshotWith();

		expect(snapshot.backend.state).toBe("up");
		expect(deriveSystemState(snapshot)).toBe("operational");
	});

	it("degrades the backend when a dependency is down", () => {
		const snapshot = buildSnapshot({
			health,
			readiness: {
				...readiness,
				ready: false,
				status: "not_ready",
				checks: [
					{ dependency: "postgres", status: "up", latency_ms: 4 },
					{
						dependency: "redis",
						status: "down",
						latency_ms: 200,
						error: "connection refused",
					},
				],
			},
			info,
			requestId: "req-2",
		});

		expect(snapshot.backend.state).toBe("degraded");
		expect(snapshot.backend.detail).toBe("serving, but a dependency is unavailable");
		expect(deriveSystemState(snapshot)).toBe("degraded");
	});

	it("is unavailable when every component is down", () => {
		const snapshot = buildSnapshot({
			health: { ...health, status: "down" },
			readiness: {
				...readiness,
				ready: false,
				status: "not_ready",
				checks: [
					{ dependency: "postgres", status: "down", latency_ms: 10 },
					{ dependency: "redis", status: "down", latency_ms: 10 },
				],
			},
			info,
			requestId: "req-3",
		});

		expect(deriveSystemState(snapshot)).toBe("unavailable");
	});
});
