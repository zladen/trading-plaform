import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

afterEach(() => {
	cleanup();
});

// `AbortSignal.timeout` exists in Node 24 and in jsdom, but its timer keeps the
// test process alive. Replacing it with a signal that never fires keeps tests
// deterministic while preserving the API shape the client relies on.
if (typeof AbortSignal.timeout !== "function") {
	vi.stubGlobal(
		"AbortSignal",
		Object.assign(AbortSignal, {
			timeout: () => new AbortController().signal,
		}),
	);
}

// jsdom does not implement `matchMedia`, which some UI primitives query.
if (typeof window.matchMedia !== "function") {
	vi.stubGlobal(
		"matchMedia",
		vi.fn().mockImplementation((query: string) => ({
			matches: false,
			media: query,
			onchange: null,
			addEventListener: vi.fn(),
			removeEventListener: vi.fn(),
			addListener: vi.fn(),
			removeListener: vi.fn(),
			dispatchEvent: vi.fn(),
		})),
	);
}
