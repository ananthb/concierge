# Introduction

**replybots** is an automated WhatsApp reply platform for small businesses, built as a single Cloudflare Worker in Rust (WebAssembly) with an Elm frontend and Cloudflare D1 for all persistent state.

It supports two reply modes per WhatsApp Business number:

- **Rule-matched reply** — send a fixed message immediately or after a short burst wait, with an optional natural-language match rule.
- **AI quick-handoff** — a single built-in front-desk persona answers greetings and simple questions briefly, then hands the conversation to a human the moment pricing, commitments, or custom requests come up.

## Key Properties

- **D1-only state** — all tenants, sessions, WhatsApp accounts, conversation state, inbound burst buffers, rate limits, metadata logs, and billing records live in a single Cloudflare D1 SQLite database.
- **Metadata-only logging** — inbound and outbound message text passes through in memory and is never persisted to D1.
- **Strict prepaid billing** — 1 credit = 1 reply, powered by Razorpay Orders and fail-closed webhooks.

## Links

- Hosted SaaS: [https://replybots.io](https://replybots.io)
- Source repository: [https://github.com/ananthb/replybots](https://github.com/ananthb/replybots)
- Rust API reference (`cargo doc`): [/api/replybots/index.html](/api/replybots/index.html)
