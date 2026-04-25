//! Public /features page — marketing-voiced overview of every capability.

use super::base::{base_html_with_meta, footer, public_nav_html, PageMeta};

pub fn features_html() -> String {
    let nav = public_nav_html("features");
    let foot = footer();

    let content = format!(
        r##"{nav}
<article class="legal">
  <h1 class="display-md m-0">Everything Concierge does for your business.</h1>
  <p class="lead">One assistant that lives where your customers already are — WhatsApp, Instagram, Discord, email — replying when you can't, escalating when it matters.</p>

  <section class="mt-32">
    <h2>WhatsApp Auto-Reply</h2>
    <p>Connect your WhatsApp Business number once. Every customer message gets an instant response — either a canned reply you wrote yourself or an AI-drafted answer in your voice. You stay in the loop on Discord; the AI does the typing.</p>
  </section>

  <section class="mt-32">
    <h2>Instagram DMs</h2>
    <p>Same flow as WhatsApp, on the other half of your customer base. Sign in with Meta, pick the business account, done. Replies post via the Instagram Graph API; nothing scrapes the app.</p>
  </section>

  <section class="mt-32">
    <h2>Discord — relay and reply</h2>
    <p>Install the Concierge bot in your team's server. Every WhatsApp / Instagram / Email message lands in a Discord channel with Reply, Approve, and Drop buttons. Reply right there and it flows back to the customer through the right channel. The bot can also answer customer messages directly when you @-mention it or in channels you designate.</p>
  </section>

  <section class="mt-32">
    <h2>Email routing on your own domain</h2>
    <p>Get <code>yourname.cncg.email</code> in under a minute. Send mail to any address on it — <code>hello@</code>, <code>orders@</code>, <code>support+invoice@</code> — and write rules to forward, reject, AI-reply, or relay to Discord. Every routing rule is glob-pattern matched and prioritized; the first one that fits wins.</p>
  </section>

  <section class="mt-32">
    <h2>AI replies with your voice</h2>
    <p>Set a tone, a business type, and the things the AI should never do (quote prices, promise dates, handle refunds). Every AI draft respects those guardrails, and incoming text is scanned for prompt-injection attempts before it hits the model.</p>
  </section>

  <section class="mt-32">
    <h2>Approval workflow</h2>
    <p>The AI doesn't send anything you haven't seen. Drafts post to Discord (or arrive as an email digest) with Approve / Reject / Edit buttons. You stay in control; the AI just saves you the typing.</p>
  </section>

  <section class="mt-32">
    <h2>Smart batching</h2>
    <p>If a customer fires off five messages in a row, Concierge waits a few seconds for them to finish, then sends one combined reply. Configurable per channel; default 5 seconds. The AI sees the whole burst.</p>
  </section>

  <section class="mt-32">
    <h2>Lead capture forms</h2>
    <p>Drop an embeddable phone-number form on any page. When someone fills it in, the concierge messages them on WhatsApp instantly. Style it to match your brand; restrict where it can be embedded.</p>
  </section>

  <section class="mt-32">
    <h2>Notifications you control</h2>
    <p>Pick where AI approval requests show up — Discord, email digest, both. Same for activity summaries (daily / weekly / monthly / quarterly). Mute the channels you don't want, no all-or-nothing toggle.</p>
  </section>

  <section class="mt-32">
    <h2>Privacy-first</h2>
    <p>We don't store message content. Metadata only — who messaged whom, when, what action ran. Account deletion wipes everything we have on you, immediately, with no support ticket.</p>
  </section>

  <section class="mt-32">
    <h2>Open source</h2>
    <p>The whole thing is on <a href="https://github.com/ananthb/concierge-worker" target="_blank" rel="noopener">GitHub</a> under AGPL-3.0. Self-host on your own Cloudflare account if you'd rather. Read the <a href="/docs">technical docs</a> for the full architecture.</p>
  </section>

  <section class="card p-22 mt-32 ta-center">
    <h2 class="m-0">Ready to set this up?</h2>
    <p class="muted mt-8 mb-16">Sign in with Google or Facebook. The wizard takes 5 minutes.</p>
    <div class="row gap-12 jc-center">
      <a href="/auth/login" class="btn primary lg">Get started &rarr;</a>
      <a href="/pricing" class="btn ghost lg">See pricing</a>
    </div>
  </section>
</article>
{foot}"##,
        nav = nav,
        foot = foot,
    );

    base_html_with_meta(
        "Features - Concierge",
        &content,
        &PageMeta {
            description: "Concierge handles WhatsApp, Instagram, Discord, and email auto-replies for small businesses. AI drafts, human approvals, smart batching, and a unified Discord inbox.",
            og_title: "Concierge Features",
            ..PageMeta::default()
        },
    )
}
