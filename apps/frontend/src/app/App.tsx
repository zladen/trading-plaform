import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router";

import { useLiveConnection } from "@/features/live-connection";
import { DashboardPage } from "@/pages/dashboard";

/**
 * Root component.
 *
 * Providers are created here and nowhere else: a single Query Client instance
 * owns all server state, and the live channel subscription lives at the root so
 * navigation cannot tear it down.
 */
export function App() {
	const [queryClient] = useState(
		() =>
			new QueryClient({
				defaultOptions: {
					queries: {
						retry: 1,
						refetchOnWindowFocus: false,
					},
				},
			}),
	);

	return (
		<QueryClientProvider client={queryClient}>
			<BrowserRouter>
				<AppShell />
			</BrowserRouter>
		</QueryClientProvider>
	);
}

function AppShell() {
	useLiveConnection();

	return (
		<main className="min-h-dvh bg-slate-950 text-slate-200">
			<Routes>
				<Route path="/" element={<DashboardPage />} />
				{/* Unknown paths return to the dashboard rather than a blank screen. */}
				<Route path="*" element={<Navigate to="/" replace />} />
			</Routes>
		</main>
	);
}
