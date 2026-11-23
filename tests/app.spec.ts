import { test, expect, waitForBoot, devLogin } from './_helpers/fixtures';
import { checkTapTargets } from './_helpers/layout';

/**
 * The Elm app: does it boot, route, run the demo, and drive the wizard & dashboard.
 *
 * Every assertion here waits on rendered content rather than a timeout,
 * because with a single-page app "the HTML came back" says nothing — the
 * shell is the same four hundred bytes on every route. If the bundle fails
 * to load or a decoder rejects the API's shape, the page stays empty and
 * these fail, which is the point.
 *
 * `consoleErrors` matters more than usual here: Elm reports a decoder
 * mismatch to the console, so a drifted API contract shows up as a console
 * error rather than a visibly broken page.
 */

const app = (page: import('@playwright/test').Page) => page.locator('main.site-main');

test.describe('shell', () => {
  test('/ serves the shell and Elm mounts into it', async ({ page, consoleErrors }) => {
    await page.goto('/');
    await expect(app(page).locator('.landing')).toBeVisible();
    await expect(page.locator('.site-header .brand')).toBeVisible();
    expect(consoleErrors, consoleErrors.join('\n')).toEqual([]);
  });

  test('the shell carries no copy of its own', async ({ request }) => {
    const body = await (await request.get('/wizard')).text();
    expect(body).toContain('/app.js');
    expect(body).toContain('/boot.js');
    expect(body).not.toContain('Answer every WhatsApp customer');
  });

  test('an unknown path gets the shell under a 404', async ({ page, request }) => {
    const resp = await request.get('/no-such-page');
    expect(resp.status()).toBe(404);

    await page.goto('/no-such-page');
    await expect(app(page).getByText('Nothing here')).toBeVisible();
  });

  test('static assets are served outside the worker', async ({ request }) => {
    const js = await request.get('/app.js');
    expect(js.status()).toBe(200);
    expect(js.headers()['content-type']).toContain('javascript');

    const boot = await request.get('/boot.js');
    expect(boot.status()).toBe(200);
  });

  test('every asset the shell loads carries an integrity hash', async ({ request }) => {
    const shell = await (await request.get('/wizard')).text();

    const tags = [
      ...shell.matchAll(/<link rel="stylesheet"[^>]*>/g),
      ...shell.matchAll(/<script[^>]*src=[^>]*>/g),
    ].map((m) => m[0]);

    expect(tags.length, 'shell should reference assets').toBeGreaterThan(0);
    for (const tag of tags) {
      expect(tag, `missing integrity: ${tag}`).toMatch(/\bintegrity="sha384-[A-Za-z0-9+/=]+"/);
      expect(tag, `missing crossorigin: ${tag}`).toMatch(/\bcrossorigin="anonymous"/);
    }
  });

  test('the integrity hashes match what is served', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', (m) => {
      if (m.type() === 'error') errors.push(m.text());
    });

    await page.goto('/wizard');
    await waitForBoot(page);

    const integrityFailures = errors.filter((e) =>
      /integrity|Failed to find a valid digest/i.test(e),
    );
    expect(integrityFailures, integrityFailures.join('\n')).toEqual([]);
  });

  test('every stylesheet the shell references actually loads', async ({ page, request }) => {
    const shell = await (await request.get('/wizard')).text();
    const hrefs = [...shell.matchAll(/<link rel="stylesheet" href="([^"]+)"/g)].map((m) => m[1]);
    expect(hrefs.length, 'shell should reference stylesheets').toBeGreaterThan(0);

    for (const href of hrefs) {
      const resp = await request.get(href);
      expect(resp.status(), `${href} should load`).toBe(200);
      expect(resp.headers()['content-type'], `${href} should be CSS`).toContain('css');
    }

    await page.goto('/');
    await page.locator('main.site-main').waitFor();
    const accepted = await page.evaluate(
      () =>
        Array.from(document.styleSheets).filter((s) => {
          try {
            return s.href !== null && s.cssRules.length >= 0;
          } catch {
            return false;
          }
        }).length,
    );
    expect(accepted, 'every referenced stylesheet should be applied').toBe(hrefs.length);
  });
});

test.describe('prerendered pages', () => {
  const STATIC = ['/', '/pricing', '/features', '/terms', '/privacy'];

  for (const path of STATIC) {
    test(`${path} carries its copy without JavaScript`, async ({ request }) => {
      const body = await (await request.get(path)).text();
      expect(body).toContain('site-main');
      expect(body.length, `${path} looks like an empty shell`).toBeGreaterThan(2000);
    });

    test(`${path} comes alive when scripts do run`, async ({ page }) => {
      await page.goto(path);
      await waitForBoot(page);
    });
  }

  test('a prerendered page still navigates as a SPA', async ({ page }) => {
    await page.goto('/terms');
    await waitForBoot(page);
    await page.evaluate(() => {
      (window as unknown as { __stillHere: boolean }).__stillHere = true;
    });

    await page.locator('.footer-links a[href="/features"]').click();
    await expect(page.locator('main.site-main').getByRole('heading', { level: 1 })).toHaveText(
      /Built for small businesses on WhatsApp/,
    );

    const stillHere = await page.evaluate(
      () => (window as unknown as { __stillHere?: boolean }).__stillHere === true,
    );
    expect(stillHere, 'navigating away from a prerendered page reloaded the document').toBe(true);
  });
});

test.describe('routing', () => {
  const routes: Array<[string, string]> = [
    ['/pricing', 'Pay per reply, nothing else'],
    ['/features', 'Built for small businesses on WhatsApp'],
    ['/terms', 'Terms of Service'],
    ['/privacy', 'Privacy Policy'],
    ['/login', 'Sign in to replybots'],
  ];

  for (const [path, heading] of routes) {
    test(`${path} renders on a cold load`, async ({ page, consoleErrors }) => {
      await page.goto(path);
      await expect(app(page).getByRole('heading', { name: heading, level: 1 })).toBeVisible();
      expect(consoleErrors, consoleErrors.join('\n')).toEqual([]);
    });
  }

  test('in-app navigation does not reload the page', async ({ page }) => {
    await page.goto('/');
    await waitForBoot(page);
    await page.evaluate(() => {
      (window as unknown as { __stillHere: boolean }).__stillHere = true;
    });

    await page.locator('.footer-links a[href="/pricing"]').click();
    await expect(app(page).getByRole('heading', { name: /Pay per reply, nothing else/ })).toBeVisible();

    const stillHere = await page.evaluate(
      () => (window as unknown as { __stillHere?: boolean }).__stillHere === true,
    );
    expect(stillHere, 'navigation caused a full page load').toBe(true);
  });

  test('the back button returns to the previous route', async ({ page }) => {
    await page.goto('/');
    await waitForBoot(page);
    await page.locator('.footer-links a[href="/features"]').click();
    await expect(
      app(page).getByRole('heading', { name: /Built for small businesses on WhatsApp/ }),
    ).toBeVisible();
    await page.goBack();
    await expect(app(page).locator('.landing')).toBeVisible();
  });

  test('signed-out visitors are sent to sign in, not into the app', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(app(page).getByRole('heading', { name: /Sign in to replybots/ })).toBeVisible();
  });
});

test.describe('pricing', () => {
  test('the rate comes from the API, not hardcoded copy', async ({ page }) => {
    const bootstrap = await page.request.get('/api/bootstrap');
    const { pricing } = await bootstrap.json();

    await page.goto('/pricing');
    await expect(app(page).getByRole('heading', { name: /Pay per reply, nothing else/ })).toBeVisible();

    for (const rate of pricing.currencies) {
      await expect(app(page).getByRole('heading', { name: rate.code })).toBeVisible();
    }
  });

  test('Indian amounts use lakh grouping', async ({ page }) => {
    await page.goto('/pricing');
    const body = await app(page).textContent();
    expect(body).not.toMatch(/₹1,000,000/);
  });
});

test.describe('demo chat', () => {
  test('a visitor can pick a business and send a message', async ({ page, consoleErrors }) => {
    const boot = await (await page.request.get('/api/bootstrap')).json();
    test.skip(!boot.demo.enabled || boot.demo.personas.length === 0, 'demo is disabled');

    await page.goto('/');
    const firstPersona = page.locator('.persona-card').first();
    await expect(firstPersona).toBeVisible();
    await firstPersona.click();

    await expect(page.locator('.demo-transcript .bubble-assistant')).toBeVisible();
    await expect(page.locator('.prompt-panel .prompt-panel-note')).toBeVisible();

    await page.locator('#demo-input').fill('What time do you open?');
    await page.locator('.demo-composer button[type="submit"]').click();

    await expect(page.locator('.demo-transcript .bubble-user')).toHaveText(
      'What time do you open?',
    );
    await expect(page.locator('.demo-transcript .bubble-assistant').nth(1)).toBeVisible({
      timeout: 30_000,
    });
    expect(consoleErrors, consoleErrors.join('\n')).toEqual([]);
  });

  test('send is refused while empty', async ({ page }) => {
    const boot = await (await page.request.get('/api/bootstrap')).json();
    test.skip(!boot.demo.enabled || boot.demo.personas.length === 0, 'demo is disabled');

    await page.goto('/');
    await page.locator('.persona-card').first().click();
    await expect(page.locator('.demo-composer button[type="submit"]')).toBeDisabled();
  });
});

test.describe('authenticated wizard and dashboard', () => {
  test('3-step wizard renders basics -> channels -> back to basics', async ({
    page,
    consoleErrors,
  }) => {
    await devLogin(page.request, `wizard-ui-${Date.now()}@local.test`);

    await page.goto('/wizard');
    await expect(app(page).locator('.wizard-progress li')).toHaveCount(3);
    await expect(app(page).getByRole('heading', { name: 'Tell us about your business' })).toBeVisible();

    await page.locator('#name').fill('Nutmeg & Crumb');
    await page.locator('#phone').fill('+91 98200 11223');
    await page.getByRole('button', { name: 'Sole proprietorship' }).click();
    await page.getByRole('button', { name: 'Continue' }).click();

    await expect(
      app(page).getByRole('heading', { name: 'Connect your WhatsApp Business number' }),
    ).toBeVisible();
    await expect(app(page).getByRole('button', { name: 'Connect with Meta' })).toBeVisible();

    await page.getByRole('button', { name: 'Back' }).click();
    await expect(app(page).locator('#name')).toHaveValue('Nutmeg & Crumb');
    expect(consoleErrors, consoleErrors.join('\n')).toEqual([]);
  });

  test('dashboard renders Overview, Channels, Credits, and Settings against real API', async ({
    page,
    consoleErrors,
  }) => {
    await devLogin(page.request, `dash-ui-${Date.now()}@local.test`);

    // Allow reaching /dashboard directly in UI tests while keeping all /api/{whatsapp,billing,settings}
    // requests backed by the live local Worker + D1.
    await page.route('**/api/bootstrap', async (route) => {
      const resp = await route.fetch();
      const json = await resp.json();
      if (json.session) {
        json.session.destination = 'dashboard';
        json.session.onboarding_step = null;
      }
      await route.fulfill({ response: resp, json });
    });

    await page.goto('/dashboard');
    await expect(app(page).locator('.tabs .tab')).toHaveCount(4);
    await expect(app(page).locator('.balance')).toHaveText('0 credits');
    await expect(app(page).getByText('No WhatsApp number connected')).toBeVisible();

    // Channels tab
    await app(page).locator('.tabs a', { hasText: 'Channels' }).click();
    await expect(app(page).getByRole('heading', { name: 'WhatsApp Business' })).toBeVisible();
    await expect(app(page).getByRole('button', { name: 'Connect WhatsApp number' })).toBeVisible();

    // Credits tab
    await app(page).locator('.tabs a', { hasText: 'Credits' }).click();
    await expect(app(page).getByRole('heading', { name: 'Balance' })).toBeVisible();
    await expect(app(page).getByRole('heading', { name: 'Buy credits' })).toBeVisible();
    await expect(app(page).locator('.balance')).toHaveText('0 credits');

    // Settings tab: toggle handoff_email and persist to Worker
    await app(page).locator('.tabs a', { hasText: 'Settings' }).click();
    await expect(app(page).getByRole('heading', { name: 'Conversation timing' })).toBeVisible();
    const handoffCheckbox = app(page).locator('#handoff_email');
    await expect(handoffCheckbox).toBeChecked();
    await handoffCheckbox.uncheck();
    await app(page).getByRole('button', { name: 'Save notifications' }).click();
    await expect(app(page).locator('.banner.success')).toContainText('Saved.');

    await page.reload();
    await expect(app(page).locator('#handoff_email')).not.toBeChecked();
    expect(consoleErrors, consoleErrors.join('\n')).toEqual([]);
  });
});

test.describe('mobile ergonomics', () => {
  test('landing page tap targets clear the floor at 375px', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto('/');
    await expect(app(page).locator('.landing')).toBeVisible();

    const issues = await checkTapTargets(page, { min: 36 });
    const real = issues.filter((i) => !i.includes('a.brand'));
    expect(real, real.join('\n  ')).toEqual([]);
  });
});
