//! Credit-purchase slider: used on /pricing, /dashboard/billing, and the wizard
//! launch step so the buying experience is identical everywhere.
//!
//! Bounds (`min_credits` / `max_credits`) come from `pricing_config` so
//! operators can tune them from /manage/billing without a code deploy.
//! The slider runs `min..SLIDER_MAX`; past the right edge a "Custom"
//! toggle swaps in a number input that accepts integers up to `max`.

use crate::helpers::{format_count, format_money};
use crate::locale::{Currency, Locale};

/// Cap for the dragged slider; beyond this the operator switches to the
/// custom number input. Pure UI knob; the real upper limit is the
/// operator-configured `pricing_config.max_credits`.
pub const SLIDER_MAX: i64 = 10_000;

/// Variant of the slider: controls the bottom action area.
pub enum SliderMode<'a> {
    /// Renders a Buy button that POSTs to /dashboard/billing/checkout. Logged-in tenants.
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
    min_credits: i64,
    max_credits: i64,
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
    let initial = min_credits;
    // The slider's right edge is the smaller of SLIDER_MAX and the
    // operator-configured ceiling — when an operator drops the cap to
    // 5,000 we don't want the slider extending past it.
    let slider_top = SLIDER_MAX.min(max_credits).max(min_credits);
    // The "custom" path picks up where the slider ends and runs to the
    // operator-set ceiling.
    let custom_min = slider_top;

    let action_html = match mode {
        SliderMode::Buy { return_to } => format!(
            r##"<form hx-post="{base_url}/dashboard/billing/checkout" hx-ext="json-enc" hx-target="body" hx-swap="innerHTML" class="mt-16">
  <input type="hidden" name="credits" :value="credits">
  <input type="hidden" name="return_to" value="{return_to}">
  <button type="submit" class="btn primary lg w-full"><span>Buy <span x-text="credits.toLocaleString(countLocale)"></span> replies: {symbol}<span x-text="{price_js}"></span></span><span class="spinner htmx-indicator" aria-hidden="true"></span></button>
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

    // Hide the "Need more?" / custom path when the operator-configured
    // ceiling already fits inside the slider.
    let show_custom = max_credits > slider_top;

    let custom_link = if show_custom {
        format!(
            r##"<span><a href="#" class="muted" @click.prevent="custom = true; if (credits < {custom_min}) credits = {custom_min}">Need more?</a></span>"##,
            custom_min = custom_min,
        )
    } else {
        String::new()
    };

    let custom_pane = if show_custom {
        format!(
            r##"<div x-show="custom" x-cloak>
    <input type="number" min="{custom_min}" max="{max}" step="1"
           x-model.number="credits"
           @input="credits = Math.max({custom_min}, Math.min({max}, parseInt($el.value) || {custom_min}))"
           class="input mono"
           placeholder="How many replies?">
    <div class="between mt-4 mono muted fs-11">
      <span>min {custom_min_display}, max {max_display}</span>
      <span><a href="#" class="muted" @click.prevent="custom = false; if (credits > {slider_top}) credits = {slider_top}">Use the slider</a></span>
    </div>
  </div>"##,
            custom_min = custom_min,
            max = max_credits,
            custom_min_display = format_count(custom_min, locale),
            max_display = format_count(max_credits, locale),
            slider_top = slider_top,
        )
    } else {
        String::new()
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
    <input type="range" min="{min}" max="{slider_top}" step="100"
           x-model.number="credits"
           class="w-full"
           style="accent-color:var(--accent)">
    <div class="between mt-4 mono muted fs-11">
      <span>{min_price}</span>
      {custom_link}
      <span>{slider_top_price}</span>
    </div>
  </div>

  {custom_pane}

  <div class="ta-center mt-16 fs-14">
    Total: <strong>{symbol}<span x-text="{price_js}"></span></strong>
  </div>

  {action_html}
</div>"##,
        initial = initial,
        per_reply_label = per_reply_label,
        symbol = symbol,
        min = min_credits,
        slider_top = slider_top,
        min_price = price_for(min_credits, locale, milli_price),
        slider_top_price = price_for(slider_top, locale, milli_price),
        price_js = price_js,
        count_locale = count_locale,
        custom_link = custom_link,
        custom_pane = custom_pane,
        action_html = action_html,
    )
}

/// Total price for `credits` reply units in the given locale.
fn price_for(credits: i64, locale: &Locale, milli_price: i64) -> String {
    let amount_minor = crate::billing::calculate_total(credits, milli_price);
    format_money(amount_minor, locale)
}
