# Setup Guide (Manual)

Deploy your own Concierge instance manually using the Wrangler CLI.

## Prerequisites
*   A [Cloudflare account](https://dash.cloudflare.com).
*   [Nix](https://nixos.org/download) installed.

## 1. Clone and enter the dev shell
```bash
git clone https://github.com/ananthb/concierge-worker.git
cd concierge-worker
nix develop
```

## 2. Infrastructure Setup
1.  **Create KV**: `wrangler kv namespace create KV` and add the ID to `wrangler.toml`.
2.  **Create D1**: `wrangler d1 create concierge-worker` and add the binding to `wrangler.toml`.
3.  **Apply Migrations**: `wrangler d1 execute concierge-worker --remote --file migrations/0001_create_schema.sql`.

## 3. Deploy
```bash
wrangler deploy
```

## 4. Channel Setup
Follow the detailed setup guides for each channel:
*   [WhatsApp Setup](whatsapp.html)
*   [Instagram Setup](instagram.html)
*   [Email Routing Setup](email-routing.html)
*   [Discord Setup](discord.html)
