//! Credit-purchase slider: used on /pricing, /admin/billing, and the wizard
//! launch step so the buying experience is identical everywhere.
//!
//! Flat per-reply rate. Slider runs `MIN_CREDITS`..`SLIDER_MAX` step 100.
//! Past the right edge, a "Custom" toggle swaps in a number input that
//! accepts integers between `SLIDER_MAX` and `MAX_CREDITS`. Live price
//! preview is computed in Alpine on the client.

use crate::billing::{MAX_CREDITS, MIN_CREDITS};
use crate::helpers::{format_count, format_money};
use crate::locale::{Currency, Locale};

/// Upper bound of the slider. Past this, the "choose your own amount" box
/// takes over; its minimum is this value, its maximum is `MAX_CREDITS`.
pub const SLIDER_MAX: i64 = 10_000;

/// Variant of the slider: controls the bottom action area.
pub enum SliderMode<'a> {
    /// Renders a Buy button that POSTs to /admin/billing/checkout. Logged-in tenants.
    Buy { return_to: &'a str },
    /// Renders no action: just shows the slider + price. Used on the public
    /// pricing page where unauthenticated visitors can play with the slider.
    Preview {
        cta_href: &'a str,
        cta_label: &'a str,
    },
}

pub fn slider_html(
    locale: &Locale,
    base_url: &str,
    mode: SliderMode<'_>,
    milli_price: i64,
) -> String {
    // Per-reply price label and the JS expression for live total. INR uses
    // `toLocaleString('en-IN')` for lakh/crore grouping; USD does standard
    // dollars-and-cents.
    // milli_price is 1/1000th of a cent/paisa.
    // 100,000 milli-units = 100 cents/paise = 1 dollar/rupee.
    let (per_reply_label, price_js) = match locale.currency {
        Currency::Usd => (
            format!(
                "{}{:.3}",
                locale.currency.symbol(),
                milli_price as f64 / 100000.0
            ),
            format!("(credits * {} / 100000).toFixed(2)", milli_price),
        ),
        Currency::Inr => (
            format!(
                "{}{:.2}",
                locale.currency.symbol(),
                milli_price as f64 / 100000.0
            ),
            format!(
                "(credits * {} / 100000).toLocaleString('en-IN')",
                milli_price
            ),
        ),
    };
    let symbol = locale.currency.symbol();
    let count_locale = locale.langid.to_string(); // "en-IN" / "en-US"
                                                  // Initial slider step value seeded server-side to match minimum.
    let initial = MIN_CREDITS.max(500); // start at a friendlier default

    let action_html = match mode {
        SliderMode::Buy { return_to } => format!(
            r##"<form hx-post="{base_url}/admin/billing/checkout" hx-ext="json-enc" hx-target="body" hx-swap="innerHTML" class="mt-16">
  <input type="hidden" name="credits" :value="credits">
  <input type="hidden" name="return_to" value="{return_to}">
  <button type="submit" class="btn primary lg w-full">Buy <span x-text="credits.toLocaleString(countLocale)"></span> replies: {symbol}<span x-text="{price_js}"></span></button>
</form>"##,
            base_url = base_url,
            return_to = return_to,
            symbol = symbol,
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
        r##"<div x-data="{{ credits: {initial}, custom: false, countLocale: '{count_locale}' }}" class="card p-22">
  <div class="between mb-12">
    <div>
      <div class="eyebrow">AI reply credits</div>
      <p class="muted m-0 mt-4 fs-13">{per_reply_label} per AI reply.</p>
    </div>
    <div class="ta-right">
      <div class="serif" style="font-size:34px;line-height:1"><span x-text="credits.toLocaleString(countLocale)"></span></div>
      <div class="mono muted fs-11">replies</div>
    </div>
  </div>

  <div x-show="!custom" x-cloak>
    <input type="range" min="{min}" max="{slider_max}" step="100"
           x-model.number="credits"
           class="w-full"
           style="accent-color:var(--accent)">
    <div class="between mt-4 mono muted fs-11">
      <span>{min_price}</span>
      <span><a href="#" class="muted" @click.prevent="custom = true; if (credits < {custom_min}) credits = {custom_min}">Need more?</a></span>
      <span>{slider_max_price}</span>
    </div>
  </div>

  <div x-show="custom" x-cloak>
    <input type="number" min="{custom_min}" max="{max}" step="1"
           x-model.number="credits"
           class="input mono"
           placeholder="How many replies?">
    <div class="between mt-4 mono muted fs-11">
      <span>min {custom_min_display}, max {max_display}</span>
      <span><a href="#" class="muted" @click.prevent="custom = false; if (credits > {slider_max}) credits = {slider_max}">Use the slider</a></span>
    </div>
  </div>

  <div class="ta-center mt-16 fs-14">
    Total: <strong>{symbol}<span x-text="{price_js}"></span></strong>
  </div>

  {action_html}
</div>"##,
        initial = initial,
        per_reply_label = per_reply_label,
        symbol = symbol,
        min = MIN_CREDITS,
        slider_max = SLIDER_MAX,
        custom_min = SLIDER_MAX,
        custom_min_display = format_count(SLIDER_MAX, locale),
        max = MAX_CREDITS,
        max_display = format_count(MAX_CREDITS, locale),
        min_price = price_for(MIN_CREDITS, locale, milli_price),
        slider_max_price = price_for(SLIDER_MAX, locale, milli_price),
        price_js = price_js,
        count_locale = count_locale,
        action_html = action_html,
    )
}

/// Total price for `credits` reply units in the given locale.
fn price_for(credits: i64, locale: &Locale, milli_price: i64) -> String {
    let amount_minor = crate::billing::calculate_total(credits, milli_price);
    format_money(amount_minor, locale)
}
