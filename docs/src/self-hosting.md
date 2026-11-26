# Self-hosting

`replybots` runs on Cloudflare Workers with D1, Workers AI, and Cloudflare Email Routing (`send_email`).

## 1. Prerequisites

- Cloudflare account with Workers, D1, and Workers AI enabled
- Meta App with WhatsApp Business API & Embedded Signup configured
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

Plaintext variables (`PUBLIC_BASE_URL`, `META_APP_ID`, `WHATSAPP_SIGNUP_CONFIG_ID`, `RAZORPAY_KEY_ID`, `EMAIL_DOMAIN`, `WHATSAPP_WABA_ID`) can be set **either**:

- In the Cloudflare Dashboard under **Workers & Pages → replybots → Settings → Variables and Secrets** (preserved across deploys by `keep_vars = true` in `wrangler.toml`), **or**
- As **Workers Builds** environment variables (forwarded via `--var` when non-empty by `npm run deploy` / `./scripts/deploy.sh`).

Environment variables (`[vars]`):

- `PUBLIC_BASE_URL` — primary site URL, e.g. `https://replybots.io`
- `META_APP_ID` — Meta App ID for WhatsApp Embedded Signup
- `WHATSAPP_SIGNUP_CONFIG_ID` — Meta Embedded Signup configuration ID
- `RAZORPAY_KEY_ID` — Razorpay key ID
- `EMAIL_DOMAIN` — domain for outbound handoff alert emails, e.g. `replybots.io` (optional)
- `WHATSAPP_WABA_ID` — fallback WhatsApp Business Account ID (optional; Embedded Signup supplies each tenant's own WABA ID)

Set secrets via `wrangler secret put` (or in the Cloudflare Dashboard):

- `WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_VERIFY_TOKEN`
- `META_APP_SECRET`
- `RAZORPAY_KEY_SECRET`, `RAZORPAY_WEBHOOK_SECRET`

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

Set `DEV_MODE=1` to stub Workers AI / safety checks and enable `POST /auth/dev-login` without Meta OAuth:

```sh
node scripts/test-server.mjs
```

`scripts/test-server.mjs` applies `migrations/` to local D1 and starts `wrangler dev --local` with `DEV_MODE=1`.
