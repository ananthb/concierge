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

test('demo-chat modal opens, posts to /demo/chat, renders assistant reply', async ({ page }) => {
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
  // The Concierge prompt mentions WhatsApp Business.
  await expect(dialog.locator('#demo-chat-prompt-panel')).toContainText(/WhatsApp Business/);
});

test('clicking the hero headline also opens the chat modal', async ({ page }) => {
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
