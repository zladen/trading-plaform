import {
	ComponentStatusRow,
	deriveSystemState,
	type SystemSnapshot,
	type SystemState,
} from "@/entities/system-status";
import { formatTimestamp, formatUptime } from "@/shared/lib";
import { Badge, Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/shared/ui";

const STATE_TONE: Record<SystemState, "positive" | "negative" | "warning"> = {
	operational: "positive",
	degraded: "warning",
	unavailable: "negative",
};

const STATE_LABEL: Record<SystemState, string> = {
	operational: "All systems operational",
	degraded: "Degraded",
	unavailable: "Unavailable",
};

/** Headline state for the whole platform. */
export function SystemStateSummary({ snapshot }: { snapshot: SystemSnapshot }) {
	const state = deriveSystemState(snapshot);

	return (
		<Badge tone={STATE_TONE[state]} data-system-state={state} className="text-xs">
			{STATE_LABEL[state]}
		</Badge>
	);
}

/** Successful state: every value shown comes from the backend responses. */
export function SystemStatusPanel({ snapshot }: { snapshot: SystemSnapshot }) {
	return (
		<Card>
			<CardHeader>
				<div className="flex flex-wrap items-center justify-between gap-3">
					<CardTitle>System status</CardTitle>
					<SystemStateSummary snapshot={snapshot} />
				</div>
				<CardDescription>
					Version {snapshot.info.version} · {snapshot.info.environment} · uptime{" "}
					{formatUptime(snapshot.info.uptimeSeconds)}
				</CardDescription>
			</CardHeader>
			<CardContent>
				<dl>
					<ComponentStatusRow status={snapshot.backend} />
					{snapshot.dependencies.map((dependency) => (
						<ComponentStatusRow
							key={`${dependency.id}-${dependency.label}`}
							status={dependency}
						/>
					))}
				</dl>
				<p className="mt-4 text-xs text-slate-500">
					Checked {formatTimestamp(snapshot.checkedAt)} · request {snapshot.requestId}
				</p>
			</CardContent>
		</Card>
	);
}

/** Loading state: skeleton rows, announced politely. */
export function SystemStatusLoading() {
	return (
		<Card aria-busy="true" aria-live="polite">
			<CardHeader>
				<CardTitle>System status</CardTitle>
				<CardDescription>Contacting the backend…</CardDescription>
			</CardHeader>
			<CardContent>
				<div className="space-y-3">
					{[0, 1, 2].map((row) => (
						<div key={row} className="h-10 animate-pulse rounded-md bg-slate-800/60" />
					))}
				</div>
			</CardContent>
		</Card>
	);
}

/** Error state: the backend could not be asked at all. */
export function SystemStatusError({ message }: { message: string }) {
	return (
		<Card role="alert" className="border-rose-500/40">
			<CardHeader>
				<CardTitle>System status unavailable</CardTitle>
				<CardDescription>{message}</CardDescription>
			</CardHeader>
			<CardContent>
				<p className="text-sm text-slate-400">
					The dashboard only displays values reported by the backend. Start the backend
					and check again.
				</p>
			</CardContent>
		</Card>
	);
}
