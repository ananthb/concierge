# Embedding Forms

Lead capture forms can be embedded on any website using an iframe.

## Basic Embed

```html
<iframe src="https://your-domain/lead/{form_id}/{slug}"
        width="400" height="200" frameborder="0"></iframe>
```

Get the exact snippet from **Admin → Lead Forms → [your form] → Embed Code**.

## Responsive Sizing

```html
<iframe src="https://your-domain/lead/{form_id}/{slug}"
        style="width: 100%; max-width: 400px; height: 200px; border: none;"></iframe>
```

## Allowed Origins

To restrict which sites can embed your form, configure **Allowed Origins** in the form settings. Add one origin per line (e.g., `https://example.com`). If left empty, any site can embed the form.

## Custom Styling

Use the style fields in the form editor to match your site's design. For advanced customization, use the **Custom CSS** field.
