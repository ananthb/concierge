<p align="center">
  <img src="assets/logo.svg" width="100" height="100" alt="Concierge logo">
</p>

# Concierge

Messaging automation for small businesses. WhatsApp auto-replies, Instagram DM auto-replies, and embeddable lead capture forms — zero effort.

**[Documentation](https://ananthb.github.io/concierge-worker/)**

## What It Does

- **WhatsApp Auto-Reply** — Incoming message? Reply instantly with a static message or an AI-generated response
- **Instagram DM Auto-Reply** — Connect your Instagram business account, auto-reply to every DM
- **Lead Capture Forms** — Embed a phone number form on any site; submissions trigger a WhatsApp message

## Built With

- [Cloudflare Workers](https://workers.cloudflare.com/) (Rust + WebAssembly)
- Meta WhatsApp Business API
- Facebook Login + Instagram Graph API
- Cloudflare AI, D1, KV

## Building Documentation

```bash
nix develop
./docs/extract-docs.sh
mdbook serve docs
```

## License

AGPL-3.0
