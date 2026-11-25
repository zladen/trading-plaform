import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createWebSocketClient } from "./websocket";

/**
 * Minimal WebSocket double.
 *
 * Tests need deterministic control over `open`, `message` and `close`, which a
 * real server cannot provide. The double implements only what the client uses:
 * ready-state constants, `addEventListener`, `send` and `close`.
 */
class FakeWebSocket {
	static readonly CONNECTING = 0;
	static readonly OPEN = 1;
	static readonly CLOSING = 2;
	static readonly CLOSED = 3;

	static instances: FakeWebSocket[] = [];

	readonly url: string;
	readyState = FakeWebSocket.CONNECTING;
	readonly sent: string[] = [];
	closedByClient = false;

	private readonly listeners = new Map<string, Set<(event: unknown) => void>>();

	constructor(url: string) {
		this.url = url;
		FakeWebSocket.instances.push(this);
	}

	addEventListener(type: string, listener: (event: unknown) => void): void {
		const set = this.listeners.get(type) ?? new Set();
		set.add(listener);
		this.listeners.set(type, set);
	}

	send(data: string): void {
		if (this.readyState !== FakeWebSocket.OPEN) {
			throw new Error("send() called on a socket that is not open");
		}
		this.sent.push(data);
	}

	close(): void {
		this.closedByClient = true;
		this.readyState = FakeWebSocket.CLOSED;
	}

	// --- test helpers ------------------------------------------------------

	emit(type: string, event: unknown = {}): void {
		for (const listener of this.listeners.get(type) ?? []) {
			listener(event);
		}
	}

	simulateOpen(): void {
		this.readyState = FakeWebSocket.OPEN;
		this.emit("open");
	}

	simulateServerMessage(payload: string): void {
		this.emit("message", { data: payload });
	}

	simulateClose(): void {
		this.readyState = FakeWebSocket.CLOSED;
		this.emit("close");
	}
}

const originalWebSocket = globalThis.WebSocket;

beforeEach(() => {
	FakeWebSocket.instances = [];
	vi.stubGlobal("WebSocket", FakeWebSocket);
});

afterEach(() => {
	vi.unstubAllGlobals();
	vi.useRealTimers();
	if (originalWebSocket !== undefined) {
		vi.stubGlobal("WebSocket", originalWebSocket);
	}
});

describe("createWebSocketClient", () => {
	it("connects to the configured path and reports state transitions", () => {
		const statuses: string[] = [];
		const client = createWebSocketClient({
			onStatusChange: (status) => statuses.push(status),
			maxReconnectAttempts: 0,
		});

		expect(FakeWebSocket.instances).toHaveLength(1);
		expect(FakeWebSocket.instances[0]?.url).toMatch(/\/api\/v1\/ws$/);

		FakeWebSocket.instances[0]?.simulateOpen();

		// The initial `connecting` state is the documented starting value, not a
		// transition, so only the change to `open` is reported.
		expect(statuses).toEqual(["open"]);
		expect(client.status).toBe("open");

		client.close();
		expect(statuses.at(-1)).toBe("closed");
	});

	it("sends an application ping on open and on the interval", () => {
		vi.useFakeTimers();

		const client = createWebSocketClient({
			pingIntervalMs: 1_000,
			maxReconnectAttempts: 0,
		});
		const socket = FakeWebSocket.instances[0];

		socket?.simulateOpen();
		expect(socket?.sent).toEqual([JSON.stringify({ type: "ping" })]);

		vi.advanceTimersByTime(1_000);
		expect(socket?.sent).toHaveLength(2);

		client.close();
		vi.advanceTimersByTime(5_000);
		// No further pings after close: the interval is cleared.
		expect(socket?.sent).toHaveLength(2);
	});

	it("forwards only well-formed server messages", () => {
		const messages: unknown[] = [];
		const client = createWebSocketClient({
			onMessage: (message) => messages.push(message),
			maxReconnectAttempts: 0,
		});
		const socket = FakeWebSocket.instances[0];

		socket?.simulateOpen();
		socket?.simulateServerMessage(
			JSON.stringify({ type: "connected", connection_id: "c1", protocol_version: 1 }),
		);
		socket?.simulateServerMessage("not json");
		socket?.simulateServerMessage(JSON.stringify({ noType: true }));

		expect(messages).toEqual([{ type: "connected", connection_id: "c1", protocol_version: 1 }]);

		client.close();
	});

	it("ignores binary frames instead of throwing", () => {
		const messages: unknown[] = [];
		const client = createWebSocketClient({
			onMessage: (message) => messages.push(message),
			maxReconnectAttempts: 0,
		});

		FakeWebSocket.instances[0]?.emit("message", { data: new ArrayBuffer(8) });

		expect(messages).toHaveLength(0);
		client.close();
	});

	it("reconnects with a growing delay and stops after the attempt budget", () => {
		vi.useFakeTimers();

		const client = createWebSocketClient({
			reconnectDelayMs: 1_000,
			maxReconnectAttempts: 2,
		});

		FakeWebSocket.instances[0]?.simulateClose();
		expect(FakeWebSocket.instances).toHaveLength(1);

		// First retry after 1 × delay.
		vi.advanceTimersByTime(1_000);
		expect(FakeWebSocket.instances).toHaveLength(2);

		FakeWebSocket.instances[1]?.simulateClose();

		// Second retry after 2 × delay.
		vi.advanceTimersByTime(1_000);
		expect(FakeWebSocket.instances).toHaveLength(2);
		vi.advanceTimersByTime(1_000);
		expect(FakeWebSocket.instances).toHaveLength(3);

		FakeWebSocket.instances[2]?.simulateClose();
		vi.advanceTimersByTime(10_000);

		// The budget is exhausted: no fourth socket, and the state is closed.
		expect(FakeWebSocket.instances).toHaveLength(3);
		expect(client.status).toBe("closed");
	});

	it("resets the retry budget after a successful reconnect", () => {
		vi.useFakeTimers();

		const client = createWebSocketClient({
			reconnectDelayMs: 1_000,
			maxReconnectAttempts: 2,
		});

		FakeWebSocket.instances[0]?.simulateClose();
		vi.advanceTimersByTime(1_000);
		FakeWebSocket.instances[1]?.simulateOpen();
		FakeWebSocket.instances[1]?.simulateClose();

		// After a successful open the first retry is immediate again.
		vi.advanceTimersByTime(1_000);
		expect(FakeWebSocket.instances).toHaveLength(3);

		client.close();
	});

	it("does not reconnect after an explicit close", () => {
		vi.useFakeTimers();

		const client = createWebSocketClient({ reconnectDelayMs: 1_000 });
		const socket = FakeWebSocket.instances[0];

		client.close();

		expect(socket?.closedByClient).toBe(true);

		vi.advanceTimersByTime(10_000);
		expect(FakeWebSocket.instances).toHaveLength(1);
	});

	it("refuses to send a ping while the socket is not open", () => {
		const client = createWebSocketClient({ maxReconnectAttempts: 0 });
		const socket = FakeWebSocket.instances[0];

		// Still CONNECTING: the client must not call `send`.
		client.ping("payload");
		expect(socket?.sent).toEqual([]);

		socket?.simulateOpen();
		const sentAfterOpen = socket?.sent.length ?? 0;
		client.ping("payload");
		expect(socket?.sent.length).toBe(sentAfterOpen + 1);
		expect(socket?.sent.at(-1)).toBe(JSON.stringify({ type: "ping", payload: "payload" }));

		client.close();
	});
});
