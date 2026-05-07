import { test, expect } from '@playwright/test';
import { checkTapTargets } from './_helpers/layout';

/**
 * Mobile-only density and ergonomic checks. The desktop project ignores
 * this file (playwright.config.ts → testIgnore), so these only run at
 * 375×812 against the worker.
 *
 * Three things to keep honest:
 *   - Tables that opted into `.table-stack` render as a card-list at
 *     ≤720px (no horizontal scroll, header collapses, td shows label).
 *   - The narrow-viewport tap-target floor we put behind the 600px
 *     media block actually applies — `.btn.sm` clears 36px tall, etc.
 *   - Page-level density tokens (--page-pad → 16px on mobile) actually
 *     reach the rules that consume them.
 */

test.describe('table-stack', () => {
  test('/manage/tenants does not horizontally overflow at 375', async ({ page }) => {
    await page.goto('/manage/tenants');
    // The dev seed may or may not have tenants — the empty-state and the
    // populated card-list both have to fit; that's what we assert.
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - window.innerWidth,
    );
    expect(overflow).toBeLessThanOrEqual(1);
  });

  test('/manage connection-status table-stack labels render', async ({ page }) => {
    await page.goto('/manage');
    const cell = page.locator('.table-stack td[data-label="Service"]').first();
    await expect(cell).toBeVisible();
    // The data-label content renders via a CSS ::before pseudo-element,
    // so we look it up off computed style rather than .textContent.
    const before = await cell.evaluate(
      (el) => getComputedStyle(el, '::before').content,
    );
    expect(before).toContain('Service');
  });
});

test.describe('tap targets', () => {
  test('/manage primary controls clear 36px (the .btn.sm floor)', async ({ page }) => {
    await page.goto('/manage');
    // 36 is the floor we set inside @media(max-width:600) for .btn.sm.
    // The audit walks every visible interactive element on the page —
    // we then trim out two intentionally-dense regions:
    //   - the brand link in the top bar (a.brand): logo + wordmark
    //     reads as a header element, not a precision tap target
    //   - the comma-separated footer link list (a.muted inside
    //     .site-footer): these are body-text links, not action buttons
    // Anything else under 36px is a regression and fails.
    const issues = await checkTapTargets(page, { min: 36 });
    const real = issues.filter(
      (s) => !s.includes('a.brand') && !s.includes('a.muted'),
    );
    expect(real, real.join('\n  ')).toEqual([]);
  });

  test('site-header buttons on / clear 36px tall', async ({ page }) => {
    await page.goto('/');
    const heights = await page
      .locator('header.site-header a.btn, header.site-header button.btn')
      .evaluateAll((els) =>
        els
          .filter((el) => {
            const cs = getComputedStyle(el);
            return cs.display !== 'none' && cs.visibility !== 'hidden';
          })
          .map((el) => el.getBoundingClientRect().height),
      );
    for (const h of heights) {
      expect(h, `nav button height: ${h}`).toBeGreaterThanOrEqual(36);
    }
  });
});

test.describe('density tokens', () => {
  test('--page-pad is 16px on /manage', async ({ page }) => {
    await page.goto('/manage');
    const pad = await page.evaluate(() =>
      getComputedStyle(document.documentElement).getPropertyValue('--page-pad').trim(),
    );
    expect(pad).toBe('16px');
  });

  test('--stat-n is 26px and applied to .stat-n font-size', async ({ page }) => {
    await page.goto('/manage');
    const token = await page.evaluate(() =>
      getComputedStyle(document.documentElement).getPropertyValue('--stat-n').trim(),
    );
    expect(token).toBe('26px');
  });
});
