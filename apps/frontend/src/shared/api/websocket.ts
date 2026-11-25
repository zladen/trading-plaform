/**
 * WebSocket transport for live updates.
 *
 * The backend owns the protocol: it sends a `connected` frame on upgrade, a
 * `heartbeat` frame on a fixed interval, and `closing` before it ends the
 * connection. The client mirrors that protocol and reports connection state
 * changes as plain values instead of throwing, because a dropped live channel
 * must not crash the UI.
 *
 * Reconnection is bounded and backoff based. The socket is never created in a
 * test environment unless a caller explicitly asks for it.
 */

import { runtimeConfig } from "@/shared/config";
import type { components } from "./generated/schema";

/** Messages the server may send. */
export type ServerMessage = components["schemas"]["ServerMessage"];

export type ConnectionStatus = "connecting" | "open" | "closed";

export interface WebSocketClientOptions {
	/** Absolute or relative WebSocket path. Defaults to `/api/v1/ws`. */
	path?: string;
	/** Application level ping interval. */
	pingIntervalMs?: number;
	/** Delay before the first reconnection attempt. */
	reconnectDelayMs?: number;
	/** Maximum number of reconnection attempts; `0` disables reconnection. */
	maxReconnectAttempts?: number;
	/** Called on every state change. */
	onStatusChange?: (status: ConnectionStatus) => void;
	/** Called for every parsed server message. */
	onMessage?: (message: ServerMessage) => void;
}

/** Handles returned to the caller for explicit teardown. */
export interface WebSocketHandle {
	/** Current connection state. */
	readonly status: ConnectionStatus;
	/** Closes the connection and stops reconnecting. */
	close: () => void;
	/** Sends an application level ping. */
	ping: (payload?: string) => void;
}

/**
 * Resolves the WebSocket URL from the API base URL.
 *
 * A relative base such as `/` becomes a URL on the current origin, which works
 * behind the Vite dev proxy and the nginx reverse proxy alike.
 */
export function resolveWebSocketUrl(path = "/api/v1/ws", baseUrl?: string): string {
	const base = baseUrl ?? runtimeConfig.apiBaseUrl;
	const url = new URL(path.replace(/^\//, ""), new URL(base, globalThis.location.origin));

	url.protocol = url.protocol === "https:" ? "wss:" : "ws:";

	return url.toString();
}

import { z } from "zod";

/**
 * Runtime schema for server frames.
 *
 * The generated types describe the shape the backend documents, but a frame
 * arrives over the network and is untrusted input: a proxy, a stale server or a
 * truncated message can all produce valid JSON that is not a server message.
 * Parsing with Zod means the store only ever receives a value that actually
 * matches one of the documented variants.
 */
const serverMessageSchema = z.discriminatedUnion("type", [
	z.object({
		type: z.literal("connected"),
		connection_id: z.string().min(1),
		protocol_version: z.number().int(),
	}),
	z.object({
		type: z.literal("heartbeat"),
		server_time: z.string().min(1),
	}),
	z.object({
		type: z.literal("closing"),
		reason: z.string(),
	}),
	z.object({
		type: z.literal("pong"),
		payload: z.string().nullish(),
	}),
]);

function parseServerMessage(raw: string): ServerMessage | null {
	let decoded: unknown;

	try {
		decoded = JSON.parse(raw) as unknown;
	} catch {
		return null;
	}

	const parsed = serverMessageSchema.safeParse(decoded);

	return parsed.success ? parsed.data : null;
}

/**
 * Opens a WebSocket connection with heartbeats and bounded reconnection.
 *
 * @param options Transport behaviour callbacks and overrides.
 * @returns A handle that must be closed when the owner unmounts.
 */
export function createWebSocketClient(options: WebSocketClientOptions = {}): WebSocketHandle {
	const pingIntervalMs = options.pingIntervalMs ?? runtimeConfig.webSocketPingIntervalMs;
	const reconnectDelayMs = options.reconnectDelayMs ?? runtimeConfig.webSocketReconnectDelayMs;
	const maxReconnectAttempts = options.maxReconnectAttempts ?? 3;

	let status: ConnectionStatus = "connecting";
	let socket: WebSocket | null = null;
	let pingTimer: ReturnType<typeof setInterval> | null = null;
	let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	let attempts = 0;
	let disposed = false;

	const setStatus = (next: ConnectionStatus): void => {
		if (status === next) {
			return;
		}

		status = next;
		options.onStatusChange?.(next);
	};

	const clearTimers = (): void => {
		if (pingTimer !== null) {
			clearInterval(pingTimer);
			pingTimer = null;
		}

		if (reconnectTimer !== null) {
			clearTimeout(reconnectTimer);
			reconnectTimer = null;
		}
	};

	const scheduleReconnect = (): void => {
		if (disposed || maxReconnectAttempts === 0 || attempts >= maxReconnectAttempts) {
			setStatus("closed");
			return;
		}

		attempts += 1;
		const delay = reconnectDelayMs * attempts;

		reconnectTimer = setTimeout(() => {
			reconnectTimer = null;
			connect();
		}, delay);
	};

	const connect = (): void => {
		if (disposed) {
			return;
		}

		setStatus("connecting");

		const target = resolveWebSocketUrl(options.path);
		const connection = new WebSocket(target);
		socket = connection;

		connection.addEventListener("open", () => {
			attempts = 0;
			setStatus("open");
			sendPing(connection);

			pingTimer = setInterval(() => {
				if (connection.readyState === WebSocket.OPEN) {
					sendPing(connection);
				}
			}, pingIntervalMs);
		});

		connection.addEventListener("message", (event) => {
			if (typeof event.data !== "string") {
				return;
			}

			const message = parseServerMessage(event.data);

			if (message !== null) {
				options.onMessage?.(message);
			}
		});

		connection.addEventListener("close", () => {
			if (pingTimer !== null) {
				clearInterval(pingTimer);
				pingTimer = null;
			}

			socket = null;

			if (disposed) {
				return;
			}

			scheduleReconnect();
		});

		connection.addEventListener("error", () => {
			// A transport error is always followed by `close`, where reconnection
			// is scheduled. Reporting `closed` here would duplicate state changes.
		});
	};

	const sendPing = (connection: WebSocket): void => {
		if (connection.readyState !== WebSocket.OPEN) {
			return;
		}

		connection.send(JSON.stringify({ type: "ping" }));
	};

	connect();

	return {
		get status() {
			return status;
		},
		close() {
			disposed = true;
			clearTimers();
			socket?.close();
			socket = null;
			setStatus("closed");
		},
		ping(payload) {
			if (socket?.readyState !== WebSocket.OPEN) {
				return;
			}

			socket.send(
				JSON.stringify(
					payload === undefined ? { type: "ping" } : { type: "ping", payload },
				),
			);
		},
	};
}
