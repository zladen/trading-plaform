import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { LiveChannelWidget } from "@/widgets/live-channel";

describe("LiveChannelWidget", () => {
	it("reports the transport state without inventing data", () => {
		render(<LiveChannelWidget />);

		// No server frame has arrived at this point, so the panel must show the
		// initial transport state and placeholder values instead of fake data.
		expect(screen.getByText("Connecting")).toBeInTheDocument();
		expect(screen.getByText("Protocol")).toBeInTheDocument();
		expect(screen.getAllByText("—").length).toBeGreaterThan(0);
	});
});
