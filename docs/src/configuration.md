# Configuration & Secrets

## Environment Variables (`wrangler.toml`)

| Variable | Description |
|----------|-------------|
| `ENVIRONMENT` | `production` or `development` |
| `WHATSAPP_WABA_ID` | Your WhatsApp Business Account ID |
| `WHATSAPP_SIGNUP_CONFIG_ID` | Meta Embedded Signup configuration ID |

## Secrets (`wrangler secret put`)

| Secret | Description |
|--------|-------------|
| `ENCRYPTION_KEY` | 32-byte hex key for AES-256-GCM encryption. Generate with `openssl rand -hex 32` |
| `GOOGLE_OAUTH_CLIENT_ID` | Google OAuth client ID (for sign-in) |
| `GOOGLE_OAUTH_CLIENT_SECRET` | Google OAuth client secret |
| `FACEBOOK_APP_ID` | Facebook app ID (sign-in + Instagram + WhatsApp signup) |
| `FACEBOOK_APP_SECRET` | Facebook app secret |
| `WHATSAPP_ACCESS_TOKEN` | System user token for your shared WABA |
| `WHATSAPP_VERIFY_TOKEN` | Webhook verification token. Generate with `openssl rand -hex 16` |
| `INSTAGRAM_VERIFY_TOKEN` | Instagram webhook verification token. Generate with `openssl rand -hex 16` |

## OAuth Redirect URIs

Register these in the respective developer consoles:

- **Google**: `https://your-domain/auth/callback`
- **Facebook**: `https://your-domain/auth/facebook/callback` and `https://your-domain/instagram/callback`
