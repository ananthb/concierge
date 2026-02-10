# Configuration

All sensitive configuration is stored as Cloudflare Workers secrets.

## Setting Secrets

Use the Wrangler CLI to set secrets:

```bash
wrangler secret put SECRET_NAME
```

You'll be prompted to enter the value securely.

## Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `ENVIRONMENT` | Set to `development` to bypass auth | No |

## All Secrets Reference

| Secret | Purpose |
|--------|---------|
| `TWILIO_SID` | Twilio Account SID |
| `TWILIO_TOKEN` | Twilio Auth Token |
| `TWILIO_FROM_SMS` | SMS sender number (e.g., `+15551234567`) |
| `TWILIO_FROM_WHATSAPP` | WhatsApp sender (e.g., `whatsapp:+15551234567`) |
| `TWILIO_FROM_EMAIL` | Email sender address for Twilio/SendGrid |
| `SENDGRID_API_KEY` | SendGrid API key (for Twilio email) |
| `WHATSAPP_ACCESS_TOKEN` | WhatsApp Business API access token |
| `WHATSAPP_PHONE_NUMBER_ID` | WhatsApp Business API phone number ID |
| `RESEND_API_KEY` | Resend API key |
| `RESEND_FROM` | Resend sender address |
| `GOOGLE_SERVICE_ACCOUNT_JSON` | Google service account JSON (for Sheets) |
| `ENCRYPTION_KEY` | Token encryption key (for Instagram) |
| `INSTAGRAM_APP_ID` | Meta/Instagram app ID |
| `INSTAGRAM_APP_SECRET` | Meta/Instagram app secret |

## Notification Channels

> **Important:** The Responders and Digest tabs in the admin UI are only visible when at least one notification channel is configured.

### Twilio SMS

```bash
wrangler secret put TWILIO_SID
wrangler secret put TWILIO_TOKEN
wrangler secret put TWILIO_FROM_SMS
```

### Twilio WhatsApp

```bash
wrangler secret put TWILIO_SID
wrangler secret put TWILIO_TOKEN
wrangler secret put TWILIO_FROM_WHATSAPP
```

### WhatsApp Business API (Meta)

For direct WhatsApp Business API integration:

```bash
wrangler secret put WHATSAPP_ACCESS_TOKEN
wrangler secret put WHATSAPP_PHONE_NUMBER_ID
```

### Twilio Email (via SendGrid)

```bash
wrangler secret put SENDGRID_API_KEY
wrangler secret put TWILIO_FROM_EMAIL
```

### Resend Email

```bash
wrangler secret put RESEND_API_KEY
wrangler secret put RESEND_FROM
```

## Channel Detection

The admin UI automatically detects which channels are available:

| Channel | Required Secrets |
|---------|------------------|
| Twilio SMS | `TWILIO_SID` + `TWILIO_FROM_SMS` |
| Twilio WhatsApp | `TWILIO_SID` + `TWILIO_FROM_WHATSAPP` |
| Twilio Email | `SENDGRID_API_KEY` + `TWILIO_FROM_EMAIL` |
| Resend Email | `RESEND_API_KEY` + `RESEND_FROM` |

## Google Sheets Integration

For syncing form submissions to Google Sheets:

```bash
wrangler secret put GOOGLE_SERVICE_ACCOUNT_JSON
```

The value should be the entire JSON content of your Google Cloud service account key file.

## Instagram Integration

For automatic event extraction from Instagram posts:

```bash
# Generate with: openssl rand -hex 32
wrangler secret put ENCRYPTION_KEY

wrangler secret put INSTAGRAM_APP_ID
wrangler secret put INSTAGRAM_APP_SECRET
```

### Setting up Meta App

1. Go to [Meta for Developers](https://developers.facebook.com/)
2. Create an app with **Instagram Basic Display** product
3. Configure OAuth redirect URI: `https://your-worker.workers.dev/instagram/callback`
4. Add test users while in development mode
