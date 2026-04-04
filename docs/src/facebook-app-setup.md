# Facebook App Setup

Step-by-step guide to configuring your Meta Developer app for Concierge.

## 1. Create a Facebook App

1. Go to [developers.facebook.com/apps](https://developers.facebook.com/apps/)
2. Click **Create App** → choose **Business** type
3. Enter app name and contact email

## 2. Basic Settings

Go to **Settings → Basic**:

- **App Domains**: `your-domain`
- **Privacy Policy URL**: `https://your-domain/privacy`
- **Terms of Service URL**: `https://your-domain/terms`
- **User Data Deletion**: Callback URL `https://your-domain/data-deletion`, select "Data Deletion Callback URL"

## 3. Facebook Login

1. Add the **Facebook Login** product
2. Go to **Facebook Login → Settings**
3. Add Valid OAuth Redirect URIs:
   - `https://your-domain/auth/facebook/callback`
   - `https://your-domain/instagram/callback`

## 4. WhatsApp

1. Add the **WhatsApp** product
2. Go to **WhatsApp → API Setup** — note your WABA ID
3. Go to **WhatsApp → Configuration**:
   - Callback URL: `https://your-domain/webhook/whatsapp`
   - Verify token: your `WHATSAPP_VERIFY_TOKEN` value
   - Subscribe to: `messages`

### Embedded Signup

1. Go to **WhatsApp → Embedded Signup**
2. Create a configuration
3. Copy the Config ID → set as `WHATSAPP_SIGNUP_CONFIG_ID` in `wrangler.toml`

## 5. Instagram Webhooks

1. Go to **Webhooks** in the left sidebar
2. Select **Instagram** from the dropdown
3. Click **Subscribe to this object**
   - Callback URL: `https://your-domain/webhook/instagram`
   - Verify token: your `INSTAGRAM_VERIFY_TOKEN` value
4. Subscribe to the `messages` field

## 6. App Review

Request these permissions:

| Permission | Purpose |
|-----------|---------|
| `email` | Sign-in (auto-approved) |
| `instagram_basic` | Read Instagram account info |
| `instagram_manage_messages` | Send/receive Instagram DMs |
| `pages_manage_metadata` | Discover Instagram business accounts |
| `pages_messaging` | Send DMs via Facebook Pages |
| `whatsapp_business_management` | WhatsApp Embedded Signup |
| `whatsapp_business_messaging` | Send WhatsApp messages |

For each permission, Meta requires:
- A screencast (30-60 seconds) showing the feature in action
- A description of the use case
- Your privacy policy URL

**Tip**: Before App Review is complete, add yourself as a tester at **App Roles → People** to test all features in development mode.
