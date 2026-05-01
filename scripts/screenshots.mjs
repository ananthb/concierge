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
const SHOTS = [
  { name: 'home.png', path: '/', viewport: DESKTOP },
  { name: 'home-mobile.png', path: '/', viewport: MOBILE },
  { name: 'login.png', path: '/auth/login', viewport: DESKTOP },
  { name: 'features.png', path: '/features', viewport: DESKTOP },
  { name: 'pricing.png', path: '/pricing', viewport: DESKTOP },
  { name: 'terms.png', path: '/terms', viewport: DESKTOP },
  { name: 'privacy.png', path: '/privacy', viewport: DESKTOP },
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

    for (const shot of SHOTS) {
      console.log(`  ${shot.name}  ${shot.path}  (${shot.viewport.width}x${shot.viewport.height})`);
      const ctx = await browser.newContext({ viewport: shot.viewport });
      const page = await ctx.newPage();
      await page.goto(`${BASE_URL}${shot.path}`, { waitUntil: 'load', timeout: 60_000 });
      // Settle any web fonts / animation transitions.
      await sleep(400);
      await page.screenshot({ path: join(OUTPUT_DIR, shot.name), fullPage: false });
      await ctx.close();
    }

    await browser.close();
    console.log(`Done. Wrote ${SHOTS.length} PNGs to doc/screenshots/`);
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
