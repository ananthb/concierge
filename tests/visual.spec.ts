import { test } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';

/**
 * Screenshot capture for the docs gallery.
 *
 * Only runs in the `screenshots` project (see playwright.config.ts), so
 * `npm test` doesn't churn `doc/screenshots/` on every run. Trigger
 * with `npm run screenshots`.
 *
 * Each public page captures a desktop and a mobile shot; mobile uses
 * `fullPage: true` so layout breakage further down the scroll
 * (footer, hero phone stack) is visible without manual scrolling.
 *
 * `process.cwd()` is the repo root — Playwright always invokes specs
 * from there, and avoiding `import.meta.url` keeps esbuild's CJS
 * transpile happy.
 */
const OUTPUT_DIR = join(process.cwd(), 'doc', 'screenshots');

const DESKTOP = { width: 1280, height: 800 };
const MOBILE = { width: 375, height: 812 };

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

test.beforeAll(async () => {
  await mkdir(OUTPUT_DIR, { recursive: true });
});

for (const shot of SHOTS) {
  test(`capture ${shot.name}`, async ({ page }) => {
    await page.setViewportSize(shot.viewport);
    await page.goto(shot.path);
    // Settle any web fonts / fade-in animations.
    await page.waitForTimeout(400);
    const isMobile = shot.viewport.width <= 480;
    await page.screenshot({
      path: join(OUTPUT_DIR, shot.name),
      fullPage: isMobile,
    });
  });
}

// ── Activated demo captures ────────────────────────────────────────────
// The hero phone is the live demo: tapping it sweeps in a chat surface.
// These shots drive that interaction so the gallery shows the demo in
// use, not just the idle illustration on home.png. Personas + the AI
// reply are stubbed so the conversation is deterministic (the local dev
// server has no Workers AI binding).
const DEMO_PERSONAS = {
  personas: [
    {
      slug: 'concierge',
      label: 'Concierge',
      description: 'Talks about Concierge itself.',
      greeting: "Hi! I'm Concierge — ask me anything about how I work.",
      prompt: 'Voice: Concierge talking about itself.',
    },
    {
      slug: 'friendly_florist',
      label: 'Friendly Florist',
      description: 'A warm neighbourhood florist.',
      greeting: 'Hi there! Welcome to Petals & Stems 🌸 How can I help?',
      business: {
        name: 'Petals & Stems',
        business_type: 'florist',
        city: 'Mumbai',
        hours: 'Tue–Sun, 9am–7pm',
        goal: 'book a delivery slot',
        goal_url: '/book',
      },
      prompt: 'Business: Petals & Stems, a neighbourhood florist in Mumbai.',
    },
  ],
};

async function stubDemo(page: any) {
  await page.route('http://localhost:8787/', async (route: any) => {
    const resp = await route.fetch();
    const headers = resp.headers();
    const body = (await resp.text()).replace(
      /(<script id="demo-personas-data"[^>]*>)[\s\S]*?(<\/script>)/,
      `$1${JSON.stringify(DEMO_PERSONAS)}$2`,
    );
    await route.fulfill({
      status: resp.status(),
      headers,
      contentType: headers['content-type'] ?? 'text/html; charset=utf-8',
      body,
    });
  });
  await page.route('**/demo/chat', (route: any) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        reply:
          "Yes — Sundays included! I can hold a slot now and confirm the moment the shop's open. Want me to pencil you in?",
      }),
    }),
  );
}

// Activate the demo as a florist customer and run one exchange, leaving
// the phone mid-conversation for the screenshot. `stubDemo` must already
// be registered (its routes must intercept the navigation below).
async function openDemoConversation(page: any) {
  await page.goto('/');
  await page.locator('#hero-headline').click();
  const demo = page.getByRole('dialog', { name: /live demo/i });
  await demo.locator('[data-testid="demo-chat-persona"]').selectOption('friendly_florist');
  await demo.getByRole('textbox').fill('do you deliver on Sundays?');
  await demo.getByRole('button', { name: 'Send' }).click();
  await demo.getByText(/Sundays included/i).waitFor();
  // Let the wipe finish and bubbles settle before the shot.
  await page.waitForTimeout(500);
  return demo;
}

test('capture demo.png', async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await stubDemo(page);
  await openDemoConversation(page);
  await page.screenshot({ path: join(OUTPUT_DIR, 'demo.png') });
});

test('capture demo-mobile.png', async ({ page }) => {
  await page.setViewportSize(MOBILE);
  await stubDemo(page);
  await openDemoConversation(page);
  // The activated phone is full-screen on mobile, so a viewport shot is
  // the whole app surface.
  await page.screenshot({ path: join(OUTPUT_DIR, 'demo-mobile.png') });
});

test('capture demo-prompt.png', async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await stubDemo(page);
  const demo = await openDemoConversation(page);
  // Open the "how it works" reference modal: the full system-prompt
  // envelope for the selected persona.
  await demo.getByRole('button', { name: /how this works/i }).click();
  const modal = page.getByRole('dialog', { name: /how the demo works/i });
  await modal.waitFor();
  await page.waitForTimeout(300);
  await page.screenshot({ path: join(OUTPUT_DIR, 'demo-prompt.png') });
});
