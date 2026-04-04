# Setup Guide

## Prerequisites

- A Cloudflare account with Workers, D1, and KV enabled
- A Meta Developer account with a Facebook App
- A Google Cloud project (for OAuth sign-in)
- Nix (for the development environment)

## 1. Clone and Build

```bash
git clone https://github.com/ananthb/concierge-worker
cd concierge-worker
nix develop
cargo test
```

## 2. Create Cloudflare Resources

```bash
# Create D1 database
wrangler d1 create concierge-worker

# Create KV namespace
wrangler kv namespace create CALENDARS_KV
```

Update the IDs in `wrangler.toml`.

## 3. Apply Database Migration

```bash
wrangler d1 execute concierge-worker --remote --file migrations/0001_create_schema.sql
```

## 4. Set Secrets

See [Configuration & Secrets](./configuration.md) for the full list.

## 5. Deploy

```bash
wrangler deploy
```

## 6. Configure Webhooks

After deploying, configure webhooks in the Meta Developer Console:

- **WhatsApp**: Callback URL `https://your-domain/webhook/whatsapp`, subscribe to `messages`
- **Instagram**: Callback URL `https://your-domain/webhook/instagram`, subscribe to `messages`

See [Facebook App Setup](./facebook-app-setup.md) for detailed instructions.
