# How It Works

Concierge runs as a single Cloudflare Worker that handles three messaging channels:

## WhatsApp Auto-Reply

1. A customer sends a WhatsApp message to your business number
2. Meta delivers the message to Concierge via webhook
3. Concierge looks up the number, checks if auto-reply is enabled
4. Sends a reply — either a static message or an AI-generated response
5. Both messages are logged to the database

## Instagram DM Auto-Reply

1. Someone sends a DM to your Instagram business account
2. Meta delivers the message via webhook
3. Concierge looks up the account by page ID, checks auto-reply config
4. Sends a reply via the Instagram Pages API
5. Both messages are logged

## Lead Capture Forms

1. You create a lead form in the admin and embed it on your website
2. A visitor enters their phone number and submits
3. Concierge generates a message (static or AI) and sends it via WhatsApp
4. The submission is logged to the database

## Platform Model

Concierge uses a **shared WhatsApp Business Account (WABA)**. You own one WABA, and customers add their phone numbers to it via Meta's Embedded Signup flow. A single platform token (`WHATSAPP_ACCESS_TOKEN`) is used to send messages from any number on the WABA.

Instagram uses **per-account OAuth tokens** via Facebook Login. Each customer connects their Instagram business account through a standard OAuth flow. Page tokens are encrypted and stored in KV.

## Architecture

- **Cloudflare Worker** — Rust compiled to WebAssembly, handles all HTTP routes
- **Cloudflare KV** — Stores account configs, tokens, and session data
- **Cloudflare D1** — SQLite database for message logs and form submissions
- **Cloudflare Workers AI** — Powers AI-mode auto-replies
