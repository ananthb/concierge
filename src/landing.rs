//! Landing page HTML served at /

pub fn landing_page_html() -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<link rel="icon" type="image/svg+xml" href="/logo.svg">
<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32.png">
<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16.png">
<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">
<link rel="manifest" href="/site.webmanifest">
<meta name="msapplication-TileColor" content="#1A1A2E">
<meta name="theme-color" content="#F38020">
<title>Concierge — Messaging automation for small businesses</title>
<meta name="description" content="WhatsApp auto-replies, Instagram DM auto-replies, and embeddable lead capture forms. One platform, zero effort.">
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
:root{{--p:#F38020;--bg:#fff;--text:#111;--muted:#666;--card:#f8f9fa;--grad:linear-gradient(135deg,#F38020,#F9A825)}}
@media(prefers-color-scheme:dark){{:root{{--bg:#0a0a0a;--text:#eee;--muted:#999;--card:#161616}}}}
body{{font-family:system-ui,-apple-system,sans-serif;background:var(--bg);color:var(--text);line-height:1.6;overflow-x:hidden}}
a{{color:var(--p);text-decoration:none}}a:hover{{text-decoration:underline}}

.hero{{min-height:90vh;display:flex;align-items:center;justify-content:center;text-align:center;padding:2rem 1rem;position:relative}}
.hero::before{{content:'';position:absolute;inset:0;background:var(--grad);opacity:.04;pointer-events:none}}
.hero-inner{{max-width:720px;position:relative}}
.hero h1{{font-size:clamp(2.5rem,6vw,4rem);font-weight:800;letter-spacing:-.03em;margin-bottom:1rem}}
.hero h1 span{{background:var(--grad);-webkit-background-clip:text;-webkit-text-fill-color:transparent;background-clip:text}}
.hero p{{font-size:1.25rem;color:var(--muted);max-width:560px;margin:0 auto 2rem}}
.cta{{display:inline-block;padding:.875rem 2.5rem;background:var(--p);color:#fff;border-radius:8px;font-size:1.125rem;font-weight:600;transition:transform .15s,box-shadow .15s}}
.cta:hover{{transform:translateY(-2px);box-shadow:0 8px 24px rgba(243,128,32,.35);text-decoration:none}}
.cta-row{{display:flex;gap:1rem;justify-content:center;flex-wrap:wrap;align-items:center}}
.cta-secondary{{display:inline-block;padding:.875rem 2rem;border:2px solid #1A1A2E;color:#1A1A2E;border-radius:8px;font-size:1.125rem;font-weight:600;transition:background .15s,color .15s}}
.cta-secondary:hover{{background:#1A1A2E;color:#fff;text-decoration:none}}

.features{{max-width:960px;margin:0 auto;padding:4rem 1rem 6rem;display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:2rem}}
.feature{{background:var(--card);border-radius:12px;padding:2rem;text-align:center}}
.feature-icon{{font-size:2.5rem;margin-bottom:.75rem;display:block}}
.feature h3{{font-size:1.125rem;margin-bottom:.5rem}}
.feature p{{font-size:.95rem;color:var(--muted)}}

.how{{max-width:720px;margin:0 auto;padding:0 1rem 6rem;text-align:center}}
.how h2{{font-size:2rem;font-weight:700;margin-bottom:2rem}}
.steps{{display:grid;gap:1.5rem;text-align:left;counter-reset:step}}
.step{{display:flex;gap:1rem;align-items:flex-start}}
.step-num{{background:var(--grad);color:#fff;width:36px;height:36px;border-radius:50%;display:flex;align-items:center;justify-content:center;font-weight:700;font-size:.9rem;flex-shrink:0;counter-increment:step}}
.step-num::before{{content:counter(step)}}
.step-body h4{{font-size:1rem;margin-bottom:.25rem}}
.step-body p{{font-size:.9rem;color:var(--muted)}}

.bottom-cta{{text-align:center;padding:4rem 1rem 6rem;background:var(--card)}}
.bottom-cta h2{{font-size:1.75rem;font-weight:700;margin-bottom:.75rem}}
.bottom-cta p{{color:var(--muted);margin-bottom:2rem;font-size:1.05rem}}

footer{{text-align:center;padding:2rem 1rem;color:var(--muted);font-size:.85rem}}
footer a{{color:var(--muted)}}footer a:hover{{color:var(--p)}}
</style>
</head>
<body>

<section class="hero">
<div class="hero-inner">
  <h1><span>Concierge</span></h1>
  <p>Automate WhatsApp replies, Instagram DMs, and lead capture — all from one place. Built for small businesses that move fast.</p>
  <div class="cta-row">
    <a href="/auth/login" class="cta">Get started free</a>
    <a href="https://ananthb.github.io/concierge-worker/" class="cta-secondary">Read the docs</a>
  </div>
</div>
</section>

<section class="features">
<div class="feature">
  <span class="feature-icon" aria-hidden="true">{whatsapp}</span>
  <h3>WhatsApp Auto&#8209;Reply</h3>
  <p>Incoming message? Reply instantly with a static message or an AI&#8209;generated response. Never miss a lead.</p>
</div>
<div class="feature">
  <span class="feature-icon" aria-hidden="true">{instagram}</span>
  <h3>Instagram DM Auto&#8209;Reply</h3>
  <p>Connect your Instagram business account and auto-reply to every DM with static or AI&#8209;powered messages.</p>
</div>
<div class="feature">
  <span class="feature-icon" aria-hidden="true">{form}</span>
  <h3>Lead Capture Forms</h3>
  <p>Embed a phone number form on any site. When someone submits, they get a WhatsApp message instantly.</p>
</div>
</section>

<section class="how">
<h2>Up and running in 3 steps</h2>
<div class="steps">
  <div class="step">
    <div class="step-num"></div>
    <div class="step-body">
      <h4>Sign in with Google</h4>
      <p>One click. No passwords to remember, no credit card required.</p>
    </div>
  </div>
  <div class="step">
    <div class="step-num"></div>
    <div class="step-body">
      <h4>Connect your channels</h4>
      <p>Add your WhatsApp number and connect Instagram via Facebook Login.</p>
    </div>
  </div>
  <div class="step">
    <div class="step-num"></div>
    <div class="step-body">
      <h4>Configure auto-replies</h4>
      <p>Set a static message or write an AI prompt. Create lead forms and embed them anywhere.</p>
    </div>
  </div>
</div>
</section>

<section class="bottom-cta">
<h2>Stop doing it manually</h2>
<p>Your customers expect instant responses. Concierge delivers them while you focus on your business.</p>
<a href="/auth/login" class="cta">Start automating — it's free</a>
</section>

<footer>
<a href="https://ananthb.github.io/concierge-worker/">Documentation</a> &middot;
<a href="https://github.com/ananthb/concierge-worker">GitHub</a> &middot;
<a href="/terms">Terms</a> &middot;
<a href="/privacy">Privacy</a>
</footer>

</body>
</html>"##,
        whatsapp = r##"<svg width="40" height="40" viewBox="0 0 24 24" fill="#25D366"><path d="M17.472 14.382c-.297-.149-1.758-.867-2.03-.967-.273-.099-.471-.148-.67.15-.197.297-.767.966-.94 1.164-.173.199-.347.223-.644.075-.297-.15-1.255-.463-2.39-1.475-.883-.788-1.48-1.761-1.653-2.059-.173-.297-.018-.458.13-.606.134-.133.298-.347.446-.52.149-.174.198-.298.298-.497.099-.198.05-.371-.025-.52-.075-.149-.669-1.612-.916-2.207-.242-.579-.487-.5-.669-.51-.173-.008-.371-.01-.57-.01-.198 0-.52.074-.792.372-.272.297-1.04 1.016-1.04 2.479 0 1.462 1.065 2.875 1.213 3.074.149.198 2.096 3.2 5.077 4.487.709.306 1.262.489 1.694.625.712.227 1.36.195 1.871.118.571-.085 1.758-.719 2.006-1.413.248-.694.248-1.289.173-1.413-.074-.124-.272-.198-.57-.347m-5.421 7.403h-.004a9.87 9.87 0 01-5.031-1.378l-.361-.214-3.741.982.998-3.648-.235-.374a9.86 9.86 0 01-1.51-5.26c.001-5.45 4.436-9.884 9.888-9.884 2.64 0 5.122 1.03 6.988 2.898a9.825 9.825 0 012.893 6.994c-.003 5.45-4.437 9.884-9.885 9.884m8.413-18.297A11.815 11.815 0 0012.05 0C5.495 0 .16 5.335.157 11.892c0 2.096.547 4.142 1.588 5.945L.057 24l6.305-1.654a11.882 11.882 0 005.683 1.448h.005c6.554 0 11.89-5.335 11.893-11.893a11.821 11.821 0 00-3.48-8.413z"/></svg>"##,
        instagram = r##"<svg width="40" height="40" viewBox="0 0 24 24" fill="#E4405F"><path d="M12 2.163c3.204 0 3.584.012 4.85.07 3.252.148 4.771 1.691 4.919 4.919.058 1.265.069 1.645.069 4.849 0 3.205-.012 3.584-.069 4.849-.149 3.225-1.664 4.771-4.919 4.919-1.266.058-1.644.07-4.85.07-3.204 0-3.584-.012-4.849-.07-3.26-.149-4.771-1.699-4.919-4.92-.058-1.265-.07-1.644-.07-4.849 0-3.204.013-3.583.07-4.849.149-3.227 1.664-4.771 4.919-4.919 1.266-.057 1.645-.069 4.849-.069zM12 0C8.741 0 8.333.014 7.053.072 2.695.272.273 2.69.073 7.052.014 8.333 0 8.741 0 12c0 3.259.014 3.668.072 4.948.2 4.358 2.618 6.78 6.98 6.98C8.333 23.986 8.741 24 12 24c3.259 0 3.668-.014 4.948-.072 4.354-.2 6.782-2.618 6.979-6.98.059-1.28.073-1.689.073-4.948 0-3.259-.014-3.667-.072-4.947-.196-4.354-2.617-6.78-6.979-6.98C15.668.014 15.259 0 12 0zm0 5.838a6.162 6.162 0 100 12.324 6.162 6.162 0 000-12.324zM12 16a4 4 0 110-8 4 4 0 010 8zm6.406-11.845a1.44 1.44 0 100 2.881 1.44 1.44 0 000-2.881z"/></svg>"##,
        form = r##"<svg width="40" height="40" viewBox="0 0 24 24" fill="#673AB7"><path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z"/></svg>"##,
    )
}

const LEGAL_STYLE: &str = r#"*{margin:0;padding:0;box-sizing:border-box}body{font-family:system-ui,-apple-system,sans-serif;color:#333;line-height:1.7;padding:2rem 1rem;max-width:720px;margin:0 auto}h1{font-size:2rem;margin-bottom:1rem}h2{font-size:1.25rem;margin:2rem 0 .5rem}p,ul{margin-bottom:1rem}ul{padding-left:1.5rem}a{color:#F38020}footer{margin-top:3rem;padding-top:1rem;border-top:1px solid #ddd;font-size:.85rem;color:#666}"#;

pub fn terms_of_service_html() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<link rel="icon" type="image/svg+xml" href="/logo.svg">
<title>Terms of Service — Concierge</title>
<style>{style}</style>
</head>
<body>
<h1>Terms of Service</h1>
<p><strong>Effective date:</strong> April 4, 2026</p>

<h2>1. Service</h2>
<p>Concierge ("the Service") is a messaging automation platform operated at concierge.calculon.tech. By using the Service you agree to these terms.</p>

<h2>2. Accounts</h2>
<p>You sign in with Google OAuth. You are responsible for the activity on your account and the phone numbers and Instagram accounts you connect.</p>

<h2>3. Acceptable Use</h2>
<p>You must not use the Service to send spam, unsolicited messages, or any content that violates applicable law. You must comply with Meta's WhatsApp Business Policy and Instagram Platform Policy.</p>

<h2>4. Data</h2>
<p>We store the minimum data needed to operate: your email, connected account metadata, and message logs. See our <a href="/privacy">Privacy Policy</a> for details. You can delete all your data at any time from Settings.</p>

<h2>5. No Warranty</h2>
<p>The Service is provided "as is" without warranty of any kind. We do not guarantee uptime, message delivery, or API availability.</p>

<h2>6. Limitation of Liability</h2>
<p>To the maximum extent permitted by law, we are not liable for any indirect, incidental, or consequential damages arising from your use of the Service.</p>

<h2>7. Changes</h2>
<p>We may update these terms. Continued use after changes constitutes acceptance.</p>

<h2>8. Contact</h2>
<p>Questions? Open an issue at <a href="https://github.com/ananthb/concierge-worker">github.com/ananthb/concierge-worker</a>.</p>

<footer><a href="/">Home</a> · <a href="/privacy">Privacy Policy</a></footer>
</body>
</html>"#,
        style = LEGAL_STYLE,
    )
}

pub fn privacy_policy_html() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<link rel="icon" type="image/svg+xml" href="/logo.svg">
<title>Privacy Policy — Concierge</title>
<style>{style}</style>
</head>
<body>
<h1>Privacy Policy</h1>
<p><strong>Effective date:</strong> April 4, 2026</p>

<h2>What we collect</h2>
<ul>
<li><strong>Account info:</strong> your Google email and name (from OAuth sign-in)</li>
<li><strong>Connected accounts:</strong> WhatsApp phone number IDs, Instagram page IDs, and encrypted access tokens</li>
<li><strong>Message logs:</strong> inbound/outbound WhatsApp and Instagram messages processed by auto-reply</li>
<li><strong>Lead form submissions:</strong> phone numbers submitted through your lead capture forms</li>
</ul>

<h2>How we use it</h2>
<p>Solely to operate the Service: routing messages, generating auto-replies, and displaying your admin dashboard. We do not sell, share, or use your data for advertising.</p>

<h2>Where it's stored</h2>
<p>Data is stored on Cloudflare's infrastructure (D1 database and KV store). Sensitive tokens are encrypted with AES-256-GCM.</p>

<h2>Third parties</h2>
<p>We interact with Meta's WhatsApp and Instagram APIs on your behalf. We use Cloudflare Workers AI for AI-powered auto-replies. No other third parties receive your data.</p>

<h2>Data retention</h2>
<p>Data is retained while your account is active. You can delete all your data at any time from <a href="/admin/settings">Settings</a>.</p>

<h2>Data deletion</h2>
<p>To delete your account and all associated data:</p>
<ul>
<li>Go to <a href="/admin/settings">Settings</a> and click "Delete Account"</li>
<li>Or remove the Concierge app from your <a href="https://www.facebook.com/settings?tab=business_tools">Facebook Business Integrations</a></li>
</ul>
<p>Deletion is immediate and irreversible.</p>

<h2>Contact</h2>
<p>Questions? Open an issue at <a href="https://github.com/ananthb/concierge-worker">github.com/ananthb/concierge-worker</a>.</p>

<footer><a href="/">Home</a> · <a href="/terms">Terms of Service</a></footer>
</body>
</html>"#,
        style = LEGAL_STYLE,
    )
}
