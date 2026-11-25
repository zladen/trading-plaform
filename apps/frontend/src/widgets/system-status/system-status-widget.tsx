import { useSystemSnapshotQuery } from "@/entities/system-status";
import { RefreshSystemStatus } from "@/features/refresh-system-status";

import { SystemStatusError, SystemStatusLoading, SystemStatusPanel } from "./system-status-panel";

/**
 * Dashboard block that loads and renders infrastructure health.
 *
 * The four required states are explicit: loading, success, error (the request
 * failed) and unavailable (the backend answered, but a dependency is down —
 * rendered inside the success state so the operator can see which one).
 */
export function SystemStatusWidget() {
	const query = useSystemSnapshotQuery();

	return (
		<section aria-labelledby="system-status-heading" className="space-y-3">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<h2 id="system-status-heading" className="text-lg font-semibold text-slate-100">
					Infrastructure
				</h2>
				<RefreshSystemStatus />
			</div>

			{query.isPending ? <SystemStatusLoading /> : null}

			{query.isError ? (
				<SystemStatusError
					message={
						query.error instanceof Error
							? query.error.message
							: "The backend could not be reached."
					}
				/>
			) : null}

			{query.isSuccess ? <SystemStatusPanel snapshot={query.data} /> : null}
		</section>
	);
}
