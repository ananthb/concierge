# Instagram DM Auto-Reply

## Connecting Instagram

1. Go to **Admin → Instagram Accounts → Connect Account**
2. Sign in with Facebook (the account that manages your Instagram business page)
3. Concierge discovers your Facebook Pages and finds the linked Instagram business account
4. The page token is encrypted and stored for sending DM replies

## Configuring Auto-Reply

- **Enabled** — Toggle the account on/off
- **Auto-Reply Enabled** — Toggle auto-reply specifically
- **Mode** — Static or AI (same as WhatsApp)
- **Prompt/Message** — The reply text or AI system prompt

## How It Works

1. Meta delivers incoming DMs to `POST /webhook/instagram`
2. Concierge verifies the webhook signature
3. Looks up the Instagram account by the recipient page ID
4. Generates a reply (static or AI)
5. Sends via the Instagram Pages Messaging API
6. Logs both messages to D1

## Requirements

Your Meta app needs these permissions (requires App Review):

- `instagram_basic`
- `instagram_manage_messages`
- `pages_manage_metadata`
- `pages_messaging`

## Token Refresh

Page tokens are long-lived but can expire. A daily cron job checks all tokens and refreshes those within 7 days of expiry.
