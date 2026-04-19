<p align="center">
  <img src="assets/logo.svg" width="100" height="100" alt="Concierge logo">
</p>

# Concierge

Automated customer engagement for small businesses. Auto-replies across WhatsApp, Instagram DMs, and email, with a unified Discord inbox for everything that needs a human.

[![Deploy to Cloudflare](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https://github.com/ananthb/concierge-worker)

**[Live Demo](https://concierge.calculon.tech)** · **[Documentation](https://ananthb.github.io/concierge-worker/)** · **[Pricing](https://concierge.calculon.tech/pricing)**

## Features

- **WhatsApp Auto-Reply**: static or AI-powered replies to every message
- **Instagram DM Auto-Reply**: connect your business account, reply automatically
- **Email Routing**: catch-all email with routing rules (glob patterns). Forward, drop, AI-draft, or relay to Discord
- **Discord Relay**: unified inbox. Messages from any channel land in Discord with Reply/Approve/Drop buttons. Reply in Discord and it flows back to the customer
- **Lead Capture Forms**: embeddable phone number forms that trigger WhatsApp messages
- **Onboarding Wizard**: 6-step guided setup (channels, admin, voice persona, canned replies)
- **Management Panel**: Cloudflare Access-protected admin for tenant management, billing, audit log
- **Billing**: prepaid reply credits. 100 free/month, then volume-priced packs via Razorpay. Purchased credits never expire
- **Privacy-first**: no message content stored. Metadata only. GDPR data deletion

## Deploy

Click the button above to deploy to Cloudflare Workers. The deploy flow will fork the repo and provision the Worker, D1 database, and KV namespace automatically.

After deploy, you still need to:

1. **Run the D1 migration** to create tables and seed credit packs:
   ```bash
   wrangler d1 execute concierge-worker --remote --file=migrations/0001_create_schema.sql
   ```

2. **Set secrets** (at minimum for Google login):
   ```bash
   wrangler secret put ENCRYPTION_KEY          # openssl rand -hex 32
   wrangler secret put GOOGLE_OAUTH_CLIENT_ID
   wrangler secret put GOOGLE_OAUTH_CLIENT_SECRET
   ```

3. **Configure channels** as needed:
   ```bash
   # Meta (WhatsApp + Instagram)
   wrangler secret put META_APP_ID
   wrangler secret put META_APP_SECRET
   wrangler secret put WHATSAPP_ACCESS_TOKEN
   wrangler secret put WHATSAPP_VERIFY_TOKEN
   wrangler secret put INSTAGRAM_VERIFY_TOKEN

   # Discord
   wrangler secret put DISCORD_PUBLIC_KEY
   wrangler secret put DISCORD_APPLICATION_ID
   wrangler secret put DISCORD_BOT_TOKEN

   # Razorpay (billing)
   wrangler secret put RAZORPAY_KEY_ID
   wrangler secret put RAZORPAY_KEY_SECRET
   wrangler secret put RAZORPAY_WEBHOOK_SECRET
   ```

4. **Set environment variables** in `wrangler.toml`:
   - `CF_ACCESS_TEAM` and `CF_ACCESS_AUD` for the management panel
   - `WHATSAPP_WABA_ID` and `WHATSAPP_SIGNUP_CONFIG_ID` for WhatsApp

See the [setup guide](https://ananthb.github.io/concierge-worker/setup.html) for full instructions.

## Architecture

- [Cloudflare Workers](https://workers.cloudflare.com/) (Rust compiled to WebAssembly)
- [Cloudflare D1](https://developers.cloudflare.com/d1/) (SQLite for metadata logs and billing)
- [Cloudflare KV](https://developers.cloudflare.com/kv/) (account configs, sessions, billing state)
- [Cloudflare Workers AI](https://developers.cloudflare.com/workers-ai/) (AI auto-replies via Llama 3.1 8B)
- [Cloudflare Email Routing](https://developers.cloudflare.com/email-routing/) (catch-all email handling)
- Meta WhatsApp Business API + Instagram Graph API
- Discord Interactions API (slash commands + cross-channel relay)
- Razorpay (payment processing for credit packs)

## Development

```bash
nix develop        # enter dev shell
cargo test         # run tests
wrangler dev       # local dev server
wrangler deploy    # deploy to Cloudflare
```

## License

[AGPL-3.0](LICENSE)
