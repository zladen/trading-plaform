import path from "node:path";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { loadEnv } from "vite";
import { defineConfig } from "vitest/config";

export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, process.cwd(), "");

	return {
		plugins: [react(), tailwindcss()],
		resolve: {
			alias: {
				"@": path.resolve(import.meta.dirname, "src"),
			},
		},
		server: {
			port: 5173,
			strictPort: true,
			// Same-origin API access in development: the browser talks to the Vite
			// dev server, which forwards `/api` to the backend. This keeps the
			// frontend free of hardcoded hosts and works with the container setup.
			proxy: {
				"/api": {
					target: env.VITE_DEV_API_TARGET ?? "http://127.0.0.1:8080",
					changeOrigin: true,
					ws: true,
				},
			},
		},
		preview: {
			port: 4173,
			strictPort: true,
		},
		build: {
			outDir: "dist",
			sourcemap: false,
		},
		test: {
			environment: "jsdom",
			setupFiles: ["./src/test/setup.ts"],
			include: ["src/**/*.test.{ts,tsx}"],
			restoreMocks: true,
			clearMocks: true,
		},
	};
});
