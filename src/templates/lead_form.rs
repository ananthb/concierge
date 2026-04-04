//! Lead capture form templates

use crate::helpers::html_escape;
use crate::types::LeadCaptureForm;

use super::HASH;

/// Render the embeddable lead capture form
pub fn lead_form_html(form: &LeadCaptureForm) -> String {
    let s = &form.style;
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<link rel="icon" type="image/svg+xml" href="/logo.svg">
<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32.png">
<meta name="theme-color" content="#F38020">
<title>{name}</title>
<script src="https://unpkg.com/htmx.org@1.9.10"></script>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
:root{{--lf-primary:{primary};--lf-text:{text};--lf-bg:{bg};--lf-radius:{radius}}}
body{{font-family:system-ui,-apple-system,sans-serif;background:var(--lf-bg);color:var(--lf-text);display:flex;align-items:center;justify-content:center;min-height:100vh;padding:1rem}}
.lf-form{{max-width:400px;width:100%;text-align:center}}
.lf-form h2{{font-size:1.25rem;margin-bottom:1rem}}
.lf-input{{width:100%;padding:.75rem 1rem;border:1px solid {hash}ccc;border-radius:var(--lf-radius);font-size:1rem;margin-bottom:.75rem;background:var(--lf-bg);color:var(--lf-text)}}
.lf-input:focus{{outline:none;border-color:var(--lf-primary)}}
.lf-btn{{width:100%;padding:.75rem;background:var(--lf-primary);color:{hash}fff;border:none;border-radius:var(--lf-radius);font-size:1rem;font-weight:600;cursor:pointer}}
.lf-btn:hover{{opacity:.9}}
.lf-btn:disabled{{opacity:.6;cursor:not-allowed}}
.lf-success{{padding:2rem;text-align:center;color:var(--lf-text)}}
.lf-success h2{{color:var(--lf-primary);margin-bottom:.5rem}}
.lf-error{{color:{hash}dc3545;font-size:.9rem;margin-bottom:.75rem}}
{custom_css}
</style>
</head>
<body>
<div class="lf-form">
  <h2>{name}</h2>
  <form hx-post="/lead/{id}/{slug}" hx-target="closest .lf-form" hx-swap="innerHTML"
        hx-on::before-request="this.querySelector('button').disabled=true"
        hx-on::after-request="this.querySelector('button').disabled=false">
    <input type="tel" name="phone" class="lf-input" placeholder="{placeholder}" required>
    <button type="submit" class="lf-btn">{button}</button>
  </form>
  <p style="margin-top:1rem;font-size:.65rem;color:{hash}999;"><a href="https://github.com/ananthb/concierge-worker" style="color:{hash}999;">Powered by Concierge</a></p>
</div>
</body>
</html>"##,
        name = html_escape(&form.name),
        id = html_escape(&form.id),
        slug = html_escape(&form.slug),
        primary = html_escape(&s.primary_color),
        text = html_escape(&s.text_color),
        bg = html_escape(&s.background_color),
        radius = html_escape(&s.border_radius),
        placeholder = html_escape(&s.placeholder_text),
        button = html_escape(&s.button_text),
        custom_css = s.custom_css.replace(['<', '>'], ""),
        hash = HASH,
    )
}

/// Success state after form submission
pub fn lead_form_success_html(form: &LeadCaptureForm) -> String {
    format!(
        r#"<div class="lf-success">
  <h2>Sent!</h2>
  <p>{message}</p>
</div>"#,
        message = html_escape(&form.style.success_message),
    )
}

/// Error state
pub fn lead_form_error_html(form: &LeadCaptureForm, error: &str) -> String {
    let s = &form.style;
    format!(
        r#"<h2>{name}</h2>
<div class="lf-error">{error}</div>
<form hx-post="/lead/{id}/{slug}" hx-target="closest .lf-form" hx-swap="innerHTML"
      hx-on::before-request="this.querySelector('button').disabled=true"
      hx-on::after-request="this.querySelector('button').disabled=false">
  <input type="tel" name="phone" class="lf-input" placeholder="{placeholder}" required>
  <button type="submit" class="lf-btn">{button}</button>
</form>"#,
        name = html_escape(&form.name),
        id = html_escape(&form.id),
        slug = html_escape(&form.slug),
        error = html_escape(error),
        placeholder = html_escape(&s.placeholder_text),
        button = html_escape(&s.button_text),
    )
}
