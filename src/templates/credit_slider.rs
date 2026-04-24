//! Credit-purchase slider — used on /pricing, /admin/billing, and the wizard
//! launch step so the buying experience is identical everywhere.
//!
//! Flat per-reply rate. Slider 100..10000 step 100. Past the right edge, a
//! "Custom" toggle swaps in a number input that accepts any integer up to
//! `MAX_CREDITS`. Live price preview is computed in Alpine on the client.

use crate::billing::{MAX_CREDITS, MIN_CREDITS, UNIT_PRICE_CENTS, UNIT_PRICE_PAISE};

/// Variant of the slider — controls the bottom action area.
pub enum SliderMode<'a> {
    /// Renders a Buy button that POSTs to /admin/billing/checkout. Logged-in tenants.
    Buy { return_to: &'a str },
    /// Renders no action — just shows the slider + price. Used on the public
    /// pricing page where unauthenticated visitors can play with the slider.
    Preview {
        cta_href: &'a str,
        cta_label: &'a str,
    },
}

pub fn slider_html(currency: &str, base_url: &str, mode: SliderMode<'_>) -> String {
    let (sym, per_reply_label, price_js) = if currency == "USD" {
        // Display dollars with two decimals.
        ("$", "$0.02", "(credits * 2 / 100).toFixed(2)")
    } else {
        // Whole rupees only.
        ("₹", "₹2", "(credits * 2).toLocaleString()")
    };
    // Initial slider step value seeded server-side to match minimum.
    let initial = MIN_CREDITS.max(500); // start at a friendlier default

    let action_html = match mode {
        SliderMode::Buy { return_to } => format!(
            r##"<form hx-post="{base_url}/admin/billing/checkout" hx-ext="json-enc" hx-target="body" hx-swap="innerHTML" class="mt-16">
  <input type="hidden" name="credits" :value="credits">
  <input type="hidden" name="return_to" value="{return_to}">
  <button type="submit" class="btn primary lg w-full">Buy <span x-text="credits.toLocaleString()"></span> replies — {sym}<span x-text="{price_js}"></span></button>
</form>"##,
            base_url = base_url,
            return_to = return_to,
            sym = sym,
            price_js = price_js,
        ),
        SliderMode::Preview {
            cta_href,
            cta_label,
        } => format!(
            r##"<a href="{cta_href}" class="btn primary lg w-full jc-center mt-16">{cta_label}</a>"##,
            cta_href = cta_href,
            cta_label = cta_label,
        ),
    };

    format!(
        r##"<div x-data="{{ credits: {initial}, custom: false }}" class="card p-22">
  <div class="between mb-12">
    <div>
      <div class="eyebrow">Reply credits</div>
      <p class="muted m-0 mt-4 fs-13">{per_reply_label} per reply. 100 free every month. Purchased credits never expire.</p>
    </div>
    <div class="ta-right">
      <div class="serif" style="font-size:34px;line-height:1"><span x-text="credits.toLocaleString()"></span></div>
      <div class="mono muted fs-11">replies</div>
    </div>
  </div>

  <div x-show="!custom" x-cloak>
    <input type="range" min="{min}" max="10000" step="100"
           x-model.number="credits"
           class="w-full"
           style="accent-color:var(--accent)">
    <div class="between mt-4 mono muted fs-11">
      <span>{sym}{min_price}</span>
      <span><a href="#" class="muted" @click.prevent="custom = true; if (credits < {min}) credits = {min}">Need more?</a></span>
      <span>{sym}{max_price}</span>
    </div>
  </div>

  <div x-show="custom" x-cloak>
    <input type="number" min="{min}" max="{max}" step="1"
           x-model.number="credits"
           class="input mono"
           placeholder="How many replies?">
    <div class="between mt-4 mono muted fs-11">
      <span>min {min}, max {max_display}</span>
      <span><a href="#" class="muted" @click.prevent="custom = false; if (credits > 10000) credits = 10000">Use the slider</a></span>
    </div>
  </div>

  <div class="ta-center mt-16 fs-14">
    Total: <strong>{sym}<span x-text="{price_js}"></span></strong>
  </div>

  {action_html}
</div>"##,
        initial = initial,
        sym = sym,
        per_reply_label = per_reply_label,
        min = MIN_CREDITS,
        max = MAX_CREDITS,
        max_display = format_with_commas(MAX_CREDITS),
        min_price = format_price(MIN_CREDITS, currency),
        max_price = format_price(10_000, currency),
        price_js = price_js,
        action_html = action_html,
    )
}

fn format_with_commas(n: i64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

fn format_price(credits: i64, currency: &str) -> String {
    if currency == "USD" {
        let cents = credits * UNIT_PRICE_CENTS;
        format!("{}.{:02}", cents / 100, cents % 100)
    } else {
        format_with_commas(credits * UNIT_PRICE_PAISE / 100)
    }
}
