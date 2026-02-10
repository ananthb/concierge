#!/usr/bin/env bash
# Extract documentation from code

set -e
cd "$(dirname "$0")/.."

# Output directory for generated content
GENERATED_DIR="docs/src/generated"
mkdir -p "$GENERATED_DIR"

echo "Extracting CSS variables..."

# Generate CSS variables markdown
cat > "$GENERATED_DIR/css-variables.md" << 'HEADER'
# CSS Variables Reference

This page is auto-generated from the source code.

## Form CSS Variables

Use these variables to customize form appearance:

| Variable | Description |
|----------|-------------|
HEADER

# Extract and add form variables
grep -oP -- '--cf-[a-z-]+' src/templates/forms.rs 2>/dev/null | sort -u | while read -r var; do
    case "$var" in
        --cf-font-family) echo "| \`$var\` | Font family |" ;;
        --cf-font-size) echo "| \`$var\` | Base font size |" ;;
        --cf-text-color) echo "| \`$var\` | Main text color |" ;;
        --cf-bg-color) echo "| \`$var\` | Page background color |" ;;
        --cf-form-bg) echo "| \`$var\` | Form container background |" ;;
        --cf-border-color) echo "| \`$var\` | Input border color |" ;;
        --cf-border-radius) echo "| \`$var\` | Border radius |" ;;
        --cf-primary-color) echo "| \`$var\` | Primary/button color |" ;;
        --cf-primary-hover) echo "| \`$var\` | Primary hover color |" ;;
        --cf-input-padding) echo "| \`$var\` | Input padding |" ;;
        *) echo "| \`$var\` | - |" ;;
    esac
done >> "$GENERATED_DIR/css-variables.md"

cat >> "$GENERATED_DIR/css-variables.md" << 'MIDDLE'

## Calendar CSS Variables

Use these variables to customize calendar and booking views:

| Variable | Description |
|----------|-------------|
MIDDLE

# Extract and add calendar variables
grep -oP -- '--cal-[a-z-]+' src/templates/base.rs 2>/dev/null | sort -u | while read -r var; do
    case "$var" in
        --cal-primary) echo "| \`$var\` | Primary/accent color |" ;;
        --cal-text) echo "| \`$var\` | Text color |" ;;
        --cal-bg) echo "| \`$var\` | Background color |" ;;
        --cal-border-radius) echo "| \`$var\` | Border radius |" ;;
        --cal-font) echo "| \`$var\` | Font family |" ;;
        *) echo "| \`$var\` | - |" ;;
    esac
done >> "$GENERATED_DIR/css-variables.md"

cat >> "$GENERATED_DIR/css-variables.md" << 'USAGE'

## Usage Examples

### Override via Query Parameter

```html
<iframe src=".../f/contact?css=--cf-primary-color:green"></iframe>
```

### Override in Page CSS (HTMX)

```css
:root {
    --cf-primary-color: #6366f1;
    --cal-primary: #6366f1;
}
```

### Override in Admin Settings

1. Open form/calendar editor
2. Go to Styling tab
3. Add Custom CSS:

```css
:root {
    --cf-primary-color: #your-color;
}
```
USAGE

echo "Generating channel configuration..."

cat > "$GENERATED_DIR/channels.md" << 'EOF'
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
EOF

echo "Generating query parameters..."

cat > "$GENERATED_DIR/query-params.md" << 'EOF'
# Query Parameters Reference

This page is auto-generated from the source code.

## Form Embedding

| Parameter | Description | Example |
|-----------|-------------|---------|
| `css` | Inline CSS to apply | `?css=button{background:green}` |
| `css_url` | URL to external stylesheet | `?css_url=https://example.com/style.css` |

## Calendar & Booking Views

| Parameter | Description | Example |
|-----------|-------------|---------|
| `css` | Inline CSS to apply | `?css=.event{color:blue}` |
| `css_url` | URL to external stylesheet | `?css_url=https://example.com/cal.css` |
| `date` | Starting date for view | `?date=2024-03-15` |
| `view` | View type (week/month/year/endless) | `?view=month` |
| `days` | Number of days to show (booking) | `?days=14` |
| `hide_title` | Hide the title | `?hide_title=true` |

## iCal Feeds

| Parameter | Description | Example |
|-----------|-------------|---------|
| `token` | Authentication token | `?token=abc123` |

## Combining Parameters

Multiple parameters can be combined:

```
/f/contact?css=button{background:green}&hide_title=true
/view/{id}/{slug}?date=2024-03-15&view=month&css_url=https://example.com/cal.css
```
EOF

echo "Generating URL routes..."

cat > "$GENERATED_DIR/routes.md" << 'EOF'
# URL Routes Reference

This page is auto-generated from the source code.

## Public Routes

| Route | Method | Description |
|-------|--------|-------------|
| `/f/{slug}` | GET | Render public form |
| `/f/{slug}/submit` | POST | Submit form |
| `/book/{calendar_id}/{slug}` | GET | Render booking form |
| `/book/{calendar_id}/{slug}/submit` | POST | Submit booking |
| `/book/{calendar_id}/{slug}/approve/{booking_id}` | GET | Approve pending booking |
| `/book/{calendar_id}/{slug}/cancel/{booking_id}` | GET | Cancel booking |
| `/view/{calendar_id}/{slug}` | GET | Render calendar view |
| `/feed/{calendar_id}/{slug}` | GET | iCal feed |

## Admin Routes

| Route | Method | Description |
|-------|--------|-------------|
| `/admin` | GET | Admin dashboard |
| `/admin/forms/new` | GET | New form editor |
| `/admin/forms/{slug}` | GET | Edit form |
| `/admin/forms/{slug}` | PUT | Update form |
| `/admin/forms/{slug}` | DELETE | Delete form |
| `/admin/forms/{slug}/responses` | GET | View form responses |
| `/admin/forms/{slug}/archive` | POST | Archive form |
| `/admin/forms/{slug}/unarchive` | POST | Unarchive form |
| `/admin/calendars` | POST | Create calendar |
| `/admin/calendars/{id}` | GET | Edit calendar |
| `/admin/calendars/{id}` | PUT | Update calendar |
| `/admin/calendars/{id}` | DELETE | Delete calendar |
| `/admin/calendars/{id}/archive` | POST | Archive calendar |
| `/admin/calendars/{id}/unarchive` | POST | Unarchive calendar |
| `/admin/calendars/{id}/bookings` | GET | View all bookings |
| `/admin/calendars/{id}/slots` | GET | Configure time slots |
| `/admin/calendars/{id}/events` | GET | Manage events |
| `/admin/calendars/{id}/booking` | POST | Create booking link |
| `/admin/calendars/{id}/booking/{link_id}` | GET | Edit booking link |
| `/admin/calendars/{id}/view` | POST | Create view link |
| `/admin/calendars/{id}/view/{link_id}` | GET | Edit view link |
| `/admin/calendars/{id}/feed` | POST | Create feed link |

## Instagram Routes

| Route | Method | Description |
|-------|--------|-------------|
| `/instagram/auth/{calendar_id}` | GET | Start Instagram OAuth |
| `/instagram/callback` | GET | OAuth callback |
| `/instagram/disconnect/{calendar_id}/{source_id}` | DELETE | Disconnect account |

## Static Assets

| Route | Description |
|-------|-------------|
| `/logo.svg` | Application logo |
EOF

echo "Generated documentation in $GENERATED_DIR"
