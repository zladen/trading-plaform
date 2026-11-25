import { useWebSocketStore } from "@/features/live-connection";
import { formatTimestamp } from "@/shared/lib";
import { Badge, Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/shared/ui";

const TONE = {
	open: "positive",
	connecting: "warning",
	closed: "negative",
} as const;

const LABEL = {
	open: "Connected",
	connecting: "Connecting",
	closed: "Disconnected",
} as const;

/**
 * Live channel panel.
 *
 * Shows only transport facts reported by the server: connection state, protocol
 * version, connection identifier and the last heartbeat. No market data exists
 * in V0.1, and this panel does not pretend otherwise.
 */
export function LiveChannelWidget() {
	const status = useWebSocketStore((state) => state.status);
	const protocolVersion = useWebSocketStore((state) => state.protocolVersion);
	const connectionId = useWebSocketStore((state) => state.connectionId);
	const lastHeartbeatAt = useWebSocketStore((state) => state.lastHeartbeatAt);
	const messageCount = useWebSocketStore((state) => state.messageCount);

	return (
		<Card>
			<CardHeader>
				<div className="flex flex-wrap items-center justify-between gap-3">
					<CardTitle>Live channel</CardTitle>
					<Badge tone={TONE[status]} data-ws-status={status}>
						<span aria-hidden="true" className="size-1.5 rounded-full bg-current" />
						{LABEL[status]}
					</Badge>
				</div>
				<CardDescription>
					WebSocket transport only. Market data and trading events are not part of V0.1.
				</CardDescription>
			</CardHeader>
			<CardContent>
				<dl className="text-sm">
					<div className="flex justify-between gap-4 py-1.5">
						<dt className="text-slate-400">Protocol</dt>
						<dd className="font-medium text-slate-100">
							{protocolVersion === null ? "—" : `v${protocolVersion}`}
						</dd>
					</div>
					<div className="flex justify-between gap-4 py-1.5">
						<dt className="text-slate-400">Connection</dt>
						<dd className="truncate font-mono text-xs text-slate-300">
							{connectionId ?? "—"}
						</dd>
					</div>
					<div className="flex justify-between gap-4 py-1.5">
						<dt className="text-slate-400">Last heartbeat</dt>
						<dd className="font-medium text-slate-100">
							{lastHeartbeatAt === null ? "—" : formatTimestamp(lastHeartbeatAt)}
						</dd>
					</div>
					<div className="flex justify-between gap-4 py-1.5">
						<dt className="text-slate-400">Messages</dt>
						<dd className="font-medium text-slate-100 tabular-nums">{messageCount}</dd>
					</div>
				</dl>
			</CardContent>
		</Card>
	);
}
