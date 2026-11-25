import { Badge } from "@/shared/ui";

import type { ComponentState, ComponentStatus } from "../model";

const TONE_BY_STATE: Record<ComponentState, "positive" | "negative" | "warning" | "neutral"> = {
	up: "positive",
	down: "negative",
	degraded: "warning",
	unknown: "neutral",
};

const LABEL_BY_STATE: Record<ComponentState, string> = {
	up: "Operational",
	down: "Unavailable",
	degraded: "Degraded",
	unknown: "Unknown",
};

/** Status pill for one component. */
export function StatusBadge({ state }: { state: ComponentState }) {
	return (
		<Badge tone={TONE_BY_STATE[state]} data-state={state}>
			<span aria-hidden="true" className="size-1.5 rounded-full bg-current" />
			{LABEL_BY_STATE[state]}
		</Badge>
	);
}

/**
 * One component row: name, state pill and measured detail.
 *
 * Rendered as a definition list entry so the relationship between the component
 * name and its state is explicit for assistive technology.
 */
export function ComponentStatusRow({ status }: { status: ComponentStatus }) {
	return (
		<div
			className="flex items-start justify-between gap-4 border-b border-slate-800/70 py-3 last:border-b-0"
			data-component={status.id}
		>
			<div className="min-w-0">
				<dt className="truncate text-sm font-medium text-slate-100">{status.label}</dt>
				{status.detail === null ? null : (
					<dd className="mt-0.5 truncate text-xs text-slate-500">{status.detail}</dd>
				)}
			</div>
			<dd className="shrink-0">
				<StatusBadge state={status.state} />
			</dd>
		</div>
	);
}
