#!/usr/bin/env node
/**
 * Wrangler dev server for Playwright Test.
 *
 * Playwright's `webServer` config wants a single command string. This shim
 * does the surrounding plumbing the screenshot pipeline used to handle
 * inline:
 *   - writes a temp .env with stub secrets so /auth/login renders the real
 *     login template instead of the maintenance fallback (the public site
 *     refuses to render auth pages without the OAuth client IDs configured),
 *   - spawns `wrangler dev --env-file <tmp>` on the chosen port,
 *   - cleans up on SIGTERM / exit so a CI run doesn't leak processes.
 *
 * The values are placeholders — the resulting OAuth URLs don't work, but
 * every page renders correctly for tests.
 */
import { spawn, spawnSync } from 'node:child_process';
import { writeFileSync, unlinkSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const PORT = process.env.PLAYWRIGHT_DEV_PORT ?? '8787';
const ENV_FILE = join(tmpdir(), `concierge-playwright-${process.pid}.env`);

writeFileSync(
  ENV_FILE,
  [
    'ENCRYPTION_KEY=screenshot-stub',
    'GOOGLE_OAUTH_CLIENT_ID=screenshot-stub.apps.googleusercontent.com',
    'GOOGLE_OAUTH_CLIENT_SECRET=screenshot-stub',
    'META_APP_ID=000000000000000',
    `PUBLIC_BASE_URL=http://localhost:${PORT}`,
    // Dev bypass for the management panel and AI bindings — see
    // `crate::dev_bypass`. Active iff CF_ACCESS_AUD is empty (it
    // never gets set in this file) AND MANAGE_BYPASS_EMAIL is
    // non-empty. Production injects CF_ACCESS_AUD via Cloudflare
    // Workers build env, so the bypass cannot activate there.
    'MANAGE_BYPASS_EMAIL=admin-test@example.com',
    '',
  ].join('\n'),
);

// Wipe `.wrangler/state` so each test run starts from a clean local
// D1 + KV. Without this, leftover schema from a previous wrangler
// dev session (created by an older migration) trips IF NOT EXISTS
// checks on subsequent runs (e.g. a CREATE INDEX referencing a
// column the existing table doesn't have).
try {
  rmSync(new URL('../.wrangler/state', import.meta.url).pathname, {
    recursive: true,
    force: true,
  });
} catch {
  // No prior state to wipe; first ever boot.
}

// Apply every migration in `migrations/` against the local D1 before
// wrangler dev starts. Without this, the management panel's pricing,
// audit, scheduled-grants, and archetypes pages 500 with "no such
// table". The --local flag points wrangler at the same SQLite file
// that `wrangler dev --local` will use when it boots.
const migrationsDir = new URL('../migrations/', import.meta.url).pathname;
const migrationFiles = readdirSync(migrationsDir)
  .filter((f) => f.endsWith('.sql'))
  .sort();
for (const file of migrationFiles) {
  const r = spawnSync(
    'wrangler',
    ['d1', 'execute', 'concierge', '--local', '--file', join(migrationsDir, file)],
    { stdio: 'inherit' },
  );
  if (r.status !== 0) {
    console.error(`migration ${file} failed (exit ${r.status}); aborting`);
    process.exit(r.status ?? 1);
  }
}

// `--local` disables remote bindings entirely. We only have one remote
// binding (the AI one), and the test suite doesn't exercise it; without
// `--local` wrangler tries to open a remote proxy session for AI, which
// needs `wrangler login` creds — works on a developer's laptop where
// they're cached, breaks in CI.
const wrangler = spawn(
  'wrangler',
  ['dev', '--local', '--port', PORT, '--env-file', ENV_FILE],
  { stdio: 'inherit', env: { ...process.env, FORCE_COLOR: '0' } },
);

const cleanup = () => {
  try { unlinkSync(ENV_FILE); } catch {}
  if (!wrangler.killed) wrangler.kill();
};

process.on('SIGTERM', cleanup);
process.on('SIGINT', cleanup);
process.on('exit', cleanup);

wrangler.on('exit', (code) => {
  cleanup();
  process.exit(code ?? 0);
});
