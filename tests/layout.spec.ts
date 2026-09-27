import { test, expect } from '@playwright/test';
import { checkLayout } from './_helpers/layout';

/**
 * Layout sanity sweep across the widths real users see:
 *   - 320 / 360         iPhone SE & Galaxy S8-class phones
 *   - 375 / 414          iPhone 8–12 Pro Max
 *   - 601 / 700          the band just above our 600px mobile breakpoint
 *                        (regression-prone — the desktop nav re-engages
 *                        here and used to wrap to two rows)
 *   - 768 / 1024         small tablet, laptop
 *
 * For each (path, width) we assert: page doesn't horizontally overflow,
 * `.site-header` stays single-row, no nav button slides past the edge.
 * Failures bubble up a deepest-leaf "culprit" pointer so a regression
 * goes from "the page looks weird" to a one-line element selector.
 */
const WIDTHS = [320, 360, 375, 414, 601, 700, 768, 1024];
// The operator panel used to be swept here too. It's API-only now, so
// there's no markup to measure.
const PATHS = ['/', '/features', '/pricing', '/login', '/terms', '/privacy'];

// Each test sets its own viewport, so we don't actually depend on project-
// level viewport defaults. The mobile project ignores this file via
// playwright.config.ts so we only run the sweep once.

for (const path of PATHS) {
  for (const width of WIDTHS) {
    test(`layout @ ${width}px on ${path}`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      await page.goto(path);
      // Elm renders after the bundle loads; measuring before it mounts would
      // check the layout of an empty body. `.site-main` is Elm's own
      // container, so its presence proves the bundle booted.
      await page.locator('main.site-main').waitFor();
      const issues = await checkLayout(page, width);
      expect(issues, issues.join('\n  ')).toEqual([]);
    });
  }
}
