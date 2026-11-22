<p align="center">
  <img src="assets/logo.svg" width="100" height="100" alt="Concierge logo">
</p>

# Concierge

Automatic WhatsApp replies for small businesses. Answers in your voice with the built-in core persona, and hands the conversation to a person the moment it shouldn't decide alone.

[![Deploy to Cloudflare](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https://github.com/ananthb/concierge)

**[Documentation](https://ananthb.github.io/concierge/)**

## Features

- **WhatsApp auto-reply**: inbound messages get a canned message or an AI reply via the Meta Business API and Embedded Signup. You keep your number; existing threads continue seamlessly.
- **Single core persona**: built-in small-business core persona designed to answer briefly and hand off quickly. No persona builder, archetypes catalog, or custom prompts.
- **Prompt Guard & Safety Vetting**: incoming messages are screened by `@cf/meta/prompt-guard-2-86m` for prompt injections. Business profiles and canned replies are vetted asynchronously via Cloudflare Queues and `@cf/meta/llama-guard-3-8b`.
- **Risk gate**: a draft that quotes an unauthorised price, makes a commitment, or strays past boundaries is withheld. The customer gets a holding sentence, the conversation enters handoff, and an alert email is sent to the tenant.
- **Human handoff**: the model can request human intervention via `[[HANDOFF]]`. Follow-ups enter a holding-pattern cooldown before going silent.
- **Prepaid Razorpay billing**: 1 credit = 1 reply (for both static and AI replies). No free trials, no card-verification gates. Default pricing: ₹0.10/reply INR, $0.001/reply USD (min 100 / max 100,000 credits). Authoritative webhook `POST /webhook/razorpay` with fail-closed HMAC-SHA256 validation.
- **Meta App Review compliance**: Facebook JS SDK Embedded Signup (`FB.login` + `WA_EMBEDDED_SIGNUP`), `tenants.facebook_id` stored for `POST /data-deletion`, metadata-only D1 `messages` logging, and prerendered `/privacy` & `/terms`.
- **Live demo**: landing page hosts a live chat backed by 3 static sample SMB fixtures in Rust.
- **Local dev**: `DEV_MODE=1` enables `POST /auth/dev-login` and local AI/safety stubs.

## Architecture

- **[Cloudflare Workers](https://workers.cloudflare.com/)** — Rust compiled to WebAssembly, serving a JSON API
- **[Elm](https://elm-lang.org/)** — Frontend, compiled to a single `public/app.js`
- **[Cloudflare D1](https://developers.cloudflare.com/d1/)** — SQLite for metadata logs and billing
- **[Cloudflare KV](https://developers.cloudflare.com/kv/)** — Account configs, sessions, conversation state
- **[Cloudflare Workers AI](https://workers.cloudflare.com/)** — Reply generation (`@cf/meta/llama-4-scout-17b-16e-instruct`), prompt injection detection (`@cf/meta/prompt-guard-2-86m`), and safety vetting (`@cf/meta/llama-guard-3-8b`)
- **[Cloudflare Queues](https://developers.cloudflare.com/queues/)** — Asynchronous content safety vetting
- **[Cloudflare Durable Objects](https://developers.cloudflare.com/durable-objects/)** — Reply buffer for message batching
- Meta WhatsApp Business API & Embedded Signup
- **[Razorpay](https://razorpay.com/)** — Prepaid credits payments

## Development

```sh
direnv allow          # or: nix develop
npm ci
npm run build:frontend   # compile Elm to public/app.js
DEV_MODE=1 wrangler dev  # local worker + migrations + dev login
```

Then open http://localhost:8787.

## Deploy

See the **[Deploy guide](https://ananthb.github.io/concierge/deployment.html)**.

## License

[AGPL-3.0](LICENSE).
