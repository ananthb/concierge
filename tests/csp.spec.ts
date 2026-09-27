import { test, expect } from './_helpers/fixtures';

/**
 * Strict-CSP regression net.
 *
 * The policy has no nonce. It used to: the Worker generated one per response
 * and stamped it into both the header and every inline `<script>`. Once the
 * marketing routes became prerendered static files served off Cloudflare's
 * asset handler, that stopped being possible — nothing can inject a
 * per-response value into a static file — so initialisation moved to
 * `/boot.js` and the policy became plain `script-src 'self' …`.
 *
 * That is *stricter*, not looser: `'self'` admits only our own files, where
 * `'self' 'nonce-…'` also admitted whatever inline block carried the right
 * nonce. The invariant these tests hold is therefore the stronger one —
 * **there is no inline script anywhere** — plus the thing that could silently
 * break it: the static pages and the Worker-rendered pages must state the same
 * policy, because they get it from two different places
 * (`public/_headers` and `add_security_headers` in src/lib.rs).
 */

/** Worker-rendered: these routes are not prerendered, so the Worker answers. */
const WORKER_PAGES = ['/wizard', '/dashboard'];

/** Prerendered static files, served without the Worker running. */
const STATIC_PAGES = ['/', '/pricing', '/features', '/terms', '/privacy'];

const ALL_PAGES = [...STATIC_PAGES, ...WORKER_PAGES];

function directive(csp: string, name: string): string {
  const found = csp
    .split(';')
    .map((d) => d.trim())
    .find((d) => d.startsWith(name));
  expect(found, `CSP is missing a ${name} directive`).toBeTruthy();
  return found!;
}

for (const path of ALL_PAGES) {
  test(`${path} — sends a Content-Security-Policy at all`, async ({ request }) => {
    // The trap this guards: a prerendered page is served by the asset handler
    // and the Worker never runs, so it gets no headers from
    // `add_security_headers`. Without `public/_headers` the only pages an
    // anonymous visitor ever sees would be the least hardened on the deploy.
    const resp = await request.get(path);
    expect(resp.status(), `${path} should be reachable`).toBeLessThan(400);
    expect(resp.headers()['content-security-policy'], `${path} has no CSP`).toBeTruthy();
  });

  test(`${path} — script-src allows no inline, eval, or wildcard`, async ({ request }) => {
    const csp = (await request.get(path)).headers()['content-security-policy'];
    const scriptSrc = directive(csp, 'script-src');

    expect(scriptSrc, "must not allow 'unsafe-inline'").not.toMatch(/'unsafe-inline'/);
    // Alpine.js needed `new Function()` to evaluate its `x-show="a === b"`
    // attributes, so the policy carried 'unsafe-eval' for its sake. Elm
    // evaluates nothing at runtime, so it's gone — and must stay gone.
    expect(scriptSrc, "must not allow 'unsafe-eval'").not.toMatch(/'unsafe-eval'/);
    // A nonce would mean some inline script is being admitted somewhere.
    expect(scriptSrc, 'should no longer carry a nonce').not.toMatch(/'nonce-/);
    expect(scriptSrc, 'must not wildcard').not.toMatch(/\s\*/);
    expect(scriptSrc, "must allow 'self'").toMatch(/'self'/);
  });

  test(`${path} — carries no inline script or style`, async ({ request }) => {
    const body = await (await request.get(path)).text();

    // Inline = a <script> with no src. This is the invariant that makes a
    // nonce-free policy work: one inline block anywhere and every page would
    // need 'unsafe-inline' or a nonce again.
    const scriptTags = body.match(/<script\b[^>]*>/gi) ?? [];
    for (const tag of scriptTags) {
      expect(tag, `inline <script> found: ${tag}`).toMatch(/\bsrc=/);
    }

    const styleTags = body.match(/<style\b[^>]*>/gi) ?? [];
    expect(styleTags, `inline <style> found on ${path}`).toEqual([]);
  });

  test(`${path} — sends the other hardening headers`, async ({ request }) => {
    const h = (await request.get(path)).headers();
    expect(h['x-frame-options']).toBe('DENY');
    expect(h['x-content-type-options']).toBe('nosniff');
    expect(h['referrer-policy']).toBe('strict-origin-when-cross-origin');
  });

  test(`${path} — no CSP violations at runtime`, async ({ page, consoleErrors }) => {
    await page.goto(path);
    // Wait for Elm to render rather than sleeping: a violation that blocked
    // the bundle would otherwise pass as "no violations yet".
    await page.locator('main.site-main').waitFor();
    const cspViolations = consoleErrors.filter((e) => e.startsWith('csp:'));
    expect(cspViolations).toEqual([]);
  });
}

test('the static and Worker policies agree', async ({ request }) => {
  // These come from two places — `public/_headers` for the prerendered pages,
  // `add_security_headers` in src/lib.rs for the rest — so they can drift
  // silently. Comparing the directives that matter catches that; the two are
  // allowed to differ only in the dev-only localhost allowances on
  // form-action and img-src.
  const staticCsp = (await request.get('/terms')).headers()['content-security-policy'];
  const workerCsp = (await request.get('/wizard')).headers()['content-security-policy'];

  for (const name of ['default-src', 'script-src', 'style-src', 'base-uri', 'object-src']) {
    expect(directive(staticCsp, name), `${name} differs between static and Worker pages`).toBe(
      directive(workerCsp, name),
    );
  }
});
