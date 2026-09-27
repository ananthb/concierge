<p align="center">
  <img src="assets/logo.svg" width="100" height="100" alt="Concierge logo">
</p>

# Concierge

Automatic WhatsApp replies for small businesses. Answers in your voice, and hands the conversation to a person the moment it shouldn't decide alone.

[![Deploy to Cloudflare](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https://github.com/ananthb/concierge)

**[Hosted Service](https://concierge.calculon.tech)** · **[Documentation](https://ananthb.github.io/concierge/)**

## Hosted Service

Don't want to self-host? [concierge.calculon.tech](https://concierge.calculon.tech) runs this exact stack as a managed service. Sign up, connect your number, and start auto-replying in minutes.

## Features

- **WhatsApp auto-reply**: inbound messages get a canned message or an AI reply, via the Meta Business API. You keep your number; nothing changes for the people already messaging you.
- **Persona builder**: a tenant-wide AI voice, built from a guided form over a platform-curated archetype (voice, business name and type, goal, catch-phrases, off-topic boundaries, handoff conditions) or written as a raw prompt. Every change is run past a safety classifier asynchronously via Cloudflare Queues; AI replies stay blocked tenant-wide until the new prompt is approved.
- **Prompt envelope**: every AI reply is wrapped by a fixed preamble + postamble (`src/prompt.rs`) that establishes the operating manual, jailbreak rails, and the universal handoff sentinel. Tenant content lives in the editable middle and never reaches the model alone. All three parts are visible in the UI.
- **Risk gate**: a draft that quotes a price, makes a commitment, comes out an odd length, or strays past the persona's stated boundaries is never sent. The customer gets a holding sentence, the conversation enters handoff, and the tenant is emailed with the reason. Always on; nothing to configure.
- **Human handoff**: the model can also ask for a person itself by emitting `[[HANDOFF]]`. Either way the pipeline switches to a holding-pattern voice for follow-ups within the cooldown (default 60 min), then goes silent, and pages the tenant once by email.
- **Conversation sessions**: per-customer threads carry a stable `conversation_id`, recent history, and any active handoff state. A configurable idle gap (default 6 h) wipes history and starts a fresh conversation; a max-history cap (default 20) bounds the multi-turn context.
- **Reply batching**: a configurable wait (default 5 s) after the latest inbound message lets customers finish typing, so a burst of three messages gets one considered answer instead of three.
- **Interactive demo**: the landing page hosts a live chat where visitors roleplay as a customer of a sample business. "View the prompt" reveals the exact middle being sent to the model. Personas come from the safety-approved archetype catalog. Real customer messages still arrive on WhatsApp — never on this chat box.
- **Onboarding wizard**: four steps — business details, connect WhatsApp, choose a voice, go live. The voice is editable afterwards from the dashboard, in the same guided form or as a raw prompt, with a preview of what it composes to.
- **Billing**: flat prepaid credits (₹0.10 / $0.001 per AI reply). Canned replies are free. Buy any quantity — no tiers, no packs. All prices live in `global_settings` and are editable through the operator API.
- **Metadata-only logging**: message bodies pass through the Worker but are never written to durable storage. Only channel, direction, sender, recipient, and timestamp are persisted. A `/data-deletion` endpoint wipes the metadata that is stored.

Instagram DMs, the Discord relay and inbound email were part of earlier revisions and were cut to get to a launchable surface. The pipeline still dispatches on a `Channel` enum, so adding one back is a variant plus an arm rather than a re-plumb.

## Architecture

- [Cloudflare Workers](https://workers.cloudflare.com/) — Rust compiled to WebAssembly, serving a JSON API
- [Elm](https://elm-lang.org/) — the frontend, compiled to a single `public/app.js`
- [Cloudflare D1](https://developers.cloudflare.com/d1/) — SQLite for metadata logs, billing, the archetype catalog
- [Cloudflare KV](https://developers.cloudflare.com/kv/) — account configs, sessions, conversation state
- [Cloudflare Workers AI](https://developers.cloudflare.com/workers-ai/) — reply generation and the safety classifier
- [Cloudflare Queues](https://developers.cloudflare.com/queues/) — asynchronous persona safety checks
- [Cloudflare Durable Objects](https://developers.cloudflare.com/durable-objects/) — the reply buffer that batches quick-fire messages
- [Cloudflare Email Service](https://developers.cloudflare.com/email-service/) — outbound handoff notifications
- Meta WhatsApp Business API
- [Razorpay](https://razorpay.com/) — payments

### How the two halves meet

The Worker owns `/api/*`, the webhooks, and the OAuth redirects. Everything else gets the SPA shell (`src/shell.rs`), and the Elm app takes over.

Auth is a session cookie, and only a session cookie. The SPA and the Worker are always on the same origin, so the `HttpOnly; Secure; SameSite=Lax` cookie set by `/auth/callback` rides along on every request — there is no token to store or attach. State-changing calls must also carry `X-Concierge-Request`, which a cross-origin caller cannot set without a preflight the Worker never grants; that pair replaces the CSRF token the old HTML forms posted.

**The marketing pages are prerendered to static HTML.** `/`, `/pricing`, `/features`, `/terms` and `/privacy` are snapshotted into `public/` by `npm run prerender` and served straight off Cloudflare's asset handler — the Worker isn't invoked for them, so they cost no wasm cold start and a crawler that never runs a script still reads them. Everything else is Worker-rendered from `src/shell.rs`, which is also the shell the prerenderer snapshots.

Elm doesn't hydrate, so `public/boot.js` clears the body before `Elm.Main.init`: handed a foreign tree, `Browser.application` renders without taking ownership and the page ends up looking perfect while being completely inert. Rates and other live values are deliberately *not* captured — the prerenderer refuses `/api/bootstrap`, so pages render their copy and the numbers arrive from the API. Baking them in would go stale the moment pricing changed.

The output is committed because Cloudflare Builds has no browser; CI regenerates and fails on a diff, so editing copy without re-running the prerender breaks the build instead of quietly serving stale HTML to crawlers.

## Development

```sh
direnv allow          # or: nix develop
npm ci
npm run build:frontend   # compile Elm to public/app.js
dev                      # local worker + migrations + /api/manage bypass
```

Then open http://localhost:8787.

| Command | What it does |
|---|---|
| `npm run build:frontend` | Compile the Elm app with `--optimize` |
| `npm run build:frontend:dev` | Same, without `--optimize` (allows `Debug.log`) |
| `dev` | Local server with migrations applied and the operator-API bypass on |
| `wrangler dev` | Plain dev server; `/api/manage/*` will 403 |
| `npm test` | Playwright suite |
| `npm run screenshots` | Recapture `doc/screenshots/` — every screen, against stubbed API fixtures |
| `npm run prerender` | Regenerate the static marketing pages in `public/`. Run after changing their copy. |
| `elm-test` | Frontend unit tests (run from `frontend/`) |
| `nix flake check` | cargo fmt, clippy, tests, and elm-format |

`wrangler dev` and `wrangler deploy` both run `scripts/build-worker.sh`, which builds the frontend before the wasm — so a deploy can't ship a shell that loads a missing `app.js`.

### Layout

```
src/            the Worker
  api/          JSON endpoints — the frontend's only interface
  shell.rs      the one HTML document
  pipeline.rs   inbound message → reply
  risk.rs       the gate that withholds a draft
  prompt.rs     the fixed envelope every AI reply is wrapped in
frontend/       the Elm app
  src/Api.elm   every request and decoder, in one place
  src/Page/     one module per screen
public/         served from the edge, Worker not invoked
  app.js        the compiled Elm bundle
  boot.js       initialises Elm; the only JavaScript we hand-write
  _headers      CSP and caching for everything here
  *.html        prerendered marketing pages (generated; committed)
```

## Deploy

See the **[Deploy guide](https://ananthb.github.io/concierge/deployment.html)** for step-by-step instructions on forking and deploying your own instance.

CI/CD is handled by **Cloudflare Builds** (Workers CI), which builds and deploys directly from this repo without needing GitHub Actions or Nix.

To wire up your fork:

1. In the Cloudflare dashboard, create a Worker named (e.g.) `concierge` and connect this repo under **Settings → Builds**.
   - **Build command:** leave default (CF Builds runs `npm install` from `package.json`)
   - **Deploy command:** `npm run deploy`
2. Bind a D1 database (`DB`), KV namespace (`KV`), Workers AI (`AI`), the Email Service send-binding (`EMAIL`), a Durable Object (`REPLY_BUFFER` → `ReplyBufferDO`), and Queues (`SAFETY_QUEUE` producer + `concierge-safety` / `concierge-safety-dlq` consumers) under **Settings → Bindings**. Names must match the `binding` values in [`wrangler.toml`](wrangler.toml).
3. Set runtime variables and secrets under **Settings → Variables and Secrets**. The full list is at the bottom of [`wrangler.toml`](wrangler.toml).
4. Push to `main`.

## License

[AGPL-3.0](LICENSE).
