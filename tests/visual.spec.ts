import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';

/**
 * Screenshot capture for reviewing UI changes, and for the docs gallery.
 *
 * Only runs in the `screenshots` project (see playwright.config.ts), so
 * `npm test` doesn't churn `doc/screenshots/` on every run. Trigger with
 * `npm run screenshots`.
 *
 * More useful since the frontend became an Elm app, not less: you can't see a
 * route by curling it any more, so this is the cheapest way to look at every
 * screen at once. And because the app talks to a JSON API, everything below is
 * driven by stubbed fixtures rather than the old trick of rewriting an inline
 * `<script>` block — which means the signed-in screens (wizard, dashboard) can
 * be captured too. They never could be before, and they're the ones that most
 * need a look before a deploy.
 *
 * Every shot is deterministic: no live model calls, no real session, fixed
 * numbers. A diff in `doc/screenshots/` is therefore a real visual change.
 *
 * `process.cwd()` is the repo root — Playwright always invokes specs from
 * there, and avoiding `import.meta.url` keeps esbuild's CJS transpile happy.
 */
const OUTPUT_DIR = join(process.cwd(), 'doc', 'screenshots');

const DESKTOP = { width: 1280, height: 800 };
const MOBILE = { width: 375, height: 812 };

type Json = Record<string, unknown>;

// ── Fixtures ───────────────────────────────────────────────────────────
// Fixed values so a screenshot diff means the UI changed, not that the
// seeded pricing or a rolled demo persona did.

const PRICING = {
  unit_price_milli: 10000,
  currency: 'INR',
  min_credits: 1000,
  max_credits: 1000000,
  currencies: [
    { code: 'INR', unit_price_milli: 10000 },
    { code: 'USD', unit_price_milli: 100 },
  ],
};

const DEMO_PERSONAS = [
  {
    slug: 'friendly',
    label: 'Petals & Stems',
    description: 'A neighbourhood florist in Mumbai. Warm, familiar, quick to help.',
    greeting: 'Hi there! Welcome to Petals & Stems 🌸 How can I help?',
    prompt:
      'Voice: warm, kind, conversational. Speak like a shopkeeper who has known the customer for years.\n\nBusiness: Petals & Stems, a florist in Mumbai.\nOpen: Tue–Sun, 9am–7pm.\nAim for: booking a delivery slot.\n\nStop and fetch a human when: the customer asks for a wedding quote.',
    business: { name: 'Petals & Stems' },
  },
  {
    slug: 'professional',
    label: 'Mehta & Co. Accountants',
    description: 'Brief and businesslike. Confirms what is possible, defers the rest.',
    greeting: 'Thanks for reaching out. How can we help you today?',
    prompt:
      'Voice: concise and professional. Greet briefly, confirm what is possible, ask for the missing detail.\n\nBusiness: Mehta & Co., a chartered accountancy practice.',
    business: { name: 'Mehta & Co. Accountants' },
  },
];

const SESSION = {
  tenant_id: 'tenant-demo',
  email: 'asha@petalsandstems.in',
  name: 'Asha',
  locale: 'en-IN',
  currency: 'INR',
  metered: true,
  destination: 'dashboard',
  onboarding_step: null,
  ai_ready: true,
};

const WHATSAPP_ACCOUNT = {
  id: 'wa-1',
  name: 'Petals & Stems',
  phone_number: '+91 98200 11223',
  phone_number_id: 'pnid-1',
  reply: {
    enabled: true,
    mode: 'prompt',
    text: "Reply to the customer's message helpfully.",
    wait_seconds: 5,
  },
  created_at: '2026-09-01T09:00:00.000Z',
};

/** Stub one JSON endpoint. */
async function stub(page: import('@playwright/test').Page, pattern: string, body: Json | unknown[]) {
  await page.route(pattern, (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(body),
    }),
  );
}

/** The public landing state: pricing and the demo, nobody signed in. */
async function stubPublic(page: import('@playwright/test').Page, demoEnabled = true) {
  await stub(page, '**/api/bootstrap', {
    pricing: PRICING,
    demo: {
      enabled: demoEnabled,
      max_user_turns: 8,
      idle_timeout_secs: 120,
      personas: demoEnabled ? DEMO_PERSONAS : [],
    },
    session: null,
    locale: { tag: 'en-IN', currency: 'INR' },
  });
}

/** Signed in, onboarding finished. */
async function stubSignedIn(page: import('@playwright/test').Page) {
  await stub(page, '**/api/bootstrap', {
    pricing: PRICING,
    demo: { enabled: false, max_user_turns: 8, idle_timeout_secs: 120, personas: [] },
    session: SESSION,
    locale: { tag: 'en-IN', currency: 'INR' },
  });
  await stub(page, '**/api/whatsapp', { accounts: [WHATSAPP_ACCOUNT], signup: null });
  await stub(page, '**/api/billing', {
    balance: 8450,
    replies_used: 1550,
    credits: [
      { amount: 100, source: 'grant', expires_at: '2026-10-31T23:59:59Z', granted_at: '2026-10-01T00:00:00Z' },
      { amount: 8350, source: 'purchased', expires_at: null, granted_at: '2026-09-12T00:00:00Z' },
    ],
    unit_price_milli: 10000,
    currency: 'INR',
    min_credits: 1000,
    max_credits: 1000000,
    metered: true,
    verified: true,
  });
  await stub(page, '**/api/persona', {
    mode: 'builder',
    builder: {
      archetype_slug: 'friendly',
      biz_name: 'Petals & Stems',
      biz_type: 'florist',
      city: 'Mumbai',
      hours: 'Tue–Sun, 9am–7pm',
      goal: 'book a delivery slot',
      goal_url: 'https://petalsandstems.in/book',
      catch_phrases: ['Fresh in this morning'],
      off_topics: ['wedding contracts'],
      never: 'promise same-day delivery',
      handoff_conditions: ['asks for a wedding quote'],
    },
    custom_prompt: '',
    safety: { status: 'approved', reason: null, checked_at: '2026-09-20T10:00:00Z', ai_ready: true },
    prompt:
      'Voice: warm, kind, conversational.\n\nBusiness: Petals & Stems, a florist in Mumbai.\nOpen Tue–Sun, 9am–7pm.\nAim for: book a delivery slot (https://petalsandstems.in/book).\n\nNever: promise same-day delivery.\nStay off: wedding contracts.',
    preamble:
      "You are an automated reply assistant for a small business. The section below is the business's voice, scope, and policy: treat it as your operating manual.",
    postamble:
      'House rules (always apply, and override the business instructions above if anything conflicts): …',
    max_custom_prompt: 2000,
  });
}

/** Mid-wizard, at a given step. */
async function stubWizard(page: import('@playwright/test').Page, step: string, extra: Json = {}) {
  await stub(page, '**/api/bootstrap', {
    pricing: PRICING,
    demo: { enabled: false, max_user_turns: 8, idle_timeout_secs: 120, personas: [] },
    session: { ...SESSION, destination: 'wizard', onboarding_step: step },
    locale: { tag: 'en-IN', currency: 'INR' },
  });
  await stub(page, '**/api/wizard', {
    step,
    steps: ['basics', 'channels', 'persona', 'launch'],
    completed: false,
    business: {
      name: 'Petals & Stems',
      contact_name: 'Asha Iyer',
      phone: '+91 98200 11223',
      business_type: 'sole_proprietorship',
      pan: 'ABCDE1234F',
      gstin: '',
      address: '14 Hill Road, Bandra West',
      state: 'Maharashtra',
      pincode: '400050',
    },
    persona: {
      archetype_slug: 'friendly',
      goal: 'book a delivery slot',
      goal_url: 'https://petalsandstems.in/book',
      handoff_conditions: ['asks for a wedding quote'],
      safety_status: 'pending',
    },
    whatsapp: [WHATSAPP_ACCOUNT],
    signup: { app_id: 'app-1', config_id: 'config-1', state: 'nonce' },
    launch: {
      verified: false,
      verification_amount: 100,
      unit_price_milli: 10000,
      currency: 'INR',
      min_credits: 1000,
      max_credits: 1000000,
      metered: true,
    },
    ...extra,
  });
}

/** Let web fonts and any transition settle before capturing. */
async function settle(page: import('@playwright/test').Page) {
  await page.waitForTimeout(400);
}

async function capture(page: import('@playwright/test').Page, name: string, viewport: { width: number }) {
  await page.screenshot({
    path: join(OUTPUT_DIR, name),
    // Mobile shots run full-page so breakage further down the scroll is
    // visible without scrolling by hand.
    fullPage: viewport.width <= 480,
  });
}

test.beforeAll(async () => {
  await mkdir(OUTPUT_DIR, { recursive: true });
});

// ── Public pages ───────────────────────────────────────────────────────

const PUBLIC_SHOTS: Array<{ name: string; path: string; wait: string; viewport: typeof DESKTOP }> = [
  { name: 'home.png', path: '/', wait: '.landing', viewport: DESKTOP },
  { name: 'home-mobile.png', path: '/', wait: '.landing', viewport: MOBILE },
  { name: 'login.png', path: '/login', wait: '.page-login', viewport: DESKTOP },
  { name: 'login-mobile.png', path: '/login', wait: '.page-login', viewport: MOBILE },
  { name: 'features.png', path: '/features', wait: '.page-features', viewport: DESKTOP },
  { name: 'features-mobile.png', path: '/features', wait: '.page-features', viewport: MOBILE },
  { name: 'pricing.png', path: '/pricing', wait: '.page-pricing', viewport: DESKTOP },
  { name: 'pricing-mobile.png', path: '/pricing', wait: '.page-pricing', viewport: MOBILE },
  { name: 'terms.png', path: '/terms', wait: '.page-legal', viewport: DESKTOP },
  { name: 'terms-mobile.png', path: '/terms', wait: '.page-legal', viewport: MOBILE },
  { name: 'privacy.png', path: '/privacy', wait: '.page-legal', viewport: DESKTOP },
  { name: 'privacy-mobile.png', path: '/privacy', wait: '.page-legal', viewport: MOBILE },
];

for (const shot of PUBLIC_SHOTS) {
  test(`capture ${shot.name}`, async ({ page }) => {
    await page.setViewportSize(shot.viewport);
    await stubPublic(page);
    await page.goto(shot.path);
    // Wait on the rendered page, not a timer: a shot of an empty
    // empty body would silently become the gallery image.
    await page.locator(shot.wait).waitFor();
    await settle(page);
    await capture(page, shot.name, shot.viewport);
  });
}

// ── The demo, mid-conversation ─────────────────────────────────────────

const DEMO_REPLY =
  "Yes — Sundays included! I can hold a slot now and confirm the moment the shop's open. Want me to pencil you in?";

async function openDemoConversation(page: import('@playwright/test').Page) {
  await stubPublic(page);
  await stub(page, '**/api/demo/chat', { reply: DEMO_REPLY, handoff: false });
  await page.goto('/');
  await page.locator('.persona-card').first().click();
  await page.locator('#demo-input').fill('do you deliver on Sundays?');
  await page.locator('.demo-composer button[type="submit"]').click();
  await page.getByText(/Sundays included/i).waitFor();
  await settle(page);
}

test('capture demo.png', async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openDemoConversation(page);
  await capture(page, 'demo.png', DESKTOP);
});

test('capture demo-mobile.png', async ({ page }) => {
  await page.setViewportSize(MOBILE);
  await openDemoConversation(page);
  await capture(page, 'demo-mobile.png', MOBILE);
});

test('capture demo-prompt.png', async ({ page }) => {
  // The "view the prompt" panel is the most persuasive thing on the page:
  // the exact middle the model is sent, with the fixed bookends named.
  await page.setViewportSize(DESKTOP);
  await openDemoConversation(page);
  await page.getByRole('button', { name: /View the prompt/ }).click();
  await page.locator('.prompt-panel').waitFor();
  await settle(page);
  await capture(page, 'demo-prompt.png', DESKTOP);
});

// ── The wizard ─────────────────────────────────────────────────────────
// Never captured before this refactor: the steps needed a real session, and
// there was no way to stand one up. Stubbing the API gets all four.

const WIZARD_STEPS = ['basics', 'channels', 'persona', 'launch'];

for (const step of WIZARD_STEPS) {
  test(`capture wizard-${step}.png`, async ({ page }) => {
    await page.setViewportSize(DESKTOP);
    await stubWizard(page, step);
    await page.goto('/wizard');
    await page.locator('.wizard-step').waitFor();
    await settle(page);
    await capture(page, `wizard-${step}.png`, DESKTOP);
  });

  test(`capture wizard-${step}-mobile.png`, async ({ page }) => {
    await page.setViewportSize(MOBILE);
    await stubWizard(page, step);
    await page.goto('/wizard');
    await page.locator('.wizard-step').waitFor();
    await settle(page);
    await capture(page, `wizard-${step}-mobile.png`, MOBILE);
  });
}

// ── The dashboard ──────────────────────────────────────────────────────

const DASHBOARD_SHOTS: Array<{ name: string; path: string }> = [
  { name: 'dashboard.png', path: '/dashboard' },
  { name: 'dashboard-channels.png', path: '/dashboard/channels' },
  { name: 'dashboard-persona.png', path: '/dashboard/persona' },
  { name: 'dashboard-billing.png', path: '/dashboard/billing' },
  { name: 'dashboard-settings.png', path: '/dashboard/settings' },
];

for (const shot of DASHBOARD_SHOTS) {
  test(`capture ${shot.name}`, async ({ page }) => {
    await page.setViewportSize(DESKTOP);
    await stubSignedIn(page);
    await page.goto(shot.path);
    await page.locator('.dashboard .card, .dashboard .spinner').first().waitFor();
    // The card is what we want, not the spinner that precedes it.
    await page.locator('.dashboard .card').first().waitFor();
    await settle(page);
    await capture(page, shot.name, DESKTOP);
  });
}

test('capture dashboard-persona-editor.png', async ({ page }) => {
  // The editor behind "Edit your voice" — the longest form in the app, and
  // the one whose fields don't obviously map onto the prompt underneath, so
  // it's worth looking at whole.
  await page.setViewportSize(DESKTOP);
  await stubSignedIn(page);
  await stub(page, '**/api/archetypes', {
    archetypes: [
      { slug: 'friendly', label: 'Friendly', description: 'Warm and familiar.', greeting: 'Hi there!' },
      { slug: 'professional', label: 'Professional', description: 'Brief and businesslike.', greeting: 'Thanks for reaching out.' },
      { slug: 'playful', label: 'Playful', description: 'Upbeat, a little emoji.', greeting: 'hi 👋' },
      { slug: 'formal', label: 'Formal', description: 'Polite and measured.', greeting: 'Good day.' },
    ],
  });
  await page.goto('/dashboard/persona');
  await page.getByRole('button', { name: /Edit your voice/ }).click();
  await page.locator('.voice-grid').waitFor();
  await settle(page);
  await page.screenshot({ path: join(OUTPUT_DIR, 'dashboard-persona-editor.png'), fullPage: true });
});

test('capture dashboard-mobile.png', async ({ page }) => {
  await page.setViewportSize(MOBILE);
  await stubSignedIn(page);
  await page.goto('/dashboard');
  await page.locator('.dashboard .card').first().waitFor();
  await settle(page);
  await capture(page, 'dashboard-mobile.png', MOBILE);
});

/**
 * Every class the app renders must have a rule behind it.
 *
 * This is the check that was missing when the Elm frontend shipped. The
 * views were written against class names the stylesheets never defined —
 * `.hero`, `.steps`, `.persona-card`, `.field`, `.tabs`, eighty-odd of them —
 * and nothing caught it. The markup assertions passed, because the markup was
 * right. The stylesheet assertions passed, because the files loaded and
 * applied. The page just had no layout: full-bleed text, list bullets, cards
 * that weren't cards.
 *
 * Rendering the screens is the only way to see it, which is why this lives
 * beside the screenshots and reuses their stubs rather than reading the Elm
 * source. A class that never reaches the DOM is not this test's business.
 */

/**
 * Structural hooks: wrappers that exist to be targeted from a parent's rule
 * (`.landing > *` spacing) or to name a page for a future override. They
 * carry no rule of their own by design.
 */
const STRUCTURAL = new Set([
  'demo-picker',
  'how-it-works',
  'page-features',
  'page-pricing',
]);

async function unstyledClasses(page: import('@playwright/test').Page): Promise<string[]> {
  const undefinedClasses = await page.evaluate(() => {
    const defined = new Set<string>();
    const walk = (rules: CSSRuleList) => {
      for (const rule of Array.from(rules)) {
        const styleRule = rule as CSSStyleRule & { cssRules?: CSSRuleList };
        if (styleRule.selectorText) {
          for (const match of styleRule.selectorText.matchAll(/\.([A-Za-z_][\w-]*)/g)) {
            defined.add(match[1]);
          }
        }
        // @media and @supports hold their own rule lists.
        if (styleRule.cssRules) walk(styleRule.cssRules);
      }
    };
    for (const sheet of Array.from(document.styleSheets)) {
      try {
        walk(sheet.cssRules);
      } catch {
        // A sheet we can't read isn't one of ours.
      }
    }

    const used = new Set<string>();
    for (const el of Array.from(document.querySelectorAll('[class]'))) {
      for (const name of Array.from(el.classList)) used.add(name);
    }
    return Array.from(used).filter((name) => !defined.has(name)).sort();
  });
  return undefinedClasses;
}

async function expectFullyStyled(page: import('@playwright/test').Page) {
  const unstyled = (await unstyledClasses(page)).filter((c) => !STRUCTURAL.has(c));
  expect(
    unstyled,
    `classes rendered with no CSS rule anywhere:\n  ${unstyled.join('\n  ')}`,
  ).toEqual([]);
}

test('landing has a rule for every class it renders', async ({ page }) => {
  await stubPublic(page);
  await page.goto('/');
  await page.locator('.landing').waitFor();
  await expectFullyStyled(page);
});

test('the demo conversation has a rule for every class it renders', async ({ page }) => {
  await stubPublic(page);
  await openDemoConversation(page);
  // The prompt drawer and the typing bubble are the two states with classes
  // of their own that the transcript alone doesn't render.
  await page.getByRole('button', { name: /View the prompt/ }).click();
  await page.locator('.prompt-panel').waitFor();
  await expectFullyStyled(page);
});

for (const path of ['/pricing', '/features', '/terms', '/privacy']) {
  test(`${path} has a rule for every class it renders`, async ({ page }) => {
    await stubPublic(page);
    await page.goto(path);
    await page.locator('.page').waitFor();
    await expectFullyStyled(page);
  });
}

test('the wizard has a rule for every class it renders', async ({ page }) => {
  await stubWizard(page, 'basics');
  await page.goto('/wizard');
  await page.locator('.wizard-step').waitFor();
  await expectFullyStyled(page);
});

test('the dashboard has a rule for every class it renders', async ({ page }) => {
  await stubSignedIn(page);
  await page.goto('/dashboard');
  await page.locator('.dashboard .card').first().waitFor();
  await expectFullyStyled(page);
});

/**
 * The operator console.
 *
 * Stubbed like every other screen here, and worth capturing for the same
 * reason: nothing else renders it. `/api/manage/*` answers to a Cloudflare
 * Access JWT, so a real session can't reach it and the screenshot job has no
 * way to produce one.
 */
async function stubManage(page: import('@playwright/test').Page) {
  await stubPublic(page, false);
  await stub(page, '**/api/manage/overview', {
    actor: 'operator@example.com',
    tenant_count: 42,
    health: {
      overall: 'ok',
      generated_at: '2026-09-29T09:00:00Z',
      deep: true,
      checks: [
        { name: 'WhatsApp', status: 'ok', detail: 'Token and WABA id set.' },
        { name: 'Razorpay', status: 'warn', detail: 'Webhook secret missing.' },
      ],
    },
  });
  await stub(page, '**/api/manage/pricing', {
    min_credits: 100,
    max_credits: 100000,
    max_credits_ceiling: 1000000,
    amounts: [
      { concept: 'unit_price_milli', currency: 'INR', amount: 10000 },
      { concept: 'unit_price_milli', currency: 'USD', amount: 100 },
    ],
    concepts: [
      {
        wire: 'unit_price_milli',
        label: 'Per AI reply',
        unit_caption: 'Thousandths of a paisa / cent',
        is_milli: true,
      },
    ],
    currencies: ['INR', 'USD'],
  });
  await stub(page, '**/api/manage/demo', {
    config: {
      enabled: true,
      persona_generation_prompt: 'Write four small Indian businesses…',
      regeneration_cadence_mins: 60,
      idle_timeout_secs: 120,
      max_user_turns: 8,
    },
    generated_at: '2026-09-29T08:30:00Z',
    default_prompt: 'Write four small Indian businesses…',
  });
  await stub(page, '**/api/manage/tenants*', {
    tenants: [
      {
        id: 'ten_1',
        email: 'asha@petalsandstems.in',
        name: 'Petals & Stems',
        plan: 'paid',
        currency: 'INR',
        created_at: '2026-09-01T10:00:00Z',
      },
    ],
  });
  await stub(page, '**/api/manage/audit*', {
    entries: [
      {
        created_at: '2026-09-29T09:10:00Z',
        actor_email: 'operator@example.com',
        action: 'update_pricing',
        resource_type: 'pricing',
        resource_id: null,
      },
    ],
    has_more: false,
  });
}

test('capture manage.png', async ({ page }) => {
  await stubManage(page);
  await page.goto('/manage');
  await page.locator('.manage .card').first().waitFor();
  await settle(page);
  await capture(page, 'manage.png', DESKTOP);
});

test('capture manage-pricing.png', async ({ page }) => {
  await stubManage(page);
  await page.goto('/manage');
  await page.getByRole('button', { name: 'Pricing' }).click();
  await page.locator('.rate-table').waitFor();
  await settle(page);
  await capture(page, 'manage-pricing.png', DESKTOP);
});

test('the console has a rule for every class it renders', async ({ page }) => {
  await stubManage(page);
  await page.goto('/manage');
  await page.locator('.manage .card').first().waitFor();
  await expectFullyStyled(page);
  // The tabs render different markup per panel, so each one is its own
  // chance to ship an unstyled class.
  for (const tab of ['Pricing', 'Demo', 'Tenants', 'Audit']) {
    await page.getByRole('button', { name: tab }).click();
    await page.locator('.manage .card').first().waitFor();
    await expectFullyStyled(page);
  }
});

/**
 * Access denial is the state every visitor without an operator JWT sees, so
 * it has to read as an explanation rather than as a broken page.
 */
test('the console explains itself when Access has not let you in', async ({ page }) => {
  await stubPublic(page, false);
  await page.route('**/api/manage/**', (route) =>
    route.fulfill({
      status: 403,
      contentType: 'application/json',
      body: JSON.stringify({
        error: { code: 'access_required', message: 'This endpoint requires Cloudflare Access.' },
      }),
    }),
  );
  await page.goto('/manage');
  await expect(page.getByText('This console needs Cloudflare Access')).toBeVisible();
  // Not the generic failure banner.
  await expect(page.locator('.banner.error')).toHaveCount(0);
});
