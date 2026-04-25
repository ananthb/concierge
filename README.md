<p align="center">
  <img src="assets/logo.svg" width="100" height="100" alt="Concierge logo">
</p>

# Concierge

Automated customer messaging for small businesses. Auto-replies across WhatsApp, Instagram DMs, and email. Managed email subdomains on `cncg.email`. Unified Discord inbox for everything that needs a human.

[![Deploy to Cloudflare](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https://github.com/ananthb/concierge-worker)

**[Hosted Service](https://concierge.calculon.tech)** · **[Documentation](https://ananthb.github.io/concierge-worker/)**

## Hosted Service

Don't want to self-host? [concierge.calculon.tech](https://concierge.calculon.tech) runs this exact stack as a managed service. Sign up, connect your channels, and start auto-replying in minutes. 100 free replies every month.

## Features

- **WhatsApp Auto-Reply**: static or AI-powered replies to every message via Meta Business API
- **Instagram DM Auto-Reply**: connect your business account, reply automatically
- **Managed Email Subdomains**: each tenant gets `*.cncg.email` addresses with smart routing rules (glob patterns). Forward, drop, AI-draft, or relay to Discord. MX records provisioned automatically via Cloudflare API
- **Discord Relay**: unified inbox. Messages from any channel land in Discord with Reply/Approve/Drop buttons. Reply in Discord and it flows back to the customer
- **Lead Capture Forms**: embeddable phone number forms that trigger WhatsApp messages
- **Onboarding Wizard**: 5-step guided setup (business info, channels, notifications, replies, billing)
- **Notification Preferences**: configurable approval + digest delivery via Discord and/or Email with batching frequency
- **Management Panel**: Cloudflare Access-protected admin for tenant management, billing, audit log
- **Billing**: flat prepaid credits — **₹2 / $0.02 per AI reply**, 100 free every month. Static auto-replies are always free. Buy any quantity (slider, no tiers, no packs). Email subdomains are a separate ₹199/$2 monthly subscription, auto-provisioned
- **Privacy-first**: no message content stored. Metadata only. GDPR data deletion

## Deploy

Click the button above to deploy to Cloudflare Workers. See the [setup guide](https://ananthb.github.io/concierge-worker/setup.html) for configuration.

## Architecture

- [Cloudflare Workers](https://workers.cloudflare.com/) (Rust compiled to WebAssembly)
- [Cloudflare D1](https://developers.cloudflare.com/d1/) (SQLite for metadata logs and billing)
- [Cloudflare KV](https://developers.cloudflare.com/kv/) (account configs, sessions, billing state)
- [Cloudflare Workers AI](https://developers.cloudflare.com/workers-ai/) (AI auto-replies)
- [Cloudflare Email Routing](https://developers.cloudflare.com/email-routing/) (inbound email handling)
- [Cloudflare Email Service](https://developers.cloudflare.com/email-service/) (outbound delivery to arbitrary recipients via the `send_email` binding's structured API)
- [Cloudflare DNS API](https://developers.cloudflare.com/api/resources/dns/) (MX + A/AAAA record provisioning for tenant subdomains)
- Meta WhatsApp Business API + Instagram Graph API
- Discord Interactions API (slash commands + cross-channel relay)
- Razorpay (one-shot credit purchases + email subdomain subscriptions)

## Development

```bash
nix develop        # enter dev shell with all tools
cargo test         # run tests
wrangler dev       # local dev server
wrangler deploy    # deploy to Cloudflare
```

## License

[AGPL-3.0](LICENSE)
