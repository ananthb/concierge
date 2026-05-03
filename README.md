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
- **Persona Builder**: tenant-wide AI persona via guided builder (voice archetype, business name + type, goal, catch-phrases, off-topic boundaries, handoff conditions), curated archetype copied from a platform-managed D1 catalog, or raw prompt. Every change is run past a safety classifier asynchronously via Cloudflare Queues; AI replies stay blocked tenant-wide until the new prompt is approved
- **Prompt Envelope**: every AI reply is wrapped by a fixed preamble + postamble (defined in `src/prompt.rs`) that establishes the operating manual, jailbreak rails, and the universal handoff sentinel. Tenant content lives in the editable middle and never reaches the model alone
- **Conversation Sessions**: per-customer threads carry a stable `conversation_id`, recent message history, and any active handoff state. A configurable idle gap (default 6 h) wipes history and starts a fresh conversation; a configurable max-history-messages cap (default 20) bounds the multi-turn context fed to the AI
- **Human Handoff**: the model can flag a turn with `[[HANDOFF]]` (universal triggers in the postamble + tenant-specific conditions in the persona). The pipeline strips the token, switches to a holding-pattern voice for follow-ups within the cooldown (default 60 min), then goes silent. The tenant is paged once via the existing approval-notification channels (Discord embed and/or immediate email)
- **Live Demo Chat**: the public welcome page hosts an interactive demo where visitors roleplay as a customer of a sample business and see the AI reply in real time. Persona picker pulls from the safety-approved D1 archetype catalog; a "View prompt" panel reveals the exact envelope being sent to the model. Real customer messages still arrive on WhatsApp / IG / email / Discord — never on this chat box
- **Managed Email Subdomains**: each tenant gets `*.cncg.email` addresses with smart routing rules (glob patterns). Forward, drop, AI-draft, or relay to Discord. MX records provisioned automatically via Cloudflare API
- **Discord Relay**: unified inbox. Messages from any channel land in Discord with Reply/Approve/Drop buttons. Reply in Discord and it flows back to the customer
- **Lead Capture Forms**: embeddable phone number forms that trigger WhatsApp messages
- **Onboarding Wizard**: guided setup (business info, channels, notifications, persona archetype, billing)
- **Notification Preferences**: configurable approval + digest delivery via Discord and/or Email with batching frequency
- **Localized**: per-tenant BCP-47 locale (`en-IN` and `en-US` shipped) drives Indian-vs-Western number grouping (₹1,00,000 vs $100,000) via icu4x; translation backbone uses fluent-rs FTL files for drop-in new languages. AI-generated reply content stays English regardless of UI locale
- **Management Panel**: Cloudflare Access-protected admin for tenant management, billing, audit log
- **Billing**: flat prepaid credits (₹0.10 / $0.001 per AI reply, 100 included every month). Static auto-replies don't consume credits. Buy any quantity (slider, no tiers, no packs). Reply-email subscription: 5 addresses per ₹99 / $1 per month. All prices live in `global_settings` and are editable from the management panel
- **Privacy-first**: no message content stored. Metadata only. GDPR data deletion

## Deploy

See the **[Deploy guide](https://ananthb.github.io/concierge/deployment.html)** for step-by-step instructions on forking and deploying your own instance to Cloudflare.

CI/CD is handled by **Cloudflare Builds** (Workers CI), which builds and deploys directly from this repo without needing GitHub Actions or Nix.

To wire up your fork:

1. In the Cloudflare dashboard, create a Worker named (e.g.) `concierge` and connect this repo under **Settings → Builds**.
   - **Build command:** leave default (CF Builds runs `npm install` from `package.json`)
   - **Deploy command:** `npm run deploy` (defined in `package.json` — installs `worker-build` then runs `wrangler deploy`)
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
