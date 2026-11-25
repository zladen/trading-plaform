import { LiveChannelWidget } from "@/widgets/live-channel";
import { SystemStatusWidget } from "@/widgets/system-status";
import { Card, CardContent, CardDescription, CardHeader, CardTitle, PriceChart } from "@/shared/ui";

/**
 * Landing page of V0.1.
 *
 * It reports infrastructure state only: the process, PostgreSQL, Redis, the
 * build version and the live channel. It intentionally contains no trading
 * widgets, charts of real prices or account data, because none of that exists
 * yet.
 */
export function DashboardPage() {
	return (
		<div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-4 py-8 sm:px-6 lg:py-12">
			<header className="space-y-2">
				<p className="text-xs font-semibold tracking-[0.2em] text-sky-400 uppercase">
					Platform V0.1
				</p>
				<h1 className="text-2xl font-semibold text-slate-50 sm:text-3xl">
					Infrastructure dashboard
				</h1>
				<p className="max-w-2xl text-sm text-slate-400">
					Live state reported by the backend API: process liveness, PostgreSQL and Redis
					readiness, build version and the WebSocket transport. Trading functionality is
					not part of this stage.
				</p>
			</header>

			<div className="grid items-start gap-6 lg:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]">
				<SystemStatusWidget />
				<LiveChannelWidget />
			</div>

			<Card>
				<CardHeader>
					<CardTitle>Price chart</CardTitle>
					<CardDescription>
						TradingView Lightweight Charts is installed and mounted, but V0.1 has no
						market data source. The chart stays empty rather than drawing a synthetic
						series.
					</CardDescription>
				</CardHeader>
				<CardContent>
					<PriceChart data={[]} label="Market data is not connected yet" height={200} />
				</CardContent>
			</Card>

			<footer className="border-t border-slate-800 pt-4 text-xs text-slate-500">
				Values are fetched from <code className="text-slate-400">/api/v1/health</code>,{" "}
				<code className="text-slate-400">/api/v1/ready</code> and{" "}
				<code className="text-slate-400">/api/v1/info</code>. Nothing on this page is
				simulated.
			</footer>
		</div>
	);
}
