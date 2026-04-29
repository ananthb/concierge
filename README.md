<p align="center">
  <img src="assets/logo.svg" width="100" height="100" alt="Concierge logo">
</p>

# Concierge

Automated customer messaging for small businesses. Auto-replies across WhatsApp, Instagram DMs, and email. Managed email subdomains on `cncg.email`. Unified Discord inbox for everything that needs a human.

[![Deploy to Cloudflare](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https://github.com/ananthb/concierge)

**[Hosted Service](https://concierge.calculon.tech)** · **[Documentation](https://ananthb.github.io/concierge/)**

## Hosted Service

Don't want to self-host? [concierge.calculon.tech](https://concierge.calculon.tech) runs this exact stack as a managed service. Sign up, connect your channels, and start auto-replying in minutes. 100 free replies every month.

## Features

- **WhatsApp Auto-Reply**: rule-routed canned or AI replies via Meta Business API
- **Instagram DM Auto-Reply**: connect your business account, reply automatically
- **Reply Rules**: per-channel ordered rules (keyword matchers and embedding-based intent matchers), each routing to canned text or an AI prompt; mandatory default fallback per channel
- **Persona Builder**: tenant-wide AI persona with three modes: curated preset (Friendly Florist / Professional Salon / Playful Cafe / Old-school Clinic), guided builder (tone, catch-phrases, off-topic boundaries), or raw prompt. Every change is run past a safety classifier asynchronously via Cloudflare Queues
- **Managed Email Subdomains**: each tenant gets `*.cncg.email` addresses with smart routing rules (glob patterns). Forward, drop, AI-draft, or relay to Discord. MX records provisioned automatically via Cloudflare API
- **Discord Relay**: unified inbox. Messages from any channel land in Discord with Reply/Approve/Drop buttons. Reply in Discord and it flows back to the customer
- **Lead Capture Forms**: embeddable phone number forms that trigger WhatsApp messages
- **Onboarding Wizard**: 5-step guided setup (business info, channels, notifications, persona preset, billing)
- **Notification Preferences**: configurable approval + digest delivery via Discord and/or Email with batching frequency
- **Localized**: per-tenant BCP-47 locale (`en-IN` and `en-US` shipped) drives Indian-vs-Western number grouping (₹1,00,000 vs $100,000) via icu4x; translation backbone uses fluent-rs FTL files for drop-in new languages. AI-generated reply content stays English regardless of UI locale
- **Management Panel**: Cloudflare Access-protected admin for tenant management, billing, audit log
- **Billing**: flat prepaid credits (₹2 / $0.02 per AI reply, 100 free every month). Static auto-replies are always free. Buy any quantity (slider, no tiers, no packs). One email address is free per account; extras are ₹99 / $1 each, one-time
- **Privacy-first**: no message content stored. Metadata only. GDPR data deletion

## Deploy

See the **[Deploy guide](https://ananthb.github.io/concierge/deployment.html)** for step-by-step instructions on forking and deploying your own instance to Cloudflare.

CI/CD is handled by **Cloudflare Builds** (Workers CI), which builds and deploys directly from this repo without needing GitHub Actions or Nix.

To wire up your fork:

1. In the Cloudflare dashboard, create a Worker named (e.g.) `concierge` and connect this repo under **Settings → Builds**.
   - **Build command:** `bash scripts/cf-build.sh` (installs rustup + worker-build; CF Builds doesn't ship Rust)
   - **Deploy command:** `. "$HOME/.cargo/env" && npx wrangler deploy` (sources the cargo env so wrangler finds worker-build)
2. Bind a D1 database (`DB`), KV namespace (`KV`), Workers AI (`AI`), Email Routing send-binding (`EMAIL`), Durable Objects (`REPLY_BUFFER` → `ReplyBufferDO`, `APPROVALS_DO` → `ApprovalsDO`), and Queues (`SAFETY_QUEUE` producer + `concierge-safety` / `concierge-safety-dlq` consumers) under **Settings → Bindings**. Names must match the `binding` values in [`wrangler.toml`](wrangler.toml).
3. Set runtime variables and secrets under **Settings → Variables and Secrets** — full list is documented at the bottom of [`wrangler.toml`](wrangler.toml).
4. Push to `main`. Cloudflare Builds runs the build command, then `wrangler deploy` — which picks up `[build] command = "worker-build --release"` from `wrangler.toml` to compile the Rust crate to WASM.

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
nix develop        # enter dev shell with all tools (Nix-only; CI does not use Nix)
cargo test         # run tests
wrangler dev       # local dev server
wrangler deploy    # deploy to Cloudflare
```

Nix is for local convenience only — Cloudflare Builds installs the same toolchain via rustup, which reads the channel from [`rust-toolchain.toml`](rust-toolchain.toml).

## License

[AGPL-3.0](LICENSE)
