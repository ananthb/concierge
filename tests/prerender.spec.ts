import { test, expect } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

/**
 * Snapshot the marketing routes to static HTML in `public/`.
 *
 * **Why.** The frontend is an Elm SPA, so a crawler that doesn't run
 * JavaScript sees an empty `<body>` with two script tags — every marketing
 * route was invisible to them.
 *
 * **How.** Boot the real app in a real browser against the real Worker, wait
 * for the route to render, and write `document.documentElement.outerHTML` to a
 * file Cloudflare's asset handler will serve. A request for `/pricing` then
 * gets `public/pricing.html` straight off the edge and the Worker is never
 * invoked, so these routes cost no wasm cold start at all.
 *
 * Elm does not hydrate: `Browser.application` replaces the body on init. The
 * snapshot is therefore first paint and crawler content, and a moment later
 * Elm renders its own markup over it. The two match because the snapshot *is*
 * Elm's output.
 *
 * **Numbers are deliberately not captured.** `/api/bootstrap` is refused, so
 * each page renders its copy with data-dependent parts in the not-loaded state
 * it already handles. Baking the seeded ₹0.10 into a static file would go
 * stale the moment pricing changed through the operator API — and a staleness
 * check couldn't catch it, because it compares against the same seeded
 * database. Crawlers get the prose, which is the indexable part; the rate only
 * ever comes from the live API.
 *
 * **Staying honest.** Cloudflare Builds has no browser, so this can't run at
 * deploy time; the output is committed. CI runs this project and then fails on
 * `git diff -- public/`, so editing Elm copy without regenerating breaks the
 * build instead of quietly serving stale HTML.
 *
 * Only runs in the `prerender` project (see playwright.config.ts), invoked by
 * `npm run prerender`.
 */

const OUT = join(process.cwd(), 'public');

type Route = { path: string; file: string; ready: string };

const ROUTES: Route[] = [
  { path: '/', file: 'index.html', ready: '.landing .hero h1' },
  { path: '/pricing', file: 'pricing.html', ready: '.page-pricing h1' },
  { path: '/features', file: 'features.html', ready: '.page-features h1' },
  { path: '/terms', file: 'terms.html', ready: '.page-legal h1' },
  { path: '/privacy', file: 'privacy.html', ready: '.page-legal h1' },
];

/**
 * The Worker's shell, fetched from a route the asset handler can never
 * shadow.
 *
 * This is what breaks the circularity. Once `public/pricing.html` exists, a
 * request for `/pricing` is answered by that file and the Worker never runs —
 * so a naive re-run would snapshot its own previous output and any drift in
 * `<head>` would compound. `/wizard` is never prerendered, so it always
 * returns a freshly generated shell, and we serve that for the navigation
 * while leaving the URL alone so Elm's router sees the real path.
 */
async function workerShell(request: import('@playwright/test').APIRequestContext) {
  const resp = await request.get('/wizard');
  expect(resp.status(), 'the Worker should serve a shell at /wizard').toBe(200);
  const html = await resp.text();
  expect(html, 'shell should load the bundle').toContain('/app.js');
  expect(html, 'shell should load the initialiser').toContain('/boot.js');
  return html;
}

test.beforeAll(async () => {
  await mkdir(OUT, { recursive: true });
});

for (const route of ROUTES) {
  test(`prerender ${route.path}`, async ({ page, request }) => {
    const shell = await workerShell(request);

    // Answer the document request with the Worker's shell, whatever may
    // already sit in public/ at this path.
    await page.route(
      (url) => url.pathname === route.path,
      async (r) => {
        if (r.request().resourceType() !== 'document') return r.continue();
        return r.fulfill({ status: 200, contentType: 'text/html; charset=utf-8', body: shell });
      },
    );

    // Refuse the payload rather than stubbing it: a stub would bake values in.
    await page.route('**/api/bootstrap', (r) => r.abort());

    await page.goto(route.path, { waitUntil: 'domcontentloaded' });

    // Proves the page actually rendered. Without this, a timing change would
    // start silently committing snapshots of an empty shell.
    await page.locator(route.ready).waitFor();

    const html = await page.evaluate(
      () => '<!doctype html>\n' + document.documentElement.outerHTML + '\n',
    );

    expect(html.length, `${route.path} snapshot is suspiciously small`).toBeGreaterThan(2000);
    // Elm takes over <body> on init, so the snapshot must still pull in the
    // bundle and the initialiser — otherwise the static page is inert and its
    // navigation dead.
    expect(html, 'snapshot must load the bundle').toContain('/app.js');
    expect(html, 'snapshot must load the initialiser').toContain('/boot.js');
    // The whole point: real copy, not a spinner.
    expect(html, 'snapshot should not be a loading state').not.toContain('class="spinner"');

    await writeFile(join(OUT, route.file), html);
  });
}
