import { RefreshCw } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";

import { systemSnapshotQueryKey } from "@/entities/system-status";
import { Button } from "@/shared/ui";

export interface RefreshSystemStatusProps {
	className?: string;
}

/**
 * Re-checks backend state on demand.
 *
 * The button invalidates the snapshot query instead of duplicating fetch logic,
 * so a refresh uses exactly the same code path as the initial load.
 */
export function RefreshSystemStatus({ className }: RefreshSystemStatusProps) {
	const queryClient = useQueryClient();

	const handleRefresh = (): void => {
		void queryClient.invalidateQueries({ queryKey: systemSnapshotQueryKey });
	};

	return (
		<Button
			type="button"
			variant="outline"
			onClick={handleRefresh}
			className={className}
			aria-label="Check system status again"
		>
			<RefreshCw aria-hidden="true" className="size-4" />
			Check again
		</Button>
	);
}
