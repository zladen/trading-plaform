import {
	createChart,
	LineSeries,
	type IChartApi,
	type ISeriesApi,
	type UTCTimestamp,
} from "lightweight-charts";
import { useEffect, useId, useRef } from "react";

import { cn } from "@/shared/lib";

/** One point of a price series. */
export interface PricePoint {
	/** Unix timestamp in seconds, as required by Lightweight Charts. */
	time: UTCTimestamp;
	value: number;
}

export interface PriceChartProps {
	/** Series to render. An empty array renders the placeholder state. */
	data: readonly PricePoint[];
	/** Colour of the line series. */
	color?: string;
	/** Height of the chart canvas in pixels. */
	height?: number;
	/** Accessible description of the chart. */
	label?: string;
	className?: string;
}

/**
 * Line chart for price series, built on TradingView Lightweight Charts.
 *
 * V0.1 has no market data feed, so the dashboard mounts this component with an
 * empty series and the component states that explicitly instead of drawing a
 * synthetic curve. The chart instance is created once per mount and disposed on
 * unmount, which is the documented lifecycle for the library.
 */
export function PriceChart({
	data,
	color = "#38bdf8",
	height = 220,
	label = "Market data is not connected yet",
	className,
}: PriceChartProps) {
	const containerRef = useRef<HTMLDivElement | null>(null);
	const chartRef = useRef<IChartApi | null>(null);
	const seriesRef = useRef<ISeriesApi<"Line"> | null>(null);
	const descriptionId = useId();

	useEffect(() => {
		const container = containerRef.current;

		if (container === null) {
			return;
		}

		const chart = createChart(container, {
			height,
			layout: {
				background: { color: "transparent" },
				textColor: "#94a3b8",
				attributionLogo: false,
			},
			grid: {
				vertLines: { color: "rgba(148, 163, 184, 0.12)" },
				horzLines: { color: "rgba(148, 163, 184, 0.12)" },
			},
			rightPriceScale: { borderColor: "rgba(148, 163, 184, 0.25)" },
			timeScale: { borderColor: "rgba(148, 163, 184, 0.25)" },
			crosshair: { mode: 0 },
			handleScroll: false,
			handleScale: false,
			autoSize: true,
		});

		const series = chart.addSeries(LineSeries, {
			color,
			lineWidth: 2,
			priceLineVisible: false,
			lastValueVisible: false,
		});

		chartRef.current = chart;
		seriesRef.current = series;

		return () => {
			chart.remove();
			chartRef.current = null;
			seriesRef.current = null;
		};
	}, [color, height]);

	useEffect(() => {
		seriesRef.current?.setData([...data]);
	}, [data]);

	return (
		<div className={cn("relative w-full", className)} style={{ height }}>
			<div
				ref={containerRef}
				className="size-full"
				role="img"
				aria-labelledby={descriptionId}
			/>
			{/* The accessible description matches what is actually rendered: an
			    empty series must not be announced as market data. */}
			<p id={descriptionId} className="sr-only">
				{data.length === 0 ? label : `${label}, ${data.length} points`}
			</p>
			{data.length === 0 ? (
				<div className="pointer-events-none absolute inset-0 grid place-items-center">
					<p className="rounded-md border border-slate-800 bg-slate-950/70 px-3 py-1.5 text-xs text-slate-500">
						{label}
					</p>
				</div>
			) : null}
		</div>
	);
}
