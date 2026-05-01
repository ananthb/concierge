//! Billing templates: tenant-facing credit balance + purchase

use crate::helpers::{format_money, html_escape};
use crate::locale::Locale;
use crate::types::{CreditSource, TenantBilling};

use super::base::{app_shell, base_html};
use super::credit_slider::{slider_html, SliderMode};

/// Summary of credits by source, computed from the ledger.
struct CreditSummary {
    total: i64,
    free: i64,
    free_expires: Option<String>,
    purchased: i64,
    granted: i64,
    granted_earliest_expiry: Option<String>,
}

fn summarize(billing: &TenantBilling) -> CreditSummary {
    let mut free = 0i64;
    let mut free_expires: Option<String> = None;
    let mut purchased = 0i64;
    let mut granted = 0i64;
    let mut granted_earliest: Option<String> = None;

    for entry in &billing.credits {
        match entry.source {
            CreditSource::FreeMonthly => {
                free += entry.amount;
                if let Some(exp) = &entry.expires_at {
                    free_expires = Some(exp.clone());
                }
            }
            CreditSource::Purchase => {
                purchased += entry.amount;
            }
            CreditSource::Grant => {
                granted += entry.amount;
                if let Some(exp) = &entry.expires_at {
                    match &granted_earliest {
                        None => granted_earliest = Some(exp.clone()),
                        Some(existing) if exp < existing => granted_earliest = Some(exp.clone()),
                        _ => {}
                    }
                }
            }
        }
    }

    CreditSummary {
        total: free + purchased + granted,
        free,
        free_expires,
        purchased,
        granted,
        granted_earliest_expiry: granted_earliest,
    }
}

fn format_expiry(iso: &str) -> String {
    // Extract YYYY-MM-DD from ISO string for display
    if iso.len() >= 10 {
        iso[..10].to_string()
    } else {
        iso.to_string()
    }
}

/// Renders the billing dashboard, including the email address-quota card.
pub fn billing_overview_with_addresses_html(
    billing: &TenantBilling,
    locale: &Locale,
    base_url: &str,
    addresses_used: u32,
    address_quota: u32,
    milli_price: i64,
    address_price: i64,
    email_pack_size: i64,
) -> String {
    let summary = summarize(billing);

    let total_class = if summary.total <= 0 { " text-warn" } else { "" };

    let free_detail = match &summary.free_expires {
        Some(exp) => format!(
            r#"<div class="mono muted fs-11">expires {}</div>"#,
            format_expiry(exp)
        ),
        None => String::new(),
    };

    let granted_detail = if summary.granted > 0 {
        match &summary.granted_earliest_expiry {
            Some(exp) => format!(
                r#"<div class="mono muted fs-11">earliest expiry {}</div>"#,
                format_expiry(exp)
            ),
            None => String::new(),
        }
    } else {
        String::new()
    };

    let slider = slider_html(
        locale,
        base_url,
        SliderMode::Buy {
            return_to: "/admin/billing",
        },
        milli_price,
    );

    let address_price_label = format_money(address_price, locale);
    let address_card = format!(
        r##"<div class="card p-18 mb-24">
            <div class="between">
                <div>
                    <div class="eyebrow">Reply-email subscription</div>
                    <div class="stat-n serif">{addresses_used} / {address_quota}</div>
                    <div class="mono muted fs-11">addresses used / quota</div>
                </div>
                <form hx-post="{base_url}/admin/billing/address" hx-target="body" hx-swap="innerHTML">
                    <button class="btn primary" type="submit">Add a pack ({pack_size} for {address_price_label}/mo)</button>
                </form>
            </div>
        </div>"##,
        addresses_used = addresses_used,
        address_quota = address_quota,
        address_price_label = address_price_label,
        pack_size = email_pack_size,
        base_url = base_url,
    );

    let content = format!(
        r##"<div class="page-pad">
  <p><a href="{base_url}/admin">&larr; Back to Dashboard</a></p>
  <div class="eyebrow">Billing</div>
  <h2 class="display-sm m-0 mt-4 mb-16">AI reply credits</h2>
  {address_card}

  <div class="stats-grid mb-24">
    <div class="card p-18 ta-center">
      <div class="stat-n serif{total_class}">{total}</div>
      <div class="mono muted fs-11">Total remaining</div>
    </div>
    <div class="card p-18 ta-center">
      <div class="stat-n serif">{free}</div>
      <div class="mono muted fs-11">Free this month</div>
      {free_detail}
    </div>
    <div class="card p-18 ta-center">
      <div class="stat-n serif">{purchased}</div>
      <div class="mono muted fs-11">Purchased</div>
      <div class="mono muted fs-11">never expire</div>
    </div>
    <div class="card p-18 ta-center">
      <div class="stat-n serif">{granted}</div>
      <div class="mono muted fs-11">Granted</div>
      {granted_detail}
    </div>
    <div class="card p-18 ta-center">
      <div class="stat-n serif">{used}</div>
      <div class="mono muted fs-11">Replies sent</div>
    </div>
  </div>

  {slider}
</div>"##,
        base_url = base_url,
        total = summary.total,
        total_class = total_class,
        free = summary.free,
        free_detail = free_detail,
        purchased = summary.purchased,
        granted = summary.granted,
        granted_detail = granted_detail,
        used = billing.replies_used,
        slider = slider,
        address_card = address_card,
    );

    let page = app_shell(&content, "Billing", base_url, locale);
    base_html("Billing - Concierge", &page, locale)
}

pub fn checkout_html(
    order_id: &str,
    amount: i64,
    locale: &Locale,
    credits: i64,
    razorpay_key: &str,
    tenant_id: &str,
    return_to: &str,
    base_url: &str,
) -> String {
    let currency = locale.currency.as_str();
    let display_amount = format_money(amount, locale);

    let content = format!(
        r##"<div class="ta-center" style="max-width:480px;margin:4rem auto;padding:0 1rem">
  <div class="card p-28">
    <h2 class="display-sm">Complete purchase</h2>
    <p class="muted m-0 mt-8 mb-24">Buying <strong>{credits}</strong> AI reply credits</p>
    <div class="stat-n serif mb-24">{display_amount}</div>
    <button id="pay-btn" class="btn primary lg w-full">Pay with Razorpay</button>
    <p class="mono muted fs-11 mt-12">Secure payment via Razorpay</p>
  </div>
  <a href="{base_url}{return_to}" class="btn ghost sm mt-16">&larr; Cancel</a>
</div>
<script src="https://checkout.razorpay.com/v1/checkout.js"></script>
<script type="module">
document.getElementById('pay-btn').addEventListener('click', () => {{
  const options = {{
    key: '{key}',
    amount: {amount},
    currency: '{currency}',
    order_id: '{order_id}',
    name: 'Concierge',
    description: '{credits} AI reply credits',
    notes: {{ tenant_id: '{tenant_id}', credits: '{credits}' }},
    handler: async (response) => {{
      await fetch('{base_url}/admin/billing/verify', {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify({{
          razorpay_order_id: response.razorpay_order_id,
          razorpay_payment_id: response.razorpay_payment_id,
          razorpay_signature: response.razorpay_signature,
          credits: '{credits}',
        }}),
      }});
      window.location.href = '{base_url}{return_to}';
    }},
    theme: {{ color: '#E86A2C' }},
  }};
  new Razorpay(options).open();
}});
</script>"##,
        credits = credits,
        display_amount = display_amount,
        amount = amount,
        currency = currency,
        order_id = html_escape(order_id),
        key = html_escape(razorpay_key),
        tenant_id = html_escape(tenant_id),
        base_url = base_url,
        return_to = return_to,
    );

    base_html("Checkout - Concierge", &content, locale)
}

/// Checkout for the sign-up verification charge. Razorpay captures the
/// amount on a real card; the webhook records the capture, flips
/// `tenants.verified_at`, and immediately refunds. The user lands back on
/// `return_to` (the wizard launch step), which now lets them hit Finish.
pub fn verification_checkout_html(
    order_id: &str,
    amount: i64,
    locale: &Locale,
    razorpay_key: &str,
    tenant_id: &str,
    return_to: &str,
    base_url: &str,
) -> String {
    let currency = locale.currency.as_str();
    let display_amount = format_money(amount, locale);
    let content = format!(
        r##"<div class="ta-center" style="max-width:480px;margin:4rem auto;padding:0 1rem">
  <div class="card p-28">
    <h2 class="display-sm">Verify your account</h2>
    <p class="muted m-0 mt-8 mb-24">We charge a small amount to confirm a real card and then refund it right away. This keeps the platform free of abuse.</p>
    <div class="stat-n serif mb-8">{display_amount}</div>
    <p class="mono muted fs-11 mb-24">refunded automatically</p>
    <button id="pay-btn" class="btn primary lg w-full">Verify with Razorpay</button>
    <p class="mono muted fs-11 mt-12">Secure payment via Razorpay</p>
  </div>
  <a href="{base_url}{return_to}" class="btn ghost sm mt-16">&larr; Cancel</a>
</div>
<script src="https://checkout.razorpay.com/v1/checkout.js"></script>
<script type="module">
document.getElementById('pay-btn').addEventListener('click', () => {{
  const options = {{
    key: '{key}',
    amount: {amount},
    currency: '{currency}',
    order_id: '{order_id}',
    name: 'Concierge',
    description: 'Sign-up verification (refunded)',
    notes: {{ tenant_id: '{tenant_id}', kind: 'verification' }},
    handler: async (response) => {{
      await fetch('{base_url}/admin/billing/verify', {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify({{
          razorpay_order_id: response.razorpay_order_id,
          razorpay_payment_id: response.razorpay_payment_id,
          razorpay_signature: response.razorpay_signature,
        }}),
      }});
      window.location.href = '{base_url}{return_to}';
    }},
    theme: {{ color: '#E86A2C' }},
  }};
  new Razorpay(options).open();
}});
</script>"##,
        display_amount = display_amount,
        amount = amount,
        currency = currency,
        order_id = html_escape(order_id),
        key = html_escape(razorpay_key),
        tenant_id = html_escape(tenant_id),
        base_url = base_url,
        return_to = return_to,
    );

    base_html("Verify account: Concierge", &content, locale)
}

/// Checkout for a reply-email subscription pack. The price comes from
/// `pricing_config.address_price_*` (default ₹99 / $1 per pack/month) and
/// grants `email_pack_size` addresses (default 5) on payment success.
pub fn address_checkout_html(
    order_id: &str,
    amount: i64,
    locale: &Locale,
    razorpay_key: &str,
    tenant_id: &str,
    base_url: &str,
) -> String {
    let currency = locale.currency.as_str();
    let display_amount = format_money(amount, locale);
    let content = format!(
        r##"<div class="ta-center" style="max-width:480px;margin:4rem auto;padding:0 1rem">
  <div class="card p-28">
    <h2 class="display-sm">Reply-email subscription</h2>
    <p class="muted m-0 mt-8 mb-24">A pack of concierge addresses, billed monthly.</p>
    <div class="stat-n serif mb-24">{display_amount}</div>
    <button id="pay-btn" class="btn primary lg w-full">Pay with Razorpay</button>
    <p class="mono muted fs-11 mt-12">Secure payment via Razorpay</p>
  </div>
  <a href="{base_url}/admin/email" class="btn ghost sm mt-16">&larr; Cancel</a>
</div>
<script src="https://checkout.razorpay.com/v1/checkout.js"></script>
<script type="module">
document.getElementById('pay-btn').addEventListener('click', () => {{
  const options = {{
    key: '{key}',
    amount: {amount},
    currency: '{currency}',
    order_id: '{order_id}',
    name: 'Concierge',
    description: 'Extra email address',
    notes: {{ tenant_id: '{tenant_id}', kind: 'address', extras: '1' }},
    handler: async (response) => {{
      await fetch('{base_url}/admin/billing/verify', {{
        method: 'POST',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify({{
          razorpay_order_id: response.razorpay_order_id,
          razorpay_payment_id: response.razorpay_payment_id,
          razorpay_signature: response.razorpay_signature,
        }}),
      }});
      window.location.href = '{base_url}/admin/email';
    }},
    theme: {{ color: '#E86A2C' }},
  }};
  new Razorpay(options).open();
}});
</script>"##,
        display_amount = display_amount,
        amount = amount,
        currency = currency,
        order_id = html_escape(order_id),
        key = html_escape(razorpay_key),
        tenant_id = html_escape(tenant_id),
        base_url = base_url,
    );

    base_html("Extra email address: Concierge", &content, locale)
}
