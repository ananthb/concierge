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
const PATHS = ['/', '/features', '/pricing', '/auth/login', '/terms', '/privacy'];
// Authed admin paths — dev-bypass is active in the test server so these
// render without a real Cloudflare Access JWT. They share the same 8-width
// sweep so the mobile refresh catches overflow on the management panel
// (tenants list, audit log, archetypes, billing matrix) at the same widths
// the public pages already gate on.
const MANAGE_PATHS = [
  '/manage',
  '/manage/tenants',
  '/manage/audit',
  '/manage/billing',
  '/manage/archetypes',
];

// Each test sets its own viewport, so we don't actually depend on project-
// level viewport defaults. The mobile project ignores this file via
// playwright.config.ts so we only run the sweep once.

for (const path of [...PATHS, ...MANAGE_PATHS]) {
  for (const width of WIDTHS) {
    test(`layout @ ${width}px on ${path}`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      await page.goto(path);
      const issues = await checkLayout(page, width);
      expect(issues, issues.join('\n  ')).toEqual([]);
    });
  }
}
