import { describe, expect, it } from "vitest";

import { formatLatency, formatTimestamp, formatUptime } from "./format";

describe("formatUptime", () => {
	it("renders seconds only for short uptimes", () => {
		expect(formatUptime(45)).toBe("45s");
	});

	it("renders minutes and seconds", () => {
		expect(formatUptime(728)).toBe("12m 8s");
	});

	it("renders hours and minutes", () => {
		expect(formatUptime(3_600 * 5 + 60 * 12)).toBe("5h 12m");
	});

	it("renders days and hours", () => {
		expect(formatUptime(86_400 * 3 + 3_600 * 4)).toBe("3d 4h");
	});

	it("clamps negative input to zero", () => {
		expect(formatUptime(-10)).toBe("0s");
	});
});

describe("formatLatency", () => {
	it("renders sub-second latencies in milliseconds", () => {
		expect(formatLatency(4.4)).toBe("4 ms");
	});

	it("renders second-scale latencies with one decimal", () => {
		expect(formatLatency(1_500)).toBe("1.5 s");
	});

	it("renders a dash for invalid values", () => {
		expect(formatLatency(Number.NaN)).toBe("—");
		expect(formatLatency(-1)).toBe("—");
	});
});

describe("formatTimestamp", () => {
	it("formats a valid RFC 3339 timestamp", () => {
		expect(formatTimestamp("2026-01-01T12:30:00Z")).toMatch(/2026/);
	});

	it("returns the input when the timestamp cannot be parsed", () => {
		expect(formatTimestamp("not-a-date")).toBe("not-a-date");
	});
});
