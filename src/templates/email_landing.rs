//! Landing page served when someone visits the EMAIL_DOMAIN host (or any of
//! its subdomains) in a browser. Replaces an instant 301 to PUBLIC_BASE_URL
//! with a brief explanation, a manual button, and a 5-second meta-refresh
//! fallback so visitors who landed here by accident still end up on the
//! main site without doing anything.
//!
//! Self-contained: inline CSS only, no external assets, since this is served
//! from a different host than the rest of the site.

use crate::helpers::html_escape;

pub fn email_landing_html(public_base_url: &str) -> String {
    let target = html_escape(public_base_url);
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Concierge Email</title>
<meta http-equiv="refresh" content="5;url={target}">
<style nonce="__CSP_NONCE__">
  :root {{ color-scheme: light; }}
  html, body {{ margin: 0; padding: 0; background: #F5EFE4; color: #1B1814;
                font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }}
  .wrap {{ min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 24px; }}
  .card {{ max-width: 540px; background: #FBF7EE; border: 1px solid #E5DCC8;
           border-radius: 12px; padding: 36px 32px; text-align: center; }}
  h1 {{ margin: 0 0 12px; font-size: 28px; letter-spacing: -0.01em; }}
  p {{ margin: 0 0 12px; line-height: 1.55; color: #3A332B; }}
  .btn {{ display: inline-block; margin-top: 20px; padding: 12px 22px;
          background: #E86A2C; color: #fff; text-decoration: none; border-radius: 8px;
          font-weight: 600; font-size: 15px; }}
  .btn:hover {{ background: #C9551E; }}
  .countdown {{ display: block; margin-top: 18px; font-size: 13px; color: #5E5246; }}
</style>
</head>
<body>
  <div class="wrap">
    <div class="card">
      <h1>Concierge Email</h1>
      <p>Concierge is automated customer messaging for small businesses — auto-replies across WhatsApp, Instagram DMs, Discord, and email.</p>
      <p>This domain hosts each tenant's customer-facing email addresses. There's nothing to see here in a browser.</p>
      <a class="btn" href="{target}">Visit Concierge</a>
      <span class="countdown">Redirecting in <span id="n">5</span>s…</span>
    </div>
  </div>
  <script nonce="__CSP_NONCE__">
    let n = 5;
    const el = document.getElementById('n');
    setInterval(() => {{ n -= 1; if (n >= 0 && el) el.textContent = n; }}, 1000);
  </script>
</body>
</html>"##
    )
}
