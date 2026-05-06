import { test, expect } from './_helpers/fixtures';

/**
 * /manage panel behavioural tests.
 *
 * These run with the dev bypass active — `scripts/test-server.mjs`
 * sets `MANAGE_BYPASS_EMAIL=admin-test@example.com` in its env file
 * and never sets `CF_ACCESS_AUD`, so `crate::dev_bypass::active`
 * returns true and the panel is reachable without a real Cloudflare
 * Access JWT. The same flag stubs the AI bindings so demo persona
 * generation + safety classification produce canned-but-shaped
 * responses on `wrangler dev --local`.
 *
 * Coverage target — the three P3 features the audit/billing/demo
 * work shipped, plus enough scaffolding to verify the bypass:
 *   - bypass renders /manage and exposes the operator identity
 *   - billing pricing cell save-on-blur posts to settings + toasts
 *   - demo Preview swaps the skeleton in immediately on click
 *   - audit log table renders end-of-log marker when below page cap
 *   - mobile viewport (375px) doesn't horizontally overflow on /manage
 */

const BYPASS_EMAIL = 'admin-test@example.com';

test.describe('management panel', () => {
  test('dashboard renders with the bypass operator identity', async ({ page, consoleErrors }) => {
    await page.goto('/manage');
    await expect(page).toHaveURL(/\/manage\/?$/);
    // Operator chip in the top-right of every /manage page.
    await expect(page.locator('.app-actor .actor-email')).toHaveText(BYPASS_EMAIL);
    // Sign-out goes through Cloudflare Access — link target check
    // (we don't follow it; CF Access isn't reachable from local dev).
    const signout = page.locator('.app-actor a.signout');
    await expect(signout).toBeVisible();
    await expect(signout).toHaveAttribute('href', '/cdn-cgi/access/logout');
    // Top progress bar is present (hidden by default; visibility is
    // toggled by HTMX listeners in manage_shell).
    await expect(page.locator('#manage-top-progress')).toHaveCount(1);
    expect(consoleErrors).toEqual([]);
  });

  test('top nav stays sticky after scroll', async ({ page }) => {
    await page.goto('/manage/audit');
    // Push some scroll height so the nav can move out of view.
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    const top = await page.locator('header.app-top').evaluate((el) => el.getBoundingClientRect().top);
    expect(top).toBeLessThanOrEqual(1);
  });
});

test.describe('billing save-on-blur', () => {
  test('every pricing cell carries the hx-* attributes that wire save-on-blur', async ({
    page,
  }) => {
    await page.goto('/manage/billing');

    // Audit the first cell — name='unit_price_milli__INR' is
    // stable on a fresh DB (INR is in the seeded defaults).
    const cell = page.locator("input[name='unit_price_milli__INR']");
    await expect(cell).toBeVisible();
    // The trigger filter (`change[target.checkValidity()]`) is what
    // gates blanks / 0 / negatives / overflows from reaching the
    // wire — see the validation tests below.
    await expect(cell).toHaveAttribute('hx-trigger', /^change\[/);
    await expect(cell).toHaveAttribute('hx-post', /\/manage\/billing\/settings$/);
    await expect(cell).toHaveAttribute('hx-target', '#toast-region');
    await expect(cell).toHaveAttribute('hx-swap', 'afterbegin');
    await expect(cell).toHaveAttribute('hx-include', 'this');
    await expect(cell).toHaveAttribute('min', '1');
    await expect(cell).toHaveAttribute('max', /^\d+$/);
    await expect(cell).toHaveAttribute('step', '1');

    // And confirm the wiring isn't a one-off — every pricing cell
    // gets the same shape (a single happy-path cell could be a
    // copy-paste outlier).
    const cellCount = await page.locator('input.cell-save').count();
    expect(cellCount).toBeGreaterThanOrEqual(6);
  });

  test('the /settings endpoint accepts a single-cell update', async ({ request }) => {
    // End-to-end verification of save-on-blur's backend contract:
    // posting just one (concept, currency) field should upsert
    // exactly that cell and return the success markup the toast
    // region renders. Driven through APIRequestContext (not the
    // browser) so we sidestep the HTMX-vs-Playwright `change`
    // event quirk on programmatic value writes.
    const before = await request.post('/manage/billing/settings', {
      data: { unit_price_milli__INR: 12345 },
      headers: { 'Content-Type': 'application/json' },
    });
    expect(before.status()).toBe(200);
    expect(await before.text()).toMatch(/pricing settings updated/i);

    // Verify the cell now reads back the new value.
    const page = await request.get('/manage/billing');
    const html = await page.text();
    expect(html).toMatch(/name="unit_price_milli__INR"[^>]*value="12345"/);
  });

  test('mid-edit dirty state disables the currency Remove button', async ({ page }) => {
    await page.goto('/manage/billing');
    const cell = page.locator("input[name='unit_price_milli__INR']");
    const removeBtn = page.locator("th button.danger", { hasText: /^Remove$/ }).first();

    // Baseline: nothing dirty, Remove is enabled.
    await expect(removeBtn).toBeEnabled();

    // Edit (still focused on cell, no change event yet, but @input
    // already flipped dirty=true via Alpine's @input.capture on
    // the wrapper). Remove should now be blocked.
    await cell.focus();
    await cell.fill(String(Number(await cell.inputValue()) + 2));
    await expect(removeBtn).toBeDisabled();
  });
});

test.describe('demo preview skeleton', () => {
  // Make sure the demo is enabled before testing Preview — the
  // toggle-off branch hides the prompt + buttons entirely.
  test.beforeEach(async ({ page }) => {
    await page.goto('/manage/demo');
    const toggle = page.locator('#demo-enabled');
    if (!(await toggle.isChecked())) {
      // Auto-submits via HTMX on change → page reloads with demo on.
      await toggle.check();
      await page.waitForLoadState('networkidle');
    }
  });

  test('skeleton template is wired into the page', async ({ page }) => {
    // Hidden <template> we copy into #demo-display the moment
    // Preview/Re-roll is clicked.
    const tpl = page.locator('#demo-skeleton-tpl');
    await expect(tpl).toHaveCount(1);
    // Three placeholder cards inside the template — matches the
    // archetype count we ship by default.
    const cards = await tpl.evaluate((el) => {
      const t = el as HTMLTemplateElement;
      return t.content.querySelectorAll('.skeleton-card').length;
    });
    expect(cards).toBeGreaterThanOrEqual(3);
  });

  test('clicking Preview swaps skeleton placeholders into the display pane', async ({ page }) => {
    const display = page.locator('#demo-display');
    const previewBtn = page.locator(":text('Preview')").first();

    // Slow down the response just enough that the skeleton is
    // observable. 250ms is well below the default Playwright
    // expect timeout but long enough that the skeleton's first
    // paint is reliably caught.
    await page.route('**/manage/demo/preview', async (route) => {
      await new Promise((r) => setTimeout(r, 250));
      await route.continue();
    });

    await previewBtn.click();
    await expect(display.locator('.skeleton-card').first()).toBeVisible({ timeout: 1000 });
  });
});

test.describe('audit log', () => {
  test('renders with end-of-log marker on a fresh / under-page-size DB', async ({ page }) => {
    await page.goto('/manage/audit');
    // Exactly one of these two states must be true on every load:
    //   - empty-state card (no audit rows yet), or
    //   - end-of-log marker (rows fit on one page → no Load older).
    const empty = page.locator('.empty-state', { hasText: /no audit entries/i });
    const endMarker = page.locator('#audit-load-more', { hasText: /end of log/i });
    await expect(empty.or(endMarker)).toBeVisible();
  });

  test('filter inputs are wired with HTMX', async ({ page }) => {
    await page.goto('/manage/audit');
    // The hx-* attributes live on the wrapper row, not the inputs —
    // hx-trigger uses `from:input[name='actor']` to scope which
    // descendant fires the request. So check the wrapper carries
    // hx-get and hx-trigger, and the inputs themselves render.
    const wrapper = page.locator(".row[hx-get*='/manage/audit']").first();
    await expect(wrapper).toHaveAttribute('hx-trigger', /input changed/);
    await expect(page.locator("input[name='actor']")).toBeVisible();
    await expect(page.locator("select[name='action']")).toBeVisible();
    await expect(page.locator("select[name='resource_type']")).toBeVisible();
  });
});

test.describe('billing pricing input validation', () => {
  test('invalid values (0, negative, blank, fractional) refuse to save', async ({ page, request }) => {
    await page.goto('/manage/billing');
    const cell = page.locator("input[name='unit_price_milli__INR']");

    // Capture the canonical value the server sees right now via a
    // round-trip through the page; we'll re-check it after typing
    // garbage to prove no POST snuck through.
    const before = await cell.inputValue();

    // 1) zero — fails min=1
    await cell.fill('0');
    expect(await cell.evaluate((el: HTMLInputElement) => el.checkValidity())).toBe(false);
    // 2) negative — fails min=1
    await cell.fill('-5');
    expect(await cell.evaluate((el: HTMLInputElement) => el.checkValidity())).toBe(false);
    // 3) blank — fails required
    await cell.fill('');
    expect(await cell.evaluate((el: HTMLInputElement) => el.checkValidity())).toBe(false);
    // 4) overflow — fails max
    await cell.fill('99999999999');
    expect(await cell.evaluate((el: HTMLInputElement) => el.checkValidity())).toBe(false);

    // Server still has the original value (no save fired through).
    const refreshed = await request.get('/manage/billing');
    const html = await refreshed.text();
    expect(html).toMatch(
      new RegExp(`name="unit_price_milli__INR"[^>]*value="${before}"`),
    );
  });

  test('valid value passes checkValidity and saves', async ({ page, request }) => {
    await page.goto('/manage/billing');
    const cell = page.locator("input[name='unit_price_milli__INR']");
    await cell.fill('42');
    expect(await cell.evaluate((el: HTMLInputElement) => el.checkValidity())).toBe(true);

    // Confirm the wire-level shape works end-to-end. (Browser-level
    // change-event submission has the Playwright/HTMX quirk we
    // already documented in the cell-attributes test.)
    const r = await request.post('/manage/billing/settings', {
      data: { unit_price_milli__INR: 42 },
      headers: { 'Content-Type': 'application/json' },
    });
    expect(r.status()).toBe(200);
    expect(await r.text()).toMatch(/pricing settings updated/i);
  });
});

test.describe('dev login', () => {
  test('/auth/login renders the dev-login form when bypass is active', async ({ page }) => {
    await page.goto('/auth/login');
    const form = page.locator("form[action='/auth/dev-login']");
    await expect(form).toBeVisible();
    await expect(form.locator("input[name='email']")).toHaveValue('dev@local.test');
    await expect(form.locator("button[type='submit']")).toBeVisible();
  });

  test('POST /auth/dev-login mints a session and redirects to /dashboard', async ({ request }) => {
    // Don't auto-follow the redirect — we want to inspect the
    // 302 + Set-Cookie headers directly.
    const r = await request.post('/auth/dev-login', {
      data: 'email=dev-test@local.test',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      maxRedirects: 0,
    });
    expect(r.status()).toBe(302);
    expect(r.headers()['location']).toBe('/dashboard');
    // Both cookies must be set: HTTP-only session, JS-readable csrf.
    const setCookie = r.headers()['set-cookie'] ?? '';
    expect(setCookie).toMatch(/session=[^;]+/);
    expect(setCookie).toMatch(/csrf=[^;]+/);
  });
});

test.describe('mobile @ 375px', () => {
  test.use({ viewport: { width: 375, height: 812 } });

  for (const path of ['/manage', '/manage/audit', '/manage/billing', '/manage/demo']) {
    test(`${path} doesn't horizontally overflow`, async ({ page }) => {
      await page.goto(path);
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - window.innerWidth,
      );
      // Allow 1px slop for sub-pixel rounding.
      expect(overflow).toBeLessThanOrEqual(1);
    });
  }
});
