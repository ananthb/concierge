import { test, expect } from './_helpers/fixtures';

/**
 * Strict-CSP regression net.
 *
 * The fetch wrapper generates a fresh nonce per HTML response and stamps
 * it into both the CSP header and every `nonce="…"` placeholder in the
 * rendered body. These tests guard the invariants that make the strict
 * `script-src 'nonce-…'` (no `'unsafe-inline'`) policy actually work —
 * if a future template ships an inline `<script>` without a nonce, or
 * the wrapper stops swapping placeholders, CI fails.
 */

const PAGES = ['/', '/login', '/features', '/pricing'];

const NONCE_RE = /nonce-([A-Za-z0-9+/=_-]+)/;

for (const path of PAGES) {
  test(`${path} — CSP nonce in header matches every body nonce`, async ({ request }) => {
    const resp = await request.get(path);
    expect(resp.status()).toBe(200);
    const csp = resp.headers()['content-security-policy'];
    expect(csp, 'CSP header missing').toBeTruthy();

    const headerMatch = csp.match(NONCE_RE);
    expect(headerMatch, 'header has no nonce-… token').not.toBeNull();
    const headerNonce = headerMatch![1];

    const body = await resp.text();
    const bodyNonces = Array.from(body.matchAll(/nonce="([^"]+)"/g)).map((m) => m[1]);
    expect(bodyNonces.length, 'body has at least one nonce attribute').toBeGreaterThan(0);
    for (const n of bodyNonces) expect(n).toBe(headerNonce);

    // Placeholder must always be replaced — leaking it would reveal
    // every nonced tag is fixed, defeating the whole scheme.
    expect(body, 'placeholder leaked into body').not.toContain('__CSP_NONCE__');
  });

  test(`${path} — strict CSP: no 'unsafe-inline' in script-src`, async ({ request }) => {
    const csp = (await request.get(path)).headers()['content-security-policy'];
    const scriptSrc = csp.split(';').map((d) => d.trim()).find((d) => d.startsWith('script-src'));
    expect(scriptSrc).toBeTruthy();
    expect(scriptSrc, 'script-src must not allow unsafe-inline').not.toMatch(/'unsafe-inline'/);
    // Alpine.js needed `new Function()` to evaluate its `x-show="a === b"`
    // attributes, so the policy carried 'unsafe-eval' for its sake. Elm
    // evaluates nothing at runtime, so it's gone — and must stay gone.
    expect(scriptSrc, "script-src must not allow unsafe-eval").not.toMatch(/'unsafe-eval'/);
  });

  test(`${path} — every inline <script>/<style> carries a nonce`, async ({ request }) => {
    const body = await (await request.get(path)).text();
    // Inline = no `src=` for scripts. Self-closing/external tags are skipped.
    const scriptOpenTags = body.match(/<script\b[^>]*>(?!\s*<\/script>)/gi) ?? [];
    for (const tag of scriptOpenTags) {
      if (/\bsrc=/.test(tag)) continue; // external scripts allow-listed by host
      expect(tag, `inline <script> missing nonce: ${tag}`).toMatch(/\bnonce="/);
    }
    const styleOpenTags = body.match(/<style\b[^>]*>/gi) ?? [];
    for (const tag of styleOpenTags) {
      expect(tag, `inline <style> missing nonce: ${tag}`).toMatch(/\bnonce="/);
    }
  });

  test(`${path} — no CSP violations at runtime`, async ({ page, consoleErrors }) => {
    await page.goto(path);
    // Wait for Elm to mount rather than sleeping: a violation that blocked
    // the bundle would otherwise pass as "no violations yet".
    await page.locator('#app').locator('*').first().waitFor();
    const cspViolations = consoleErrors.filter((e) => e.startsWith('csp:'));
    expect(cspViolations).toEqual([]);
  });
}
