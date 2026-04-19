<p align="center">
  <img src="assets/logo.svg" width="100" height="100" alt="Concierge logo">
</p>

# Concierge

Automated customer engagement for small businesses. Auto-replies across WhatsApp, Instagram DMs, and email — with a unified Discord inbox for everything that needs a human.

**[Documentation](https://ananthb.github.io/concierge-worker/)** · **[Pricing](https://concierge.calculon.tech/pricing)**

## Features

- **WhatsApp Auto-Reply** — static or AI-powered replies to every message
- **Instagram DM Auto-Reply** — connect your business account, reply automatically
- **Email Routing** — catch-all email with routing rules (glob patterns). Forward, drop, AI-draft, or relay to Discord
- **Discord Relay** — unified inbox. Messages from any channel land in Discord with Reply/Approve/Drop buttons. Reply in Discord → flows back to the customer
- **Lead Capture Forms** — embeddable phone number forms that trigger WhatsApp messages
- **Onboarding Wizard** — 6-step guided setup (channels, admin, voice persona, canned replies)
- **Management Panel** — Cloudflare Access-protected admin for tenant management, billing, audit log
- **Billing** — prepaid reply credits. 100 free/month, then volume-priced packs via Razorpay
- **Privacy-first** — no message content stored. Metadata only. GDPR data deletion.

## Architecture

- [Cloudflare Workers](https://workers.cloudflare.com/) — Rust compiled to WebAssembly
- [Cloudflare D1](https://developers.cloudflare.com/d1/) — SQLite for metadata logs and billing
- [Cloudflare KV](https://developers.cloudflare.com/kv/) — account configs, sessions, billing state
- [Cloudflare Workers AI](https://developers.cloudflare.com/workers-ai/) — AI auto-replies (Llama 3.1 8B)
- [Cloudflare Email Routing](https://developers.cloudflare.com/email-routing/) — catch-all email handling
- Meta WhatsApp Business API + Instagram Graph API
- Discord Interactions API — slash commands + cross-channel relay
- Razorpay — payment processing for credit packs

## Quick Start

```bash
# Clone and enter dev shell
git clone https://github.com/ananthb/concierge-worker
cd concierge-worker
nix develop

# Create D1 database and apply schema
wrangler d1 create concierge-worker
wrangler d1 execute concierge-worker --remote --file=migrations/0001_create_schema.sql

# Set secrets
wrangler secret put ENCRYPTION_KEY
wrangler secret put GOOGLE_OAUTH_CLIENT_ID
wrangler secret put GOOGLE_OAUTH_CLIENT_SECRET
wrangler secret put META_APP_ID
wrangler secret put META_APP_SECRET
wrangler secret put WHATSAPP_ACCESS_TOKEN
wrangler secret put WHATSAPP_VERIFY_TOKEN

# Deploy
wrangler deploy
```

See the [setup guide](https://ananthb.github.io/concierge-worker/setup.html) for full instructions.

## License

[AGPL-3.0](LICENSE)
