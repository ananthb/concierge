import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';

/**
 * Screenshot capture for reviewing UI changes, and for the docs gallery.
 *
 * Only runs in the `screenshots` project (see playwright.config.ts), so
 * `npm test` doesn't churn `doc/screenshots/` on every run. Trigger with
 * `npm run screenshots`.
 */
const OUTPUT_DIR = join(process.cwd(), 'doc', 'screenshots');

const DESKTOP = { width: 1280, height: 800 };
const MOBILE = { width: 375, height: 812 };

type Json = Record<string, unknown>;

// ── Fixtures ───────────────────────────────────────────────────────────

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
    id: 'front-desk',
    label: 'Front Desk',
    tag: 'Default',
    business_name: 'Nutmeg & Crumb Bakery',
    prompt:
      'You reply for Nutmeg & Crumb Bakery in Indiranagar, Bengaluru (Tue–Sun, 8 AM–8 PM). Greet warmly, answer hours/location/menu questions, and hand off custom cake orders or bulk quotes to Asha.',
    opener: 'Hi! Welcome to Nutmeg & Crumb Bakery. How can we help today?',
  },
  {
    id: 'orders',
    label: 'Order & Booking',
    tag: 'Orders',
    business_name: 'ClearSmile Dental Clinic',
    prompt:
      'You reply for ClearSmile Dental Clinic in Pune (Mon–Sat, 9:30 AM–7:30 PM). Confirm appointment hours and location, collect preferred day/time, and hand off medical questions or bookings to the front desk.',
    opener: 'Hello from ClearSmile Dental Clinic! Looking to book an appointment?',
  },
  {
    id: 'after-hours',
    label: 'After-Hours Triage',
    tag: 'Triage',
    business_name: 'Kaveri Plumbing & Repairs',
    prompt:
      'You reply after hours for Kaveri Plumbing & Repairs. Ask whether the issue is urgent (active leak) or routine, collect the area/pin code, and hand off immediately to the on-call technician.',
    opener: 'Hi! Kaveri Plumbing here — is this an urgent leak or a routine repair?',
  },
  {
    id: 'concise',
    label: 'Concise Support',
    tag: 'Brief',
    business_name: 'Veda Yoga Studio',
    prompt:
      'You reply for Veda Yoga Studio. Keep answers to 1–2 short sentences on class timings (6 AM–8 PM) and trial passes, and hand off membership pricing or private sessions to the studio team.',
    opener: 'Namaste! Welcome to Veda Yoga Studio. How can we help?',
  },
];

const SESSION = {
  tenant_id: 'tenant-demo',
  email: 'asha@nutmegandcrumb.in',
  name: 'Asha',
  locale: 'en-IN',
  currency: 'INR',
  metered: true,
  destination: 'dashboard',
  onboarding_step: null,
  ai_ready: true,
  safety_status: 'approved',
  safety_reason: null,
};

const WHATSAPP_ACCOUNT = {
  id: 'wa-1',
  name: 'Nutmeg & Crumb Bakery',
  phone_number: '+91 98200 11223',
  phone_number_id: 'pnid-1',
  waba_id: 'waba-1',
  reply: {
    enabled: true,
    mode: 'ai',
    text: '',
    bot_prompt: '',
    wait_seconds: 3,
  },
  canned_safety_status: 'approved',
  created_at: '2026-09-01T09:00:00.000Z',
};

const SETTINGS = {
  conversation: {
    idle_gap_mins: null,
    handoff_cooldown_mins: null,
    max_history_messages: null,
    defaults: {
      idle_gap_mins: 240,
      handoff_cooldown_mins: 60,
      max_history_messages: 20,
    },
    min: {
      idle_gap_mins: 5,
      handoff_cooldown_mins: 5,
      max_history_messages: 4,
    },
    max: {
      idle_gap_mins: 1440,
      handoff_cooldown_mins: 1440,
      max_history_messages: 200,
    },
  },
  notifications: {
    handoff_email: true,
  },
  locale: 'en-IN',
  currency: 'INR',
  locales: ['en-IN', 'en-US'],
  currencies: ['INR', 'USD'],
};

/** Stub one JSON endpoint. */
async function stub(
  page: import('@playwright/test').Page,
  pattern: string,
  body: Json | unknown[],
) {
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
      max_user_turns: 6,
      idle_timeout_secs: 300,
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
    demo: { enabled: false, max_user_turns: 6, idle_timeout_secs: 300, personas: [] },
    session: SESSION,
    locale: { tag: 'en-IN', currency: 'INR' },
  });
  await stub(page, '**/api/whatsapp', {
    accounts: [WHATSAPP_ACCOUNT],
    signup: null,
    bot_presets: DEMO_PERSONAS.map(({ id, label, tag, prompt }) => ({ id, label, tag, prompt })),
  });
  await stub(page, '**/api/billing', {
    balance: 8450,
    replies_used: 1550,
    metered: true,
    currency: 'INR',
    pricing: {
      unit_price_milli: 10000,
      min_credits: 1000,
      max_credits: 1000000,
      currency: 'INR',
    },
    ledger: [
      {
        amount: 8450,
        source: 'purchase',
        expires_at: null,
        granted_at: '2026-09-12T00:00:00Z',
      },
    ],
  });
  await stub(page, '**/api/settings', SETTINGS);
}

/** Mid-wizard, at a given step. */
async function stubWizard(
  page: import('@playwright/test').Page,
  step: string,
  extra: Json = {},
) {
  await stub(page, '**/api/bootstrap', {
    pricing: PRICING,
    demo: { enabled: false, max_user_turns: 6, idle_timeout_secs: 300, personas: [] },
    session: { ...SESSION, destination: 'wizard', onboarding_step: step },
    locale: { tag: 'en-IN', currency: 'INR' },
  });
  await stub(page, '**/api/wizard', {
    step,
    steps: ['basics', 'channels', 'launch'],
    completed: false,
    business: {
      name: 'Nutmeg & Crumb Bakery',
      contact_name: 'Asha Iyer',
      phone: '+91 98200 11223',
      business_type: 'sole_proprietorship',
      pan: 'ABCDE1234F',
      gstin: '',
      address: '14 Hill Road, Bandra West',
      state: 'Maharashtra',
      pincode: '400050',
    },
    profile_safety_status: 'approved',
    whatsapp: [WHATSAPP_ACCOUNT],
    signup: { app_id: 'app-1', config_id: 'config-1', state: 'nonce' },
    launch: {
      balance: 0,
      unit_price_milli: 10000,
      currency: 'INR',
      min_credits: 1000,
      max_credits: 1000000,
      metered: true,
    },
    ...extra,
  });
}

async function settle(page: import('@playwright/test').Page) {
  await page.waitForTimeout(400);
}

async function capture(
  page: import('@playwright/test').Page,
  name: string,
  viewport: { width: number },
) {
  await page.screenshot({
    path: join(OUTPUT_DIR, name),
    fullPage: viewport.width <= 480,
  });
}

test.beforeAll(async () => {
  await mkdir(OUTPUT_DIR, { recursive: true });
});

// ── Public pages ───────────────────────────────────────────────────────

const PUBLIC_SHOTS: Array<{
  name: string;
  path: string;
  wait: string;
  viewport: typeof DESKTOP;
}> = [
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
    await page.locator(shot.wait).waitFor();
    await settle(page);
    await capture(page, shot.name, shot.viewport);
  });
}

// ── The demo, mid-conversation ─────────────────────────────────────────

const DEMO_REPLY =
  'Yes — we are open Tue–Sun, 8:00 AM – 8:00 PM! Let me know if you need anything else.';

async function openDemoConversation(page: import('@playwright/test').Page) {
  await stubPublic(page);
  await stub(page, '**/api/demo/chat', { reply: DEMO_REPLY, handed_off: false });
  await page.goto('/');
  await page.locator('.persona-btn').first().click();
  await page.locator('#demo-input').fill('Are you open on Sundays?');
  await page.locator('.demo-composer button[type="submit"]').click();
  await page.getByText(/8:00 AM/i).waitFor();
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

// ── The 3-step wizard ──────────────────────────────────────────────────

const WIZARD_STEPS = ['basics', 'channels', 'launch'];

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
  { name: 'dashboard-billing.png', path: '/dashboard/billing' },
  { name: 'dashboard-settings.png', path: '/dashboard/settings' },
];

for (const shot of DASHBOARD_SHOTS) {
  test(`capture ${shot.name}`, async ({ page }) => {
    await page.setViewportSize(DESKTOP);
    await stubSignedIn(page);
    await page.goto(shot.path);
    await page.locator('.dashboard .card').first().waitFor();
    await settle(page);
    await capture(page, shot.name, DESKTOP);
  });
}

test('capture dashboard-mobile.png', async ({ page }) => {
  await page.setViewportSize(MOBILE);
  await stubSignedIn(page);
  await page.goto('/dashboard');
  await page.locator('.dashboard .card').first().waitFor();
  await settle(page);
  await capture(page, 'dashboard-mobile.png', MOBILE);
});

// ── CSS coverage checks ────────────────────────────────────────────────

const STRUCTURAL = new Set([
  'demo-picker',
  'how-it-works',
  'page-features',
  'page-pricing',
]);

async function unstyledClasses(page: import('@playwright/test').Page): Promise<string[]> {
  return await page.evaluate(() => {
    const defined = new Set<string>();
    const walk = (rules: CSSRuleList) => {
      for (const rule of Array.from(rules)) {
        const styleRule = rule as CSSStyleRule & { cssRules?: CSSRuleList };
        if (styleRule.selectorText) {
          for (const match of styleRule.selectorText.matchAll(/\.([A-Za-z_][\w-]*)/g)) {
            defined.add(match[1]);
          }
        }
        if (styleRule.cssRules) walk(styleRule.cssRules);
      }
    };
    for (const sheet of Array.from(document.styleSheets)) {
      try {
        walk(sheet.cssRules);
      } catch {
        // Ignore cross-origin sheets.
      }
    }

    const used = new Set<string>();
    for (const el of Array.from(document.querySelectorAll('[class]'))) {
      for (const name of Array.from(el.classList)) used.add(name);
    }
    return Array.from(used)
      .filter((name) => !defined.has(name))
      .sort();
  });
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
  for (const step of WIZARD_STEPS) {
    await stubWizard(page, step);
    await page.goto('/wizard');
    await page.locator('.wizard-step').waitFor();
    await expectFullyStyled(page);
  }
});

test('the dashboard has a rule for every class it renders', async ({ page }) => {
  await stubSignedIn(page);
  for (const path of [
    '/dashboard',
    '/dashboard/channels',
    '/dashboard/billing',
    '/dashboard/settings',
  ]) {
    await page.goto(path);
    await page.locator('.dashboard .card').first().waitFor();
    await expectFullyStyled(page);
  }
});
