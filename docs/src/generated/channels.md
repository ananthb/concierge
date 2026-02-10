# Channel Configuration Reference

This page is auto-generated from the source code.

## Available Channels

The following notification channels are supported:

| Channel | Provider | Use Case |
|---------|----------|----------|
| `twilio_sms` | Twilio | SMS to mobile numbers |
| `twilio_whatsapp` | Twilio | WhatsApp via Twilio |
| `meta_whatsapp` | Meta | WhatsApp Business API |
| `twilio_email` | SendGrid via Twilio | Email notifications |
| `resend_email` | Resend | Email notifications |

## Required Secrets

### Twilio SMS

```bash
wrangler secret put TWILIO_SID
wrangler secret put TWILIO_TOKEN
wrangler secret put TWILIO_FROM_SMS        # e.g., +15551234567
```

### Twilio WhatsApp

```bash
wrangler secret put TWILIO_SID
wrangler secret put TWILIO_TOKEN
wrangler secret put TWILIO_FROM_WHATSAPP   # e.g., whatsapp:+15551234567
```

### WhatsApp Business API (Meta)

```bash
wrangler secret put WHATSAPP_ACCESS_TOKEN
wrangler secret put WHATSAPP_PHONE_NUMBER_ID
```

### Twilio Email (via SendGrid)

```bash
wrangler secret put SENDGRID_API_KEY
wrangler secret put TWILIO_FROM_EMAIL      # e.g., noreply@yourdomain.com
```

### Resend Email

```bash
wrangler secret put RESEND_API_KEY
wrangler secret put RESEND_FROM            # e.g., noreply@yourdomain.com
```

## Channel Detection

The admin UI shows Responders and Digest tabs only when channels are available:

| Channel | Detection |
|---------|-----------|
| Twilio SMS | `TWILIO_SID` and `TWILIO_FROM_SMS` are set |
| Twilio WhatsApp | `TWILIO_SID` and `TWILIO_FROM_WHATSAPP` are set |
| Twilio Email | `SENDGRID_API_KEY` and `TWILIO_FROM_EMAIL` are set |
| Resend Email | `RESEND_API_KEY` and `RESEND_FROM` are set |

> **Note:** If you don't see the Responders or Digest tabs in the admin UI, ensure at least one channel's secrets are configured.
