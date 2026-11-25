import { beforeEach, describe, expect, it } from "vitest";

import { useWebSocketStore } from "../model/store";

function reset(): void {
	useWebSocketStore.getState().reset();
}

describe("useWebSocketStore", () => {
	beforeEach(reset);

	it("starts in the connecting state with no server data", () => {
		const state = useWebSocketStore.getState();

		expect(state.status).toBe("connecting");
		expect(state.protocolVersion).toBeNull();
		expect(state.connectionId).toBeNull();
		expect(state.lastHeartbeatAt).toBeNull();
		expect(state.messageCount).toBe(0);
	});

	it("records the connection metadata from the `connected` frame", () => {
		useWebSocketStore.getState().setStatus("open");
		useWebSocketStore.getState().handleServerMessage({
			type: "connected",
			connection_id: "conn-42",
			protocol_version: 1,
		});

		const state = useWebSocketStore.getState();

		expect(state.connectionId).toBe("conn-42");
		expect(state.protocolVersion).toBe(1);
		expect(state.messageCount).toBe(1);
	});

	it("keeps the newest heartbeat time and counts every message", () => {
		useWebSocketStore.getState().handleServerMessage({
			type: "heartbeat",
			server_time: "2026-01-01T00:00:00Z",
		});
		useWebSocketStore.getState().handleServerMessage({
			type: "heartbeat",
			server_time: "2026-01-01T00:00:30Z",
		});

		const state = useWebSocketStore.getState();

		expect(state.lastHeartbeatAt).toBe("2026-01-01T00:00:30Z");
		expect(state.messageCount).toBe(2);
	});

	it("counts pong and closing frames without inventing fields", () => {
		useWebSocketStore.getState().handleServerMessage({ type: "pong" });
		useWebSocketStore.getState().handleServerMessage({ type: "closing", reason: "shutdown" });

		const state = useWebSocketStore.getState();

		expect(state.messageCount).toBe(2);
		// A closing frame does not change transport status: the socket's own
		// close event owns that transition.
		expect(state.status).toBe("connecting");
	});

	it("clears every field on reset so a reconnect starts clean", () => {
		useWebSocketStore.getState().setStatus("open");
		useWebSocketStore.getState().handleServerMessage({
			type: "connected",
			connection_id: "conn-1",
			protocol_version: 1,
		});

		reset();

		const state = useWebSocketStore.getState();

		expect(state.status).toBe("connecting");
		expect(state.connectionId).toBeNull();
		expect(state.protocolVersion).toBeNull();
		expect(state.messageCount).toBe(0);
	});
});
