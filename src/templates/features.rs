//! Public /features page: short, scannable overview of every capability.

use super::base::{base_html_with_meta, footer, public_nav_html, PageMeta};

pub fn features_html() -> String {
    let nav = public_nav_html("features");
    let foot = footer();

    let content = format!(
        r##"{nav}
<article class="page narrow">
  <h1 class="display-md m-0">One assistant. Every channel.</h1>
  <p class="lead">Concierge replies for you on WhatsApp, Instagram, Discord, and email: instantly, in your voice, and only when it should.</p>

  <h2 class="mt-32 mb-12">Channels</h2>
  <div class="channels-grid">
    <div class="card p-22">
      <div class="eyebrow">WhatsApp Business</div>
      <p class="m-0 mt-8">Connect a number, pick a tone, ship. Customer messages get an instant static or AI reply.</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">Instagram DMs</div>
      <p class="m-0 mt-8">Sign in with Meta, choose the business account. Replies go out through the official Graph API.</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">Discord</div>
      <p class="m-0 mt-8">Install the bot. The concierge replies when @-mentioned or in channels you designate.</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">Email</div>
      <p class="m-0 mt-8">Pick a name at <code>name@cncg.email</code>. Inbound mail gets a reply; you and your team get a copy via Cc/Bcc.</p>
    </div>
  </div>

  <h2 class="mt-32 mb-12">How it works</h2>
  <div class="channels-grid">
    <div class="card p-22">
      <div class="eyebrow">1. Connect</div>
      <p class="m-0 mt-8">5-minute wizard. OAuth into the channels you use; everything else stays untouched.</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">2. Configure</div>
      <p class="m-0 mt-8">Pick static or AI replies per channel. Set a persona: tone, business type, things to never say.</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">3. Run</div>
      <p class="m-0 mt-8">Inbound messages flow through. Bursts collapse into one reply (configurable wait: default 5s).</p>
    </div>
  </div>

  <h2 class="mt-32 mb-12">AI you can trust</h2>
  <div class="channels-grid">
    <div class="card p-22">
      <div class="eyebrow">Voice and guardrails</div>
      <p class="m-0 mt-8">Set tone, biz type, and things the AI must never do. Every reply respects those rules.</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">Prompt-injection screening</div>
      <p class="m-0 mt-8">Inbound text is scanned before it reaches the model. Suspicious messages are skipped, not auto-replied.</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">Pay only for AI</div>
      <p class="m-0 mt-8">Static auto-replies are free, forever. AI replies cost ₹2 / $0.02 each, with 100 free per month.</p>
    </div>
  </div>

  <h2 class="mt-32 mb-12">More that just works</h2>
  <div class="channels-grid">
    <div class="card p-22">
      <div class="eyebrow">Lead capture</div>
      <p class="m-0 mt-8">Embed a phone-number form on any page. Submissions trigger an instant WhatsApp message.</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">Notification recipients</div>
      <p class="m-0 mt-8">Add Cc/Bcc emails to any concierge address. We verify each one with a one-click link.</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">Privacy by default</div>
      <p class="m-0 mt-8">We log metadata, never message bodies. Account deletion wipes everything, immediately.</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">Open source</div>
      <p class="m-0 mt-8">AGPL-3.0 on <a href="https://github.com/ananthb/concierge" target="_blank" rel="noopener">GitHub</a>. Self-host if you'd rather. <a href="https://ananthb.github.io/concierge/" target="_blank" rel="noopener">Architecture docs</a>.</p>
    </div>
  </div>

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
            description: "Concierge auto-replies on WhatsApp, Instagram, Discord, and email. Static replies free; AI replies ₹2 / $0.02 with 100 free per month. 5-minute setup. Open source.",
            og_title: "Concierge Features",
            ..PageMeta::default()
        },
    )
}
