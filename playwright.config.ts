import { defineConfig, devices } from '@playwright/test';

const PORT = 8787;
const BASE_URL = `http://localhost:${PORT}`;

/**
 * Playwright Test config.
 *
 * Three projects:
 * - `desktop` and `mobile` run the full behavioural + layout suite at
 *   each viewport. They're what `npm test` exercises and what CI gates
 *   on.
 * - `screenshots` only runs `tests/visual.spec.ts` and writes the PNGs
 *   the docs gallery embeds. It's invoked via `npm run screenshots` and
 *   is *not* part of the default run — capturing screenshots churns the
 *   doc/screenshots/ files on every push otherwise.
 *
 * The dev server starts once per `playwright test` invocation via the
 * shim in `scripts/test-server.mjs`, which writes stub OAuth secrets so
 * /auth/login renders the real login template (not the maintenance
 * fallback the worker shows when essentials are missing).
 */
export default defineConfig({
  testDir: './tests',
  fullyParallel: false, // single dev server, sequential keeps logs readable
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: process.env.CI ? [['github'], ['html', { open: 'never' }]] : 'list',
  outputDir: 'test-results',

  use: {
    baseURL: BASE_URL,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },

  projects: [
    {
      name: 'desktop',
      use: { ...devices['Desktop Chrome'], viewport: { width: 1280, height: 800 } },
      testIgnore: /visual\.spec\.ts/,
    },
    {
      name: 'mobile',
      use: { ...devices['Desktop Chrome'], viewport: { width: 375, height: 812 } },
      // The layout sweep manages its own viewports, so running it under both
      // projects would just duplicate work. Keep it desktop-only.
      testIgnore: [/visual\.spec\.ts/, /layout\.spec\.ts/],
    },
    {
      name: 'screenshots',
      use: { ...devices['Desktop Chrome'] },
      testMatch: /visual\.spec\.ts/,
    },
  ],

  webServer: {
    command: 'node scripts/test-server.mjs',
    url: BASE_URL,
    // Wipes .wrangler/state before boot. Interactive `nix run .#dev`
    // leaves this unset so dev state persists across restarts.
    env: { CONCIERGE_TEST_RESET: '1' },
    reuseExistingServer: !process.env.CI,
    timeout: 240_000, // first wasm build can take 2+ minutes
    stdout: 'pipe',
    stderr: 'pipe',
  },
});
