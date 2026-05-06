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
import { writeFileSync, unlinkSync, readdirSync, existsSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// Resolve paths in two distinct ways:
//   * Read-only project files (migrations, wrangler.toml) come from
//     `import.meta.url` — this works when the script is in the
//     project tree (`npm test`) AND when it's been copied into the
//     Nix store (`nix run .#dev`). In the latter case the migrations
//     are read out of the store, which is fine.
//   * Writable paths (`.wrangler/state`) anchor at `process.cwd()`
//     so wrangler creates its local D1 + KV inside the user's
//     checkout, never inside the read-only Nix store.
const PROJECT_DIR_FOR_READING = new URL('..', import.meta.url).pathname;
const CWD = process.cwd();

if (!existsSync(join(CWD, 'wrangler.toml'))) {
  console.error(
    'test-server.mjs: refusing to run from a directory without wrangler.toml.\n' +
      `  cwd: ${CWD}\n` +
      "  cd into the project root and re-run (or invoke `nix run .#dev` from there).",
  );
  process.exit(2);
}

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

// Optional fresh-slate wipe — only runs when `CONCIERGE_TEST_RESET=1`
// is set. Playwright sets it so each suite starts on a clean D1 + KV;
// interactive `nix run .#dev` leaves it unset so signups, tenants,
// and KV state persist across restarts.
if (process.env.CONCIERGE_TEST_RESET === '1') {
  try {
    rmSync(join(CWD, '.wrangler', 'state'), { recursive: true, force: true });
  } catch {
    // No prior state to wipe.
  }
}

// Apply every migration in `migrations/` against the local D1 before
// wrangler dev starts. Without this, the management panel's pricing,
// audit, scheduled-grants, and archetypes pages 500 with "no such
// table". The --local flag points wrangler at the same SQLite file
// that `wrangler dev --local` will use when it boots.
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

// `--local` disables remote bindings entirely. We only have one remote
// binding (the AI one), and the test suite doesn't exercise it; without
// `--local` wrangler tries to open a remote proxy session for AI, which
// needs `wrangler login` creds — works on a developer's laptop where
// they're cached, breaks in CI.
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
