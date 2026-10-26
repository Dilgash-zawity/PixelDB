import { defineConfig } from '@playwright/test';

/**
 * UI tests: the frontend in a real browser (the installed Chrome) against a
 * fake backend (e2e/fake-backend.ts) that answers Tauri commands in memory.
 * Run with `bun run test:ui`; the Vite dev server is started for them.
 */
export default defineConfig({
	testDir: 'e2e',
	timeout: 60_000,
	// The dev server compiles each screen on its first visit.
	expect: { timeout: 20_000 },
	fullyParallel: true,
	reporter: process.env.CI ? 'github' : 'list',
	use: {
		baseURL: 'http://localhost:5179',
		channel: process.env.PLAYWRIGHT_CHANNEL ?? 'chrome',
		trace: 'retain-on-failure'
	},
	webServer: {
		command: 'bunx vite dev --port 5179 --strictPort',
		url: 'http://localhost:5179',
		reuseExistingServer: !process.env.CI,
		timeout: 120_000
	}
});
