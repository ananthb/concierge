import { test, expect } from './_helpers/fixtures';

const PUBLIC_PAGES = [
  { path: '/', title: /Concierge/i },
  { path: '/features', title: /features/i },
  { path: '/pricing', title: /pricing/i },
  { path: '/auth/login', title: /sign in/i },
  { path: '/terms', title: /terms/i },
  { path: '/privacy', title: /privacy/i },
];

for (const { path, title } of PUBLIC_PAGES) {
  test(`${path} renders cleanly`, async ({ page, consoleErrors }) => {
    await page.goto(path);
    await expect(page).toHaveTitle(title);
    await expect(page.locator('main#main')).toBeVisible();
    await expect(page.locator('footer.site-footer')).toBeVisible();
    expect(consoleErrors).toEqual([]);
  });
}

test('public nav has Features / Pricing / Open source / Sign in (no Docs)', async ({ page }) => {
  await page.goto('/');
  const nav = page.locator('header.site-header nav.site-nav');
  await expect(nav.getByRole('link', { name: 'Features' })).toBeVisible();
  await expect(nav.getByRole('link', { name: 'Pricing' })).toBeVisible();
  await expect(nav.getByRole('link', { name: 'Sign in' })).toBeVisible();
  // Open source is in the DOM on every viewport; it's just `display:none`
  // below 760px (`.nav-ext`). Match the underlying anchor so the
  // assertion holds for both desktop and mobile projects.
  await expect(nav.locator('a[href*="github.com/ananthb/concierge"]')).toHaveCount(1);
  // Docs was deliberately removed from the top nav (still in the footer).
  await expect(nav.locator('a[href*="ananthb.github.io"]')).toHaveCount(0);
});

test('footer carries all seven links', async ({ page }) => {
  await page.goto('/');
  const footer = page.locator('footer.site-footer');
  for (const name of ['Features', 'Pricing', 'Docs', 'Open-source', 'AGPL-3.0', 'Terms of Service', 'Privacy Policy']) {
    await expect(footer.getByRole('link', { name })).toBeVisible();
  }
});

test('brand link returns home', async ({ page }) => {
  await page.goto('/pricing');
  const brand = page.locator('header.site-header a.brand');
  await expect(brand).toHaveAttribute('href', '/');
});

// Stub catalog payload reused by every demo-chat spec — `/demo/personas`
// reads from D1, but the dev test server doesn't run migrations, so we
// fulfill the route with a fixed list. The Concierge prompt body is
// what the View-prompt panel asserts against.
const CONCIERGE_DEMO_PROMPT =
  'Voice: Concierge talking about itself. WhatsApp Business, Instagram, Discord, email.';
async function stubDemoPersonas(page: any) {
  await page.route('**/demo/personas', (route: any) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        personas: [
          {
            slug: 'concierge',
            label: 'Concierge',
            description: 'Talks about Concierge.',
            greeting: "Hi! I'm Concierge.",
            is_system: true,
            prompt: CONCIERGE_DEMO_PROMPT,
          },
          {
            slug: 'friendly_florist',
            label: 'Friendly Florist',
            description: 'Florist voice.',
            greeting: 'Hi there! Welcome to the shop.',
            is_system: false,
            business: {
              name: 'Petals & Stems',
              business_type: 'florist',
              city: 'Mumbai',
              hours: 'Tue–Sun 9am–7pm',
              goal: 'book a delivery slot',
              goal_url: '/book',
            },
            prompt: 'Business: Petals & Stems, a florist.',
          },
        ],
      }),
    }),
  );
}

test('demo-chat modal opens, posts to /demo/chat, renders assistant reply', async ({ page }) => {
  await stubDemoPersonas(page);
  // Stub the AI call so the test doesn't need a Workers AI binding to work
  // and so the assertion is on a deterministic string.
  let postedBody: unknown = null;
  await page.route('**/demo/chat', async (route) => {
    postedBody = route.request().postDataJSON();
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ reply: 'I cover WhatsApp, Instagram, Discord, and email.' }),
    });
  });

  await page.goto('/');
  await page.locator('#demo-chat-hint-text').click();

  const dialog = page.getByRole('dialog', { name: /live demo/i });
  await expect(dialog).toBeVisible();
  // Default-persona greeting shows on open.
  await expect(dialog.getByText(/i'm Concierge/i)).toBeVisible();

  await dialog.getByRole('textbox').fill('what channels do you cover?');
  await dialog.getByRole('button', { name: 'Send' }).click();

  await expect(dialog.getByText(/I cover WhatsApp, Instagram, Discord, and email\./)).toBeVisible();

  // The wire format includes the persona slug and starts the message
  // history at the first user turn — the client-side greeting is
  // display-only and gets stripped before POST.
  expect(postedBody).toMatchObject({
    persona: 'concierge',
    messages: [{ role: 'user', content: 'what channels do you cover?' }],
  });
});

test('demo-chat persona picker swaps greeting and persona slug on the wire', async ({ page }) => {
  await stubDemoPersonas(page);
  let lastBody: any = null;
  await page.route('**/demo/chat', async (route) => {
    lastBody = route.request().postDataJSON();
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ reply: 'sure thing 🌸' }),
    });
  });

  await page.goto('/');
  await page.locator('#demo-chat-hint-text').click();
  const dialog = page.getByRole('dialog', { name: /live demo/i });
  await expect(dialog).toBeVisible();

  // Default greeting is the Concierge one.
  await expect(dialog.getByText(/i'm Concierge/i)).toBeVisible();

  // Switch to the florist preset; greeting changes.
  const select = dialog.locator('[data-testid="demo-chat-persona"]');
  await select.selectOption('friendly_florist');
  await expect(dialog.getByText(/welcome to the shop/i)).toBeVisible();
  await expect(dialog.getByText(/i'm Concierge/i)).toHaveCount(0);

  await dialog.getByRole('textbox').fill('do you ship next-day?');
  await dialog.getByRole('button', { name: 'Send' }).click();
  await expect(dialog.getByText(/sure thing/i)).toBeVisible();

  expect(lastBody).toMatchObject({
    persona: 'friendly_florist',
    messages: [{ role: 'user', content: 'do you ship next-day?' }],
  });
});

test('demo-chat view-prompt toggle reveals the active persona prompt', async ({ page }) => {
  await stubDemoPersonas(page);
  await page.goto('/');
  await page.locator('#demo-chat-hint-text').click();
  const dialog = page.getByRole('dialog', { name: /live demo/i });

  const toggle = dialog.getByRole('button', { name: /view system prompt/i });
  await expect(toggle).toHaveAttribute('aria-expanded', 'false');
  await toggle.click();
  await expect(dialog.getByRole('button', { name: /hide system prompt/i })).toHaveAttribute(
    'aria-expanded',
    'true',
  );
  // Three-section envelope: preamble (fixed, never editable), middle
  // (current persona's prompt), postamble (fixed safety rules).
  const panel = dialog.locator('#demo-chat-prompt-panel');
  await expect(panel).toContainText(/automated reply assistant for a small business/i);
  await expect(panel).toContainText(/WhatsApp Business/);
  await expect(panel).toContainText(/House rules/i);
  // Both fixed bookends are rendered as their own pre blocks.
  await expect(panel.locator('.chat-prompt-fixed')).toHaveCount(2);
  await expect(panel.locator('.chat-prompt-middle')).toHaveCount(1);
});

test('clicking the hero headline also opens the chat modal', async ({ page }) => {
  await stubDemoPersonas(page);
  await page.route('**/demo/chat', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ reply: 'ok' }),
    }),
  );
  await page.goto('/');
  await page.locator('#hero-headline').click();
  const dialog = page.getByRole('dialog', { name: /live demo/i });
  await expect(dialog).toBeVisible();
});

test('hero hint informs users the headline is clickable', async ({ page }) => {
  await page.goto('/');
  // The hint text is in the DOM as an aria-describedby target so screen
  // readers see it; sighted users get the tooltip via the initial pulse
  // and on hover.
  const headline = page.locator('#hero-headline');
  await expect(headline).toHaveAttribute('aria-describedby', 'demo-chat-hint-text');
  await expect(page.locator('#demo-chat-hint-text')).toContainText(/click to chat/i);
});

test('demo-chat surfaces the rate-limit message on 429', async ({ page }) => {
  await stubDemoPersonas(page);
  await page.route('**/demo/chat', (route) =>
    route.fulfill({
      status: 429,
      contentType: 'application/json',
      body: JSON.stringify({ error: 'rl' }),
    }),
  );

  await page.goto('/');
  await page.locator('#demo-chat-hint-text').click();
  const dialog = page.getByRole('dialog', { name: /live demo/i });
  await dialog.getByRole('textbox').fill('hello?');
  await dialog.getByRole('button', { name: 'Send' }).click();
  await expect(dialog.getByText(/quite a few messages/i)).toBeVisible();
});

test('demo-chat shows business goal row in card', async ({ page }) => {
  await stubDemoPersonas(page);
  await page.goto('/');
  await page.locator('#demo-chat-hint-text').click();
  const dialog = page.getByRole('dialog', { name: /live demo/i });
  // Switch to a builder persona so the business card renders.
  const select = dialog.locator('[data-testid="demo-chat-persona"]');
  await select.selectOption('friendly_florist');
  // Goal row text comes from the stub payload's business.goal field.
  await expect(dialog.locator('.chat-business-card')).toContainText(
    /book a delivery slot/i,
  );
});

test('demo-chat flips into holding pattern when server returns handoff:true', async ({
  page,
}) => {
  await stubDemoPersonas(page);
  // First send: server flags handoff. Second send: client must echo
  // `handoff: true` in the request body and the server's holding-pattern
  // reply gets shown; the chip stays visible.
  let firstBody: any = null;
  let secondBody: any = null;
  let calls = 0;
  await page.route('**/demo/chat', async (route) => {
    calls += 1;
    const body = route.request().postDataJSON();
    if (calls === 1) {
      firstBody = body;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          reply: 'I have flagged a teammate. They will be in touch.',
          handoff: true,
        }),
      });
    } else {
      secondBody = body;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          reply: 'Still holding for a teammate — they will reply directly.',
          handoff: true,
        }),
      });
    }
  });

  await page.goto('/');
  await page.locator('#demo-chat-hint-text').click();
  const dialog = page.getByRole('dialog', { name: /live demo/i });
  // Switch to florist so we're not on Concierge (handoff applies to
  // builder personas in real use; the demo accepts it on any).
  await dialog.locator('[data-testid="demo-chat-persona"]').selectOption('friendly_florist');

  await dialog.getByRole('textbox').fill('actually I want a refund');
  await dialog.getByRole('button', { name: 'Send' }).click();
  await expect(dialog.locator('.chat-handoff-chip')).toBeVisible();
  expect(firstBody).toMatchObject({ handoff: false });

  // Pressing Enter on the input dispatches the same submit handler
  // and avoids button-visibility flakes when the modal scrolls on
  // mobile. The handler is `@submit.prevent="send()"` so either path
  // exercises the same code.
  const input = dialog.getByRole('textbox');
  await input.fill('are you still there?');
  await input.press('Enter');
  await expect(dialog.getByText(/Still holding for a teammate/i)).toBeVisible();
  // Client must echo handoff:true on subsequent sends.
  expect(secondBody).toMatchObject({ handoff: true });
});
