import { render, screen } from "@testing-library/react";
import { createChart } from "lightweight-charts";
import { afterEach, describe, expect, it, vi } from "vitest";

import { PriceChart } from "./price-chart";

// `createChart` needs layout measurement, which jsdom does not perform. The
// real chart geometry is irrelevant to the behaviour under test: what matters
// is that the chart and its series are created and disposed exactly once.
vi.mock("lightweight-charts", () => {
	const series = { setData: vi.fn() };
	const chart = { addSeries: vi.fn(() => series), remove: vi.fn() };

	return {
		createChart: vi.fn(() => chart),
		LineSeries: "Line",
	};
});

afterEach(() => {
	vi.mocked(createChart).mockClear();
});

describe("PriceChart", () => {
	it("states that no market data is connected when the series is empty", () => {
		const { container } = render(<PriceChart data={[]} />);

		// The explicit placeholder is the point: the component must never draw a
		// synthetic curve to look busy. It is scoped to the visible element
		// because the same text is also the accessible description.
		const placeholder = container.querySelector("p:not(.sr-only)");
		expect(placeholder?.textContent).toBe("Market data is not connected yet");
		expect(screen.getAllByText("Market data is not connected yet")).toHaveLength(2);
		expect(createChart).toHaveBeenCalledOnce();
	});

	it("creates the chart and pushes the provided series into it", () => {
		const { container } = render(
			<PriceChart data={[{ time: 1_767_225_600 as never, value: 42.5 }]} />,
		);

		// A non-empty series removes the visible placeholder.
		const placeholder = container.querySelector("p:not(.sr-only)");
		expect(placeholder).toBeNull();

		const chart = vi.mocked(createChart).mock.results[0]?.value as {
			addSeries: ReturnType<typeof vi.fn>;
		};
		const series = chart.addSeries.mock.results[0]?.value as {
			setData: ReturnType<typeof vi.fn>;
		};

		expect(series.setData).toHaveBeenCalledWith([{ time: 1_767_225_600, value: 42.5 }]);
	});

	it("describes the rendered state instead of always announcing the placeholder", () => {
		const empty = render(<PriceChart data={[]} />);
		expect(empty.container.querySelector(".sr-only")?.textContent).toBe(
			"Market data is not connected yet",
		);
		empty.unmount();

		const filled = render(<PriceChart data={[{ time: 1 as never, value: 1 }]} />);
		expect(filled.container.querySelector(".sr-only")?.textContent).toBe(
			"Market data is not connected yet, 1 points",
		);
	});

	it("disposes the chart when it unmounts", () => {
		const { unmount } = render(<PriceChart data={[]} />);

		const chart = vi.mocked(createChart).mock.results[0]?.value as {
			remove: ReturnType<typeof vi.fn>;
		};

		expect(chart.remove).not.toHaveBeenCalled();
		unmount();
		expect(chart.remove).toHaveBeenCalledOnce();
	});
});
