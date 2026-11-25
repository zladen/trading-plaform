import { describe, expect, it } from "vitest";

import { resolveWebSocketUrl } from "./websocket";

describe("resolveWebSocketUrl", () => {
	it("derives a ws URL on the current origin for a relative base", () => {
		expect(resolveWebSocketUrl("/api/v1/ws", "/")).toBe("ws://localhost:3000/api/v1/ws");
	});

	it("upgrades to wss when the base URL uses https", () => {
		expect(resolveWebSocketUrl("/api/v1/ws", "https://api.example.test/")).toBe(
			"wss://api.example.test/api/v1/ws",
		);
	});

	it("keeps the host of an absolute http base URL", () => {
		expect(resolveWebSocketUrl("/api/v1/ws", "http://127.0.0.1:8080/")).toBe(
			"ws://127.0.0.1:8080/api/v1/ws",
		);
	});
});
