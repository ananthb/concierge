//! Public /features page: short, scannable overview of every capability.

use crate::i18n::{t, t_args};
use crate::locale::Locale;

use super::base::{base_html_with_meta, public_nav_html, PageMeta};

/// Format a milli-unit price (1/1000 of paisa or cent) as a major-currency
/// string with the `₹` or `$` prefix.
fn fmt_milli(milli: i64, symbol: &str, decimals: usize) -> String {
    format!("{}{:.*}", symbol, decimals, milli as f64 / 100_000.0)
}

pub fn features_html(locale: &Locale, milli_paise: i64, milli_cents: i64) -> String {
    let nav = public_nav_html("features", locale);
    let inr_price = fmt_milli(milli_paise, "₹", 2);
    let usd_price = fmt_milli(milli_cents, "$", 3);
    let price_args: &[(&str, &str)] = &[("inr", &inr_price), ("usd", &usd_price)];

    let content = format!(
        r##"{nav}
<article class="page narrow">
  <h1 class="display-md m-0">{headline}</h1>
  <p class="lead">{lead}</p>

  <h2 class="mt-32 mb-12">{channels_h}</h2>
  <div class="channels-grid">
    <div class="card p-22">
      <div class="eyebrow">{wa_eyebrow}</div>
      <p class="m-0 mt-8">{wa_body}</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">{ig_eyebrow}</div>
      <p class="m-0 mt-8">{ig_body}</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">{dc_eyebrow}</div>
      <p class="m-0 mt-8">{dc_body}</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">{em_eyebrow}</div>
      <p class="m-0 mt-8">{em_body}</p>
    </div>
  </div>

  <h2 class="mt-32 mb-12">{how_h}</h2>
  <div class="channels-grid">
    <div class="card p-22">
      <div class="eyebrow">{s1_eyebrow}</div>
      <p class="m-0 mt-8">{s1_body}</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">{s2_eyebrow}</div>
      <p class="m-0 mt-8">{s2_body}</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">{s3_eyebrow}</div>
      <p class="m-0 mt-8">{s3_body}</p>
    </div>
  </div>

  <h2 class="mt-32 mb-12">{trust_h}</h2>
  <div class="channels-grid">
    <div class="card p-22">
      <div class="eyebrow">{voice_eyebrow}</div>
      <p class="m-0 mt-8">{voice_body}</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">{inj_eyebrow}</div>
      <p class="m-0 mt-8">{inj_body}</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">{pay_eyebrow}</div>
      <p class="m-0 mt-8">{pay_body}</p>
    </div>
  </div>

  <h2 class="mt-32 mb-12">{more_h}</h2>
  <div class="channels-grid">
    <div class="card p-22">
      <div class="eyebrow">{leads_eyebrow}</div>
      <p class="m-0 mt-8">{leads_body}</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">{rec_eyebrow}</div>
      <p class="m-0 mt-8">{rec_body}</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">{priv_eyebrow}</div>
      <p class="m-0 mt-8">{priv_body}</p>
    </div>
    <div class="card p-22">
      <div class="eyebrow">{os_eyebrow}</div>
      <p class="m-0 mt-8">{os_body}</p>
    </div>
  </div>

  <section class="card p-22 mt-32 ta-center">
    <h2 class="m-0">{cta_h}</h2>
    <p class="muted mt-8 mb-16">{cta_body}</p>
    <div class="row gap-12 jc-center">
      <a href="/auth/login" class="btn primary lg">{cta_primary}</a>
      <a href="/pricing" class="btn ghost lg">{cta_secondary}</a>
    </div>
  </section>
</article>"##,
        nav = nav,
        headline = t(locale, "features-headline"),
        lead = t(locale, "features-lead"),
        channels_h = t(locale, "features-channels-heading"),
        wa_eyebrow = t(locale, "features-card-whatsapp-eyebrow"),
        wa_body = t(locale, "features-card-whatsapp-body"),
        ig_eyebrow = t(locale, "features-card-instagram-eyebrow"),
        ig_body = t(locale, "features-card-instagram-body"),
        dc_eyebrow = t(locale, "features-card-discord-eyebrow"),
        dc_body = t(locale, "features-card-discord-body"),
        em_eyebrow = t(locale, "features-card-email-eyebrow"),
        em_body = t(locale, "features-card-email-body"),
        how_h = t(locale, "features-how-heading"),
        s1_eyebrow = t(locale, "features-card-step-1-eyebrow"),
        s1_body = t(locale, "features-card-step-1-body"),
        s2_eyebrow = t(locale, "features-card-step-2-eyebrow"),
        s2_body = t(locale, "features-card-step-2-body"),
        s3_eyebrow = t(locale, "features-card-step-3-eyebrow"),
        s3_body = t(locale, "features-card-step-3-body"),
        trust_h = t(locale, "features-trust-heading"),
        voice_eyebrow = t(locale, "features-card-voice-eyebrow"),
        voice_body = t(locale, "features-card-voice-body"),
        inj_eyebrow = t(locale, "features-card-injection-eyebrow"),
        inj_body = t(locale, "features-card-injection-body"),
        pay_eyebrow = t(locale, "features-card-pay-eyebrow"),
        pay_body = t_args(locale, "features-card-pay-body", price_args),
        more_h = t(locale, "features-more-heading"),
        leads_eyebrow = t(locale, "features-card-leads-eyebrow"),
        leads_body = t(locale, "features-card-leads-body"),
        rec_eyebrow = t(locale, "features-card-recipients-eyebrow"),
        rec_body = t(locale, "features-card-recipients-body"),
        priv_eyebrow = t(locale, "features-card-privacy-eyebrow"),
        priv_body = t(locale, "features-card-privacy-body"),
        os_eyebrow = t(locale, "features-card-os-eyebrow"),
        os_body = t(locale, "features-card-os-body"),
        cta_h = t(locale, "features-cta-heading"),
        cta_body = t(locale, "features-cta-body"),
        cta_primary = t(locale, "features-cta-primary"),
        cta_secondary = t(locale, "features-cta-secondary"),
    );

    base_html_with_meta(
        "Features - Concierge",
        &content,
        &PageMeta {
            description: t_args(locale, "features-meta-description", price_args),
            og_title: t(locale, "features-og-title"),
            og_type: "website",
        },
        locale,
    )
}
