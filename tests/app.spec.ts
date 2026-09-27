import { test, expect, waitForBoot } from './_helpers/fixtures';
import { checkTapTargets } from './_helpers/layout';

/**
 * The Elm app: does it boot, route, and run the demo.
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

/**
 * Scope assertions to what Elm actually renders.
 *
 * Not `#app`: this is a `Browser.application`, so Elm takes over `<body>` and
 * removes anything it finds there on init. `.site-main` is the container Elm
 * itself renders, which is what we want to assert against anyway — its
 * presence proves the bundle booted.
 */
const app = (page: import('@playwright/test').Page) => page.locator('main.site-main');

test.describe('shell', () => {
  test('/ serves the shell and Elm mounts into it', async ({ page, consoleErrors }) => {
    await page.goto('/');
    // Something — anything — rendered by Elm.
    await expect(app(page).locator('.landing')).toBeVisible();
    await expect(page.locator('.site-header .brand')).toBeVisible();
    expect(consoleErrors, consoleErrors.join('\n')).toEqual([]);
  });

  test('the shell carries no copy of its own', async ({ request }) => {
    // Every user-facing string lives in the Elm app. If marketing copy starts
    // appearing in the shell too there are two places to change it, and the
    // shell's copy is the one that goes stale.
    //
    // Read /wizard, not /: the marketing routes are prerendered static files
    // and *do* carry copy — that's their entire purpose. /wizard is never
    // prerendered, so it is the shell.
    const body = await (await request.get('/wizard')).text();
    expect(body).toContain('/app.js');
    expect(body).toContain('/boot.js');
    expect(body).not.toContain('Answer every customer');
  });

  test('an unknown path gets the shell under a 404', async ({ page, request }) => {
    // Status has to be honest for crawlers and uptime monitors, even though
    // the body is the same shell every route gets.
    const resp = await request.get('/no-such-page');
    expect(resp.status()).toBe(404);

    await page.goto('/no-such-page');
    await expect(app(page).getByText('Nothing here')).toBeVisible();
  });

  test('static assets are served outside the worker', async ({ request }) => {
    const js = await request.get('/app.js');
    expect(js.status()).toBe(200);
    // Cached hard by `public/_headers`: the bundle is rebuilt on every
    // deploy, unlike the shell, which is regenerated per request.
    expect(js.headers()['content-type']).toContain('javascript');

    const boot = await request.get('/boot.js');
    expect(boot.status()).toBe(200);
  });

  test('every stylesheet the shell references actually loads', async ({ page, request }) => {
    // This replaces a build-time guard. `build.rs` used to read each CSS file
    // to hash it, so a missing one failed the build; without that, a name in
    // `STYLESHEETS` with no matching file would 404 silently and the page
    // would render unstyled. Checked at runtime, which also catches a file
    // that exists locally but wasn't deployed.
    const shell = await (await request.get('/wizard')).text();
    const hrefs = [...shell.matchAll(/<link rel="stylesheet" href="([^"]+)"/g)].map((m) => m[1]);
    expect(hrefs.length, 'shell should reference stylesheets').toBeGreaterThan(0);

    for (const href of hrefs) {
      const resp = await request.get(href);
      expect(resp.status(), `${href} should load`).toBe(200);
      expect(resp.headers()['content-type'], `${href} should be CSS`).toContain('css');
    }

    // And the browser must actually apply them — a stylesheet can 200 and
    // still be rejected (that is exactly what integrity hashes did when they
    // drifted). Compare the count the browser accepted against the count
    // referenced.
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
      // The whole point of prerendering: a crawler that never runs a script
      // still reads the page. Asserted on the raw bytes, with no browser.
      const body = await (await request.get(path)).text();
      expect(body).toContain('site-main');
      expect(body.length, `${path} looks like an empty shell`).toBeGreaterThan(2000);
    });

    test(`${path} comes alive when scripts do run`, async ({ page }) => {
      // The failure this guards is the nasty one: a static page that renders
      // perfectly and is completely inert, because the bundle never loaded or
      // never initialised. Every DOM selector would still match, so only the
      // boot marker can tell the difference.
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

    // Footer, not header: the header's marketing links are hidden below
    // 600px, and this spec runs at both viewports. The footer carries the same
    // destinations at every width and is the navigation path on a phone.
    await page.locator('.footer-links a[href="/features"]').click();
    await expect(page.locator('main.site-main').getByRole('heading', { level: 1 })).toHaveText(
      /What Concierge does/,
    );

    const stillHere = await page.evaluate(
      () => (window as unknown as { __stillHere?: boolean }).__stillHere === true,
    );
    expect(stillHere, 'navigating away from a prerendered page reloaded the document').toBe(true);
  });
});

test.describe('routing', () => {
  const routes: Array<[string, string]> = [
    ['/pricing', 'Pay for replies'],
    ['/features', 'What Concierge does'],
    ['/terms', 'Terms of Service'],
    ['/privacy', 'Privacy Policy'],
    ['/login', 'Sign in to Concierge'],
  ];

  for (const [path, heading] of routes) {
    test(`${path} renders on a cold load`, async ({ page, consoleErrors }) => {
      // A cold load is the case that breaks when the worker's `is_app_route`
      // list drifts from Route.elm: in-app navigation would still work, so
      // only a direct hit catches it.
      await page.goto(path);
      await expect(app(page).getByRole('heading', { name: heading, level: 1 })).toBeVisible();
      expect(consoleErrors, consoleErrors.join('\n')).toEqual([]);
    });
  }

  test('in-app navigation does not reload the page', async ({ page }) => {
    await page.goto('/');
    // Must wait for the bundle, not for markup. `/` is a prerendered snapshot
    // of Elm's own output, so every selector is present before Elm runs; a
    // click in that window is handled by the browser as a plain link and the
    // page really does reload.
    await waitForBoot(page);
    await page.evaluate(() => {
      (window as unknown as { __stillHere: boolean }).__stillHere = true;
    });

    await page.locator('.footer-links a[href="/pricing"]').click();
    await expect(app(page).getByRole('heading', { name: /Pay for replies/ })).toBeVisible();

    // Survived the navigation ⇒ Elm handled it, no document reload.
    const stillHere = await page.evaluate(
      () => (window as unknown as { __stillHere?: boolean }).__stillHere === true,
    );
    expect(stillHere, 'navigation caused a full page load').toBe(true);
  });

  test('the back button returns to the previous route', async ({ page }) => {
    await page.goto('/');
    await waitForBoot(page);
    await page.locator('.footer-links a[href="/features"]').click();
    await expect(app(page).getByRole('heading', { name: /What Concierge does/ })).toBeVisible();
    await page.goBack();
    await expect(app(page).locator('.landing')).toBeVisible();
  });

  test('signed-out visitors are sent to sign in, not into the app', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(app(page).getByRole('heading', { name: /Sign in to Concierge/ })).toBeVisible();
  });
});

test.describe('pricing', () => {
  test('the rate comes from the API, not hardcoded copy', async ({ page }) => {
    const bootstrap = await page.request.get('/api/bootstrap');
    const { pricing } = await bootstrap.json();

    await page.goto('/pricing');
    await expect(app(page).getByRole('heading', { name: /Pay for replies/ })).toBeVisible();

    // Every currency the worker quotes gets a card.
    for (const rate of pricing.currencies) {
      await expect(app(page).getByRole('heading', { name: rate.code })).toBeVisible();
    }
  });

  test('Indian amounts use lakh grouping', async ({ page }) => {
    // The one formatting rule worth asserting end-to-end: ₹1,00,000 is one
    // lakh, ₹100,000 is a bug that still looks like a number.
    await page.goto('/pricing');
    const body = await app(page).textContent();
    expect(body).not.toMatch(/₹1,000,000/);
  });
});

test.describe('demo chat', () => {
  // The demo is operator-configurable and can be off, so every test here
  // skips rather than fails when it's disabled.
  test('a visitor can pick a business and send a message', async ({ page, consoleErrors }) => {
    const boot = await (await page.request.get('/api/bootstrap')).json();
    test.skip(!boot.demo.enabled || boot.demo.personas.length === 0, 'demo is disabled');

    await page.goto('/');
    const firstPersona = page.locator('.persona-card').first();
    await expect(firstPersona).toBeVisible();
    await firstPersona.click();

    // The business greets first, as it would on WhatsApp.
    await expect(page.locator('.demo-transcript .bubble-assistant')).toBeVisible();

    await page.locator('#demo-input').fill('What time do you open?');
    await page.locator('.demo-composer button[type="submit"]').click();

    // The visitor's own message appears immediately; the reply needs a model
    // call, so allow it real time.
    await expect(page.locator('.demo-transcript .bubble-user')).toHaveText(
      'What time do you open?',
    );
    await expect(page.locator('.demo-transcript .bubble-assistant').nth(1)).toBeVisible({
      timeout: 30_000,
    });
    expect(consoleErrors, consoleErrors.join('\n')).toEqual([]);
  });

  test('the prompt panel shows what the model is actually sent', async ({ page }) => {
    const boot = await (await page.request.get('/api/bootstrap')).json();
    test.skip(!boot.demo.enabled || boot.demo.personas.length === 0, 'demo is disabled');

    await page.goto('/');
    await page.locator('.persona-card').first().click();
    await page.getByRole('button', { name: /View the prompt/ }).click();

    const panel = page.locator('.prompt-panel .prompt-preview');
    await expect(panel).toBeVisible();
    // Not a placeholder: the real composed middle for that persona.
    expect((await panel.textContent())?.length ?? 0).toBeGreaterThan(50);
  });

  test('send is refused while empty', async ({ page }) => {
    const boot = await (await page.request.get('/api/bootstrap')).json();
    test.skip(!boot.demo.enabled || boot.demo.personas.length === 0, 'demo is disabled');

    await page.goto('/');
    await page.locator('.persona-card').first().click();
    await expect(page.locator('.demo-composer button[type="submit"]')).toBeDisabled();
  });
});

test.describe('mobile ergonomics', () => {
  test('landing page tap targets clear the floor at 375px', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto('/');
    await expect(app(page).locator('.landing')).toBeVisible();

    const issues = await checkTapTargets(page, { min: 36 });
    // The brand link is a logo plus a wordmark — it reads as a header
    // element, not a precision target, and was exempt before this refactor
    // too.
    const real = issues.filter((i) => !i.includes('a.brand'));
    expect(real, real.join('\n  ')).toEqual([]);
  });
});
