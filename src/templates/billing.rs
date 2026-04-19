//! Billing templates — tenant-facing credit balance + purchase

use crate::helpers::html_escape;
use crate::types::{CreditPackRow, CreditSource, TenantBilling};

use super::base::{app_shell, base_html};
use super::HASH;

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

pub fn billing_overview_html(
    billing: &TenantBilling,
    packs: &[CreditPackRow],
    currency: &str,
    base_url: &str,
) -> String {
    let summary = summarize(billing);

    let total_style = if summary.total <= 0 {
        r#"style="color:var(--warn)""#
    } else {
        ""
    };

    let free_detail = match &summary.free_expires {
        Some(exp) => format!(
            r#"<div class="mono muted" style="font-size:11px">expires {}</div>"#,
            format_expiry(exp)
        ),
        None => String::new(),
    };

    let granted_detail = if summary.granted > 0 {
        match &summary.granted_earliest_expiry {
            Some(exp) => format!(
                r#"<div class="mono muted" style="font-size:11px">earliest expiry {}</div>"#,
                format_expiry(exp)
            ),
            None => String::new(),
        }
    } else {
        String::new()
    };

    let pack_buttons: String = packs
        .iter()
        .map(|p| {
            let price = if currency == "INR" {
                format!("₹{}", p.price_inr / 100)
            } else {
                format!("${}", p.price_usd / 100)
            };
            format!(
                r##"<form hx-post="{base_url}/admin/billing/checkout" hx-target="body" hx-swap="innerHTML" style="display:inline">
  <input type="hidden" name="credits" value="{credits}">
  <button class="btn sm" type="submit">{name}: {credits} replies, {price}</button>
</form>"##,
                base_url = base_url,
                credits = p.replies,
                name = html_escape(&p.name),
                price = price,
            )
        })
        .collect();

    let content = format!(
        r##"<div style="padding:24px 28px">
  <p><a href="{base_url}/admin">&larr; Back to Dashboard</a></p>
  <div class="eyebrow">Billing</div>
  <h2 class="display-sm" style="margin:4px 0 16px">Reply credits</h2>

  <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:16px;margin-bottom:24px">
    <div class="card" style="padding:18px;text-align:center">
      <div class="stat-n serif" {total_style}>{total}</div>
      <div class="mono muted" style="font-size:11px">Total remaining</div>
    </div>
    <div class="card" style="padding:18px;text-align:center">
      <div class="stat-n serif">{free}</div>
      <div class="mono muted" style="font-size:11px">Free this month</div>
      {free_detail}
    </div>
    <div class="card" style="padding:18px;text-align:center">
      <div class="stat-n serif">{purchased}</div>
      <div class="mono muted" style="font-size:11px">Purchased</div>
      <div class="mono muted" style="font-size:11px">never expire</div>
    </div>
    <div class="card" style="padding:18px;text-align:center">
      <div class="stat-n serif">{granted}</div>
      <div class="mono muted" style="font-size:11px">Granted</div>
      {granted_detail}
    </div>
    <div class="card" style="padding:18px;text-align:center">
      <div class="stat-n serif">{used}</div>
      <div class="mono muted" style="font-size:11px">Replies sent</div>
    </div>
  </div>

  <div class="card" style="padding:22px">
    <h3 style="margin-bottom:8px">Buy reply credits</h3>
    <p class="muted" style="margin-bottom:16px">Purchase a pack to top up your balance. Purchased credits never expire. First 100 replies each month are free.</p>
    <div class="row gap-12" style="flex-wrap:wrap">
      {pack_buttons}
    </div>
  </div>
</div>"##,
        base_url = base_url,
        total = summary.total,
        total_style = total_style,
        free = summary.free,
        free_detail = free_detail,
        purchased = summary.purchased,
        granted = summary.granted,
        granted_detail = granted_detail,
        used = billing.replies_used,
        pack_buttons = pack_buttons,
    );

    let page = app_shell(&content, "Billing", base_url);
    base_html("Billing - Concierge", &page)
}

pub fn checkout_html(
    order_id: &str,
    amount: i64,
    currency: &str,
    credits: i64,
    razorpay_key: &str,
    tenant_id: &str,
    base_url: &str,
) -> String {
    let display_amount = if currency == "INR" {
        format!("₹{}", amount / 100)
    } else {
        format!("${}", amount / 100)
    };

    // Using r##" to avoid issues with JS single quotes
    let content = format!(
        r##"<div style="max-width:480px;margin:4rem auto;text-align:center;padding:0 1rem">
  <div class="card" style="padding:28px">
    <h2 class="display-sm">Complete purchase</h2>
    <p class="muted" style="margin:8px 0 24px">Buying <strong>{credits}</strong> reply credits</p>
    <div class="stat-n serif" style="margin-bottom:24px">{display_amount}</div>
    <button id="pay-btn" class="btn primary lg" style="width:100%">Pay with Razorpay</button>
    <p class="mono muted" style="font-size:11px;margin-top:12px">Secure payment via Razorpay</p>
  </div>
  <a href="{base_url}/admin/billing" class="btn ghost sm" style="margin-top:16px">&larr; Cancel</a>
</div>
<script src="https://checkout.razorpay.com/v1/checkout.js"></script>
<script>
document.getElementById("pay-btn").addEventListener("click", function() {{
  var options = {{
    key: "{key}",
    amount: {amount},
    currency: "{currency}",
    order_id: "{order_id}",
    name: "Concierge",
    description: "{credits} reply credits",
    notes: {{ tenant_id: "{tenant_id}", credits: "{credits}" }},
    handler: function(response) {{
      fetch("{base_url}/admin/billing/verify", {{
        method: "POST",
        headers: {{"Content-Type": "application/json"}},
        body: JSON.stringify({{
          razorpay_order_id: response.razorpay_order_id,
          razorpay_payment_id: response.razorpay_payment_id,
          razorpay_signature: response.razorpay_signature,
          credits: "{credits}"
        }})
      }}).then(function() {{ window.location.href = "{base_url}/admin/billing"; }});
    }},
    theme: {{ color: "#E86A2C" }}
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
    );

    base_html("Checkout - Concierge", &content)
}
