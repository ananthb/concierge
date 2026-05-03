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
import { spawn } from 'node:child_process';
import { writeFileSync, unlinkSync } from 'node:fs';
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
    '',
  ].join('\n'),
);

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
