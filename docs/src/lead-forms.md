# Lead Capture Forms

Lead forms are embeddable phone number capture widgets. When someone submits their phone number, they receive a WhatsApp message immediately.

## Creating a Form

1. Go to **Admin → Lead Forms → New Form**
2. Configure:
   - **Name** — displayed as the form heading
   - **WhatsApp Account** — which number sends the reply
   - **Reply Mode** — Static or AI
   - **Reply Prompt** — the message text or AI prompt
   - **Style** — colors, button text, placeholder, success message, custom CSS
   - **Allowed Origins** — restrict which domains can embed the form (empty = all)
3. Copy the embed code from the form's edit page

## Embedding

Use the iframe snippet from the edit page:

```html
<iframe src="https://your-domain/lead/{form_id}/{slug}"
        width="400" height="200" frameborder="0"></iframe>
```

The form submits via HTMX within the iframe — no page navigation.

## CORS

If you set allowed origins, only those domains can embed and submit the form. Leave empty to allow embedding from any site.

## Styling

Customize the form appearance with:

- Primary color, text color, background color
- Border radius
- Button text and placeholder text
- Success message shown after submission
- Custom CSS (injected into the form's `<style>` tag)
