import { useEffect } from "react";

import { createWebSocketClient } from "@/shared/api";
import { useWebSocketStore } from "../model/store";

export interface LiveConnectionOptions {
	/** Disables the connection, used in tests and on pages without live data. */
	enabled?: boolean;
}

/**
 * Keeps the live channel open for as long as the calling component is mounted.
 *
 * The hook owns the socket lifecycle and mirrors server messages into the
 * Zustand store. It deliberately does not retry forever: a backend that cannot
 * accept WebSocket connections is a real failure the dashboard must show, not
 * something to hide behind endless reconnection.
 */
export function useLiveConnection(options: LiveConnectionOptions = {}): void {
	const enabled = options.enabled ?? true;

	const setStatus = useWebSocketStore((state) => state.setStatus);
	const handleServerMessage = useWebSocketStore((state) => state.handleServerMessage);
	const reset = useWebSocketStore((state) => state.reset);

	useEffect(() => {
		if (!enabled) {
			return;
		}

		const handleMessage = (message: Parameters<typeof handleServerMessage>[0]): void => {
			handleServerMessage(message);
		};

		const client = createWebSocketClient({
			onStatusChange: setStatus,
			onMessage: handleMessage,
		});

		return () => {
			client.close();
			reset();
		};
	}, [enabled, handleServerMessage, reset, setStatus]);
}
