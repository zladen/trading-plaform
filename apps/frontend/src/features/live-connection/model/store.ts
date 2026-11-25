import { create } from "zustand";

import type { ConnectionStatus, ServerMessage } from "@/shared/api";

interface WebSocketState {
	status: ConnectionStatus;
	/** Protocol version reported by the server on the `connected` frame. */
	protocolVersion: number | null;
	/** Connection identifier assigned by the server. */
	connectionId: string | null;
	/** Server time from the most recent heartbeat. */
	lastHeartbeatAt: string | null;
	/** Total number of server messages observed on this connection. */
	messageCount: number;
	setStatus: (status: ConnectionStatus) => void;
	handleServerMessage: (message: ServerMessage) => void;
	reset: () => void;
}

const initialState = {
	status: "connecting" as ConnectionStatus,
	protocolVersion: null,
	connectionId: null,
	lastHeartbeatAt: null,
	messageCount: 0,
};

/**
 * Client-side liveness state of the live channel.
 *
 * This is transport state only. Market data and trading events belong to
 * server state and must live in React Query, never here.
 */
export const useWebSocketStore = create<WebSocketState>((set) => ({
	...initialState,
	setStatus: (status) => {
		set({ status });
	},
	handleServerMessage: (message) => {
		set((state) => {
			switch (message.type) {
				case "connected":
					return {
						...state,
						connectionId: message.connection_id,
						protocolVersion: message.protocol_version,
						messageCount: state.messageCount + 1,
					};
				case "heartbeat":
					return {
						...state,
						lastHeartbeatAt: message.server_time,
						messageCount: state.messageCount + 1,
					};
				case "closing":
				case "pong":
					return { ...state, messageCount: state.messageCount + 1 };
				default:
					return state;
			}
		});
	},
	reset: () => {
		set(initialState);
	},
}));
