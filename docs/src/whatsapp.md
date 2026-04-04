# WhatsApp Auto-Reply

## Adding a WhatsApp Number

1. Go to **Admin → WhatsApp Accounts → Connect WhatsApp Number**
2. Complete Meta's Embedded Signup flow to register your phone number
3. The phone number and phone number ID are configured automatically

Alternatively, use the manual flow at `/admin/whatsapp/manual` if you already know your phone number ID.

## Configuring Auto-Reply

Each WhatsApp account has its own auto-reply settings:

- **Enabled** — Toggle auto-reply on/off
- **Mode** — Static (fixed message) or AI (generated response)
- **Prompt/Message** — The static message to send, or the AI system prompt

### Static Mode

Every incoming message gets the same reply. Good for "We'll get back to you" or business hours info.

### AI Mode

The prompt is used as a system instruction for Cloudflare Workers AI. The sender's name and message are passed as context. The AI generates a contextual reply.

## Platform Model

All WhatsApp numbers share a single platform token (`WHATSAPP_ACCESS_TOKEN`). This is a system user token for your WhatsApp Business Account. You don't need per-customer tokens.

## Message Logging

All inbound and outbound messages are logged to the D1 database (`whatsapp_messages` table) for audit and debugging.
