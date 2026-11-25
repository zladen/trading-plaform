import { useQuery, type UseQueryResult } from "@tanstack/react-query";

import { fetchHealth, fetchInfo, fetchReadiness } from "@/shared/api";
import { buildSnapshot, type SystemSnapshot } from "./model";

/** Options that mirror React Query's `queryOptions` shape without importing it. */
export interface SystemSnapshotQueryOptions {
	/** Disables automatic refetching, used by tests and manual-only screens. */
	enabled?: boolean;
}

/**
 * Query key for the composed snapshot.
 *
 * The three endpoints are requested together because the dashboard cannot make
 * a meaningful statement from only one of them, and they are cheap.
 */
export const systemSnapshotQueryKey = ["system", "snapshot"] as const;

/**
 * Fetches health, readiness and info concurrently and composes the dashboard
 * snapshot.
 *
 * The three requests share one `AbortSignal`, so cancelling the query cancels
 * all in-flight calls. A failure in any of them fails the whole snapshot: the
 * UI must not claim the system is healthy from partial data.
 */
export async function loadSystemSnapshot(signal?: AbortSignal): Promise<SystemSnapshot> {
	const [health, readiness, info] = await Promise.all([
		fetchHealth(signal === undefined ? {} : { signal }),
		fetchReadiness(signal === undefined ? {} : { signal }),
		fetchInfo(signal === undefined ? {} : { signal }),
	]);

	return buildSnapshot({
		health: health.data,
		readiness: readiness.data,
		info: info.data,
		requestId: info.requestId,
	});
}

/**
 * React Query hook for the dashboard snapshot.
 *
 * Refetching is manual or interval based: the operator presses "Check again",
 * and background polling keeps the panel honest while it stays open.
 */
export function useSystemSnapshotQuery(
	options: SystemSnapshotQueryOptions = {},
): UseQueryResult<SystemSnapshot> {
	return useQuery({
		queryKey: systemSnapshotQueryKey,
		queryFn: ({ signal }) => loadSystemSnapshot(signal),
		enabled: options.enabled ?? true,
		staleTime: 5_000,
		// Retry policy belongs to the Query Client so the application and its
		// tests can configure it in one place.
		refetchOnWindowFocus: false,
	});
}
