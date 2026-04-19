//! Legal pages: Terms of Service and Privacy Policy

use crate::templates::base::{base_html_with_meta, brand_mark, PageMeta};

pub fn terms_of_service_html() -> String {
    let content = format!(
        r##"<header style="display:flex;align-items:center;gap:28px;padding:18px 28px;border-bottom:1px solid var(--hair);background:var(--paper)">
  {brand}
  <div style="margin-left:auto"><a href="/" class="btn ghost sm">&larr; Home</a></div>
</header>
<article class="legal">
  <h1>Terms of Service</h1>
  <p class="muted">Effective April 4, 2026</p>

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
</article>"##,
        brand = brand_mark(),
    );

    base_html_with_meta(
        "Terms of Service | Concierge",
        &content,
        &PageMeta {
            description: "Terms of Service for Concierge, an automated messaging platform for small businesses.",
            og_title: "Terms of Service | Concierge",
            ..PageMeta::default()
        },
    )
}

pub fn privacy_policy_html() -> String {
    let content = format!(
        r##"<header style="display:flex;align-items:center;gap:28px;padding:18px 28px;border-bottom:1px solid var(--hair);background:var(--paper)">
  {brand}
  <div style="margin-left:auto"><a href="/" class="btn ghost sm">&larr; Home</a></div>
</header>
<article class="legal">
  <h1>Privacy Policy</h1>
  <p class="muted">Effective April 4, 2026</p>

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
</article>"##,
        brand = brand_mark(),
    );

    base_html_with_meta(
        "Privacy Policy | Concierge",
        &content,
        &PageMeta {
            description: "Privacy Policy for Concierge. We store the minimum data needed to operate and you can delete everything at any time.",
            og_title: "Privacy Policy | Concierge",
            ..PageMeta::default()
        },
    )
}
