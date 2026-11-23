# Self-hosting

`replybots` runs on Cloudflare Workers with D1, Workers AI, and Cloudflare Email Routing (`send_email`).

## 1. Prerequisites

- Cloudflare account with Workers, D1, and Workers AI enabled
- Meta App with WhatsApp Business API & Embedded Signup configured
- Google OAuth client (for tenant sign-in)
- Razorpay account (for prepaid credit top-ups)

## 2. Create the D1 Database & Apply Migrations

```sh
wrangler d1 create replybots
```

Update `database_id` in `wrangler.toml`, then apply the schema migration:

```sh
wrangler d1 execute replybots --remote --file migrations/0001_create_schema.sql
```

## 3. Configure Environment Variables & Secrets

Set runtime variables at deploy time (or in `wrangler.toml`):

- `PUBLIC_BASE_URL` — e.g. `https://replybots.io`
- `EMAIL_DOMAIN` — domain for outbound handoff alert emails (e.g. `replybots.io`)
- `WHATSAPP_WABA_ID` — fallback WhatsApp Business Account ID
- `WHATSAPP_SIGNUP_CONFIG_ID` — Meta Embedded Signup configuration ID

Set secrets via `wrangler secret put`:

- `GOOGLE_OAUTH_CLIENT_ID`, `GOOGLE_OAUTH_CLIENT_SECRET`
- `WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_VERIFY_TOKEN`
- `META_APP_ID`, `META_APP_SECRET`
- `RAZORPAY_KEY_ID`, `RAZORPAY_KEY_SECRET`, `RAZORPAY_WEBHOOK_SECRET`

See [Bindings & Environment](reference/bindings-and-env.md) for the full list extracted from `wrangler.toml` and `src/`.

## 4. Configure Webhooks

- **Meta WhatsApp webhook**: point to `<PUBLIC_BASE_URL>/webhook/whatsapp` with your `WHATSAPP_VERIFY_TOKEN`.
- **Meta data deletion callback**: point to `<PUBLIC_BASE_URL>/data-deletion`.
- **Razorpay webhook**: point to `<PUBLIC_BASE_URL>/webhook/razorpay` using `RAZORPAY_WEBHOOK_SECRET`.

## 5. Build & Deploy

```sh
npm ci
npm run build:frontend
wrangler deploy
```

## Local Development

Set `DEV_MODE=1` to stub Workers AI / safety checks and enable `POST /auth/dev-login` without Google OAuth:

```sh
node scripts/test-server.mjs
```

`scripts/test-server.mjs` applies `migrations/` to local D1 and starts `wrangler dev --local` with `DEV_MODE=1`.
