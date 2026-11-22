#!/usr/bin/env node
/**
 * Wrangler dev server for Playwright Test.
 *
 * Playwright's `webServer` config wants a single command string. This shim
 * writes a temp .env with stub secrets, applies D1 migrations locally, and
 * spawns `wrangler dev --local`.
 */
import { spawn, spawnSync } from 'node:child_process';
import { writeFileSync, unlinkSync, readdirSync, existsSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const PROJECT_DIR_FOR_READING = new URL('..', import.meta.url).pathname;
const CWD = process.cwd();

if (!existsSync(join(CWD, 'wrangler.toml'))) {
  console.error(
    'test-server.mjs: refusing to run from a directory without wrangler.toml.\n' +
      `  cwd: ${CWD}\n` +
      '  cd into the project root and re-run (or invoke `nix run .#dev` from there).',
  );
  process.exit(2);
}

const PORT = process.env.PLAYWRIGHT_DEV_PORT ?? '8787';
const ENV_FILE = join(tmpdir(), `concierge-playwright-${process.pid}.env`);

writeFileSync(
  ENV_FILE,
  [
    'GOOGLE_OAUTH_CLIENT_ID=screenshot-stub.apps.googleusercontent.com',
    'GOOGLE_OAUTH_CLIENT_SECRET=screenshot-stub',
    'META_APP_ID=000000000000000',
    'META_APP_SECRET=screenshot-stub',
    'WHATSAPP_ACCESS_TOKEN=screenshot-stub',
    'WHATSAPP_WABA_ID=000000000000000',
    'WHATSAPP_SIGNUP_CONFIG_ID=000000000000000',
    'RAZORPAY_KEY_ID=rzp_test_stub',
    'RAZORPAY_KEY_SECRET=rzp_secret_stub',
    'RAZORPAY_WEBHOOK_SECRET=whsec_stub',
    `PUBLIC_BASE_URL=http://localhost:${PORT}`,
    // Local dev bypass for AI bindings and /auth/dev-login — see `crate::dev_bypass`.
    'DEV_MODE=1',
    '',
  ].join('\n'),
);

if (process.env.CONCIERGE_TEST_RESET === '1') {
  try {
    rmSync(join(CWD, '.wrangler', 'state'), { recursive: true, force: true });
  } catch {
    // No prior state to wipe.
  }
}

const migrationsDir = join(PROJECT_DIR_FOR_READING, 'migrations');
const migrationFiles = readdirSync(migrationsDir)
  .filter((f) => f.endsWith('.sql'))
  .sort();
for (const file of migrationFiles) {
  const r = spawnSync(
    'wrangler',
    ['d1', 'execute', 'concierge', '--local', '--file', join(migrationsDir, file)],
    { stdio: 'inherit', cwd: CWD },
  );
  if (r.status !== 0) {
    console.error(`migration ${file} failed (exit ${r.status}); aborting`);
    process.exit(r.status ?? 1);
  }
}

const wrangler = spawn(
  'wrangler',
  ['dev', '--local', '--port', PORT, '--env-file', ENV_FILE],
  { stdio: 'inherit', cwd: CWD, env: { ...process.env, FORCE_COLOR: '0' } },
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
