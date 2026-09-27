import { test as base, expect } from '@playwright/test';

/**
 * Auto-collected console errors / page errors / CSP violations.
 *
 * Every test gets a `consoleErrors` array populated as the page runs;
 * specs assert it's empty (or filter what they expect to ignore) at
 * the relevant point. CSP-related console messages are routed here
 * too so a fresh `'unsafe-inline'`-needing line in a template fails
 * the suite instead of slipping past unnoticed.
 */
export const test = base.extend<{ consoleErrors: string[] }>({
  consoleErrors: async ({ page }, use) => {
    const errors: string[] = [];
    page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
    page.on('console', (msg) => {
      const text = msg.text();
      const lower = text.toLowerCase();
      if (msg.type() === 'error') errors.push(`console: ${text}`);
      else if (lower.includes('content security policy') || lower.includes('refused to')) {
        errors.push(`csp: ${text}`);
      }
    });
    await use(errors);
  },
});

export { expect };

/**
 * Wait until the Elm app has actually booted.
 *
 * Necessary because the marketing routes are prerendered: their markup is a
 * snapshot of Elm's own output, so every selector you might wait on is already
 * present in the static HTML before the bundle has run. Waiting on markup
 * therefore proves nothing and races the bundle — a click landing in that
 * window gets handled by the browser as a plain link, not by Elm's router.
 *
 * `window.__conciergeBooted` is set by `public/boot.js` after
 * `Elm.Main.init` returns. A JS global can't be captured by a DOM snapshot,
 * which is exactly why it's used instead of an attribute.
 */
export async function waitForBoot(page: import('@playwright/test').Page) {
  await page.waitForFunction(
    () => (window as unknown as { __conciergeBooted?: boolean }).__conciergeBooted === true,
  );
}
