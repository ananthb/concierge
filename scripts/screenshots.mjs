#!/usr/bin/env node
/**
 * Screenshot generator for the Concierge public site.
 *
 * Spawns `wrangler dev` (unless --serve is omitted, in which case it expects
 * a server already running on :8787), drives Chromium via Playwright, and
 * writes PNGs into doc/screenshots/. Used both for the docs gallery and as
 * a sanity check that recent template changes still render.
 *
 * Prereq: a Nix devShell (or `npx playwright install chromium`) so Chromium
 * is on PLAYWRIGHT_BROWSERS_PATH.
 *
 * Usage:
 *   node scripts/screenshots.mjs           # assumes wrangler dev is running
 *   node scripts/screenshots.mjs --serve   # spawn wrangler dev for the run
 */

import { chromium } from 'playwright';
import { spawn } from 'child_process';
import { setTimeout as sleep } from 'timers/promises';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { mkdir, writeFile, rm } from 'fs/promises';
import { tmpdir } from 'os';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = join(__dirname, '..');
const OUTPUT_DIR = join(PROJECT_ROOT, 'doc', 'screenshots');

const BASE_URL = 'http://localhost:8787';

const DESKTOP = { width: 1280, height: 800 };
const MOBILE = { width: 375, height: 812 };

// Public pages only. Anything behind /admin needs a session and isn't part
// of the docs gallery — capture those manually if/when we want them.
//
// Each public page is captured at both desktop and mobile widths so we
// can spot mobile regressions (the nav row historically overflowed on
// 375px). Mobile shots feed both the docs gallery and the layout-sanity
// check below.
const SHOTS = [
  { name: 'home.png', path: '/', viewport: DESKTOP },
  { name: 'home-mobile.png', path: '/', viewport: MOBILE },
  { name: 'login.png', path: '/auth/login', viewport: DESKTOP },
  { name: 'login-mobile.png', path: '/auth/login', viewport: MOBILE },
  { name: 'features.png', path: '/features', viewport: DESKTOP },
  { name: 'features-mobile.png', path: '/features', viewport: MOBILE },
  { name: 'pricing.png', path: '/pricing', viewport: DESKTOP },
  { name: 'pricing-mobile.png', path: '/pricing', viewport: MOBILE },
  { name: 'terms.png', path: '/terms', viewport: DESKTOP },
  { name: 'terms-mobile.png', path: '/terms', viewport: MOBILE },
  { name: 'privacy.png', path: '/privacy', viewport: DESKTOP },
  { name: 'privacy-mobile.png', path: '/privacy', viewport: MOBILE },
];

async function startWrangler(envFile) {
  console.log('Starting wrangler dev...');
  const proc = spawn('wrangler', ['dev', '--port', '8787', '--env-file', envFile], {
    cwd: PROJECT_ROOT,
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, FORCE_COLOR: '0' },
  });

  return new Promise((resolve, reject) => {
    let output = '';
    let resolved = false;

    const onData = (data) => {
      const chunk = data.toString();
      output += chunk;
      process.stdout.write(chunk.replace(/^/gm, '  [wrangler] '));
      // Wrangler v3/v4 prints "Ready on http://localhost:8787" once the
      // worker is bound. That's the green light.
      if (!resolved && /Ready on http/i.test(output)) {
        resolved = true;
        console.log('wrangler dev ready');
        resolve(proc);
      }
    };

    proc.stdout.on('data', onData);
    proc.stderr.on('data', onData);
    proc.on('error', (err) => { if (!resolved) reject(err); });
    proc.on('exit', (code) => {
      if (!resolved) reject(new Error(`wrangler exited (code=${code}) before ready:\n${output}`));
    });

    // First-time wasm builds + DO setup can take well over a minute.
    // Subsequent runs reuse the cargo target dir and finish in seconds.
    sleep(180_000).then(() => {
      if (!resolved) {
        proc.kill();
        reject(new Error('wrangler dev did not become ready within 180s'));
      }
    });
  });
}

/**
 * Layout sanity check: catch the kinds of breakage you can't see in a
 * cropped 375x812 screenshot. Three things are checked on every page:
 *
 *   1. Horizontal page overflow — `documentElement.scrollWidth` must
 *      not exceed the viewport. A wider doc means *something* is poking
 *      past the right edge and dragging in a horizontal scrollbar.
 *
 *   2. Culprit hunt — when (1) fires, walk every element and return
 *      the deepest leaf nodes whose right edge crosses the viewport.
 *      The deepest-leaf rule keeps us from blaming `<body>` for a
 *      stray button two layers down.
 *
 *   3. Header-specific check — even if the page itself fits, an
 *      individual nav button getting clipped by `overflow:hidden` is
 *      a regression worth flagging.
 *
 * Returns an array of human-readable problem strings; empty means OK.
 */
async function checkLayout(page, viewport) {
  return await page.evaluate(({ vw }) => {
    const issues = [];
    const docWidth = document.documentElement.scrollWidth;
    if (docWidth > vw + 1) {
      issues.push(`page overflows viewport: scrollWidth=${docWidth} viewport=${vw}`);
      // Walk the DOM, find leaf elements whose right edge crosses the
      // viewport. We only report leaves (no overflowing descendants)
      // because the parent's overflow is *caused* by the child.
      const culprits = [];
      const all = document.body.getElementsByTagName('*');
      for (const el of all) {
        const cs = getComputedStyle(el);
        if (cs.display === 'none' || cs.visibility === 'hidden') continue;
        const r = el.getBoundingClientRect();
        if (r.width === 0 && r.height === 0) continue;
        if (r.right <= vw + 1) continue;
        // Skip if any descendant also overflows further — the descendant
        // is the real cause, not us.
        let hasOverflowingChild = false;
        for (const child of el.getElementsByTagName('*')) {
          const cr = child.getBoundingClientRect();
          if (cr.right > r.right - 0.5) { hasOverflowingChild = true; break; }
        }
        if (hasOverflowingChild) continue;
        const tag = el.tagName.toLowerCase();
        const cls = (el.className && typeof el.className === 'string')
          ? '.' + el.className.trim().split(/\s+/).slice(0, 3).join('.')
          : '';
        const id = el.id ? '#' + el.id : '';
        const label = (el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 50);
        culprits.push(`${tag}${id}${cls} right=${r.right.toFixed(0)} text="${label}"`);
        if (culprits.length >= 5) break;
      }
      for (const c of culprits) issues.push(`  culprit: ${c}`);
    }
    const header = document.querySelector('.site-header');
    if (header) {
      const headerRect = header.getBoundingClientRect();
      if (headerRect.right > vw + 1) {
        issues.push(`.site-header right edge ${headerRect.right.toFixed(0)} exceeds viewport ${vw}`);
      }
      // Tall-header check: when the nav can't fit on one row it wraps to a
      // second row (or each button's text wraps), making the bar 1.5–2x its
      // natural height. The user perceives this as the bar "overflowing"
      // even though scrollWidth is still clean. ~80px is comfortably above
      // the single-line height (~58–73px depending on breakpoint) and
      // below any legitimate two-row layout we'd intentionally render.
      if (headerRect.height > 80) {
        issues.push(`.site-header is ${headerRect.height.toFixed(0)}px tall — nav row likely wrapping at width ${vw}`);
      }
      for (const btn of header.querySelectorAll('a, button')) {
        const cs = getComputedStyle(btn);
        if (cs.display === 'none' || cs.visibility === 'hidden') continue;
        const r = btn.getBoundingClientRect();
        if (r.width === 0 && r.height === 0) continue;
        if (r.right > vw + 1) {
          const label = (btn.textContent || '').trim().slice(0, 40);
          issues.push(`nav item "${label}" extends to ${r.right.toFixed(0)}px (viewport ${vw}px)`);
        }
      }
    }
    return issues;
  }, { vw: viewport.width });
}

// Widths we additionally probe (without saving PNGs) so we catch overflow
// outside the canonical 375px capture: smaller phones (320, 360) and
// the just-above-mobile-breakpoint band where desktop styles re-engage
// (601–768) — that's where five nav buttons can squeeze the brand off
// the page even when the 375px mobile shot looks fine.
const NARROW_PROBE_WIDTHS = [320, 360, 414, 601, 700, 768];

async function probeNarrowWidths(browser, path) {
  const issues = [];
  for (const width of NARROW_PROBE_WIDTHS) {
    const ctx = await browser.newContext({ viewport: { width, height: 800 } });
    const page = await ctx.newPage();
    await page.goto(`${BASE_URL}${path}`, { waitUntil: 'load', timeout: 60_000 });
    await sleep(200);
    const found = await checkLayout(page, { width, height: 800 });
    for (const i of found) issues.push(`@${width}px: ${i}`);
    await ctx.close();
  }
  return issues;
}

async function captureAll() {
  await mkdir(OUTPUT_DIR, { recursive: true });

  const shouldServe = process.argv.includes('--serve');
  let wranglerProc = null;
  let envFile = null;

  // Stub the secrets that handlers::health::essentials_missing checks for,
  // so /auth/login renders the real login template instead of the branded
  // maintenance fallback. Values are placeholders — the resulting OAuth
  // URLs won't work, but the page renders correctly for capture. Written
  // to a tempfile so we never touch the repo's .dev.vars.
  if (shouldServe) {
    envFile = join(tmpdir(), `concierge-screenshots-${process.pid}.env`);
    await writeFile(envFile, [
      'ENCRYPTION_KEY=screenshot-stub',
      'GOOGLE_OAUTH_CLIENT_ID=screenshot-stub.apps.googleusercontent.com',
      'GOOGLE_OAUTH_CLIENT_SECRET=screenshot-stub',
      'META_APP_ID=000000000000000',
      '',
    ].join('\n'));
  }

  try {
    if (shouldServe) {
      wranglerProc = await startWrangler(envFile);
      // First request after boot can be slow while the worker JIT-compiles
      // the wasm; give it a moment so the first screenshot isn't a fallback.
      await sleep(2000);
    }

    console.log('Launching Chromium...');
    const browser = await chromium.launch();

    const problems = [];
    const probedPaths = new Set();
    for (const shot of SHOTS) {
      const isMobile = shot.viewport.width <= 480;
      console.log(`  ${shot.name}  ${shot.path}  (${shot.viewport.width}x${shot.viewport.height})`);
      const ctx = await browser.newContext({ viewport: shot.viewport });
      const page = await ctx.newPage();
      await page.goto(`${BASE_URL}${shot.path}`, { waitUntil: 'load', timeout: 60_000 });
      // Settle any web fonts / animation transitions.
      await sleep(400);
      // Mobile shots go full-page so layout breakage further down the
      // scroll (footer, hero postcard, terminal log) is visible without
      // having to manually scroll a captured viewport.
      await page.screenshot({ path: join(OUTPUT_DIR, shot.name), fullPage: isMobile });
      const issues = await checkLayout(page, shot.viewport);
      if (issues.length) {
        problems.push({ shot: shot.name, issues });
        for (const issue of issues) console.log(`    ! ${issue}`);
      }
      await ctx.close();
      // For each unique path, also probe narrow phone widths once.
      if (isMobile && !probedPaths.has(shot.path)) {
        probedPaths.add(shot.path);
        const narrow = await probeNarrowWidths(browser, shot.path);
        if (narrow.length) {
          problems.push({ shot: `${shot.path} (narrow probe)`, issues: narrow });
          for (const i of narrow) console.log(`    ! ${i}`);
        }
      }
    }

    await browser.close();
    console.log(`Done. Wrote ${SHOTS.length} PNGs to doc/screenshots/`);

    if (problems.length) {
      console.error('\nLayout problems detected:');
      for (const { shot, issues } of problems) {
        console.error(`  ${shot}:`);
        for (const issue of issues) console.error(`    - ${issue}`);
      }
      throw new Error(`${problems.length} screenshot(s) had layout problems — see above.`);
    }
  } finally {
    if (wranglerProc) {
      console.log('Stopping wrangler dev...');
      wranglerProc.kill();
    }
    if (envFile) await rm(envFile, { force: true });
  }
}

captureAll().catch((err) => {
  console.error('Error:', err.stack || err.message);
  process.exit(1);
});
