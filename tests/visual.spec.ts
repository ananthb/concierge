import { test } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';

/**
 * Screenshot capture for the docs gallery.
 *
 * Only runs in the `screenshots` project (see playwright.config.ts), so
 * `npm test` doesn't churn `doc/screenshots/` on every run. Trigger
 * with `npm run screenshots`.
 *
 * Each public page captures a desktop and a mobile shot; mobile uses
 * `fullPage: true` so layout breakage further down the scroll
 * (footer, hero phone stack) is visible without manual scrolling.
 *
 * `process.cwd()` is the repo root — Playwright always invokes specs
 * from there, and avoiding `import.meta.url` keeps esbuild's CJS
 * transpile happy.
 */
const OUTPUT_DIR = join(process.cwd(), 'doc', 'screenshots');

const DESKTOP = { width: 1280, height: 800 };
const MOBILE = { width: 375, height: 812 };

const SHOTS = [
  { name: 'home.png', path: '/', viewport: DESKTOP },
  { name: 'home-mobile.png', path: '/', viewport: MOBILE },
  { name: 'login.png', path: '/auth/login', viewport: DESKTOP },
  { name: 'login-mobile.png', path: '/auth/login', viewport: MOBILE },
  { name: 'features.png', path: '/features', viewport: DESKTOP },
  { name: 'features-mobile.png', path: '/features', viewport: MOBILE },
  { name: 'pricing.png', path: '/pricing', viewport: DESKTOP },
  { name: 'pricing-mobile.png', path: '/pricing', viewport: MOBILE },
  { name: 'terms.png', path: '/terms', viewport: DESKTOP },
  { name: 'terms-mobile.png', path: '/terms', viewport: MOBILE },
  { name: 'privacy.png', path: '/privacy', viewport: DESKTOP },
  { name: 'privacy-mobile.png', path: '/privacy', viewport: MOBILE },
];

test.beforeAll(async () => {
  await mkdir(OUTPUT_DIR, { recursive: true });
});

for (const shot of SHOTS) {
  test(`capture ${shot.name}`, async ({ page }) => {
    await page.setViewportSize(shot.viewport);
    await page.goto(shot.path);
    // Settle any web fonts / fade-in animations.
    await page.waitForTimeout(400);
    const isMobile = shot.viewport.width <= 480;
    await page.screenshot({
      path: join(OUTPUT_DIR, shot.name),
      fullPage: isMobile,
    });
  });
}
