//! `/api/billing/*` — prepaid reply credit balance and Razorpay checkout.
//!
//! Strict prepaid model: 1 credit = 1 reply.
//! Credits are granted exclusively by the Razorpay `payment.captured` webhook.

use serde::Serialize;
use worker::*;

use crate::billing;
use crate::billing::razorpay;
use crate::helpers::generate_id;
use crate::storage;
use crate::types::TenantBilling;

#[derive(Serialize)]
struct BillingView {
    balance: i64,
    replies_used: i64,
    metered: bool,
    currency: String,
    pricing: BillingPricingView,
    ledger: Vec<CreditView>,
}

#[derive(Serialize)]
struct BillingPricingView {
    unit_price_milli: i64,
    min_credits: i64,
    max_credits: i64,
    currency: String,
}

#[derive(Serialize)]
struct CreditView {
    id: String,
    amount: i64,
    total_minor: i64,
    currency: String,
    source: String,
    expires_at: Option<String>,
    granted_at: String,
}

#[derive(Serialize)]
struct OrderView {
    order_id: String,
    amount: i64,
    currency: String,
    key_id: String,
    credits: i64,
}

pub async fn handle(mut req: Request, env: Env, rest: &[&str]) -> Result<Response> {
    let tenant_id = match super::require_tenant(&req, &env).await {
        Ok(t) => t,
        Err(r) => return r,
    };
    let db = env.d1("DB")?;
    let method = req.method();

    match (method, rest) {
        (Method::Get, []) => {
            let mut bill = storage::get_tenant_billing(&db, &tenant_id).await?;
            billing::refresh_billing(&mut bill);
            storage::save_tenant_billing(&db, &tenant_id, &bill).await?;

            let tenant = storage::get_tenant(&db, &tenant_id)
                .await?
                .unwrap_or_default();
            let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
            let code = locale.currency.as_str();
            let ledger = load_invoices(&db, &tenant_id, &bill, code).await?;

            super::json(&BillingView {
                balance: balance_of(&bill),
                replies_used: bill.replies_used,
                metered: tenant.plan.is_metered(),
                currency: code.to_string(),
                pricing: BillingPricingView {
                    unit_price_milli: billing::unit_price_milli(code),
                    min_credits: billing::MIN_CREDITS,
                    max_credits: billing::MAX_CREDITS,
                    currency: code.to_string(),
                },
                ledger,
            })
        }

        (Method::Get, ["invoices", invoice_id]) => {
            let bill = storage::get_tenant_billing(&db, &tenant_id).await?;
            let tenant = storage::get_tenant(&db, &tenant_id)
                .await?
                .unwrap_or_default();
            let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
            let code = locale.currency.as_str();
            let ledger = load_invoices(&db, &tenant_id, &bill, code).await?;
            let Some(inv) = ledger.into_iter().find(|e| e.id == *invoice_id) else {
                return super::not_found("Invoice");
            };
            let onboarding = storage::get_onboarding(&db, &tenant_id)
                .await
                .unwrap_or_default();
            let html = render_invoice_html(&inv, &onboarding.business, &tenant.email);
            let headers = Headers::new();
            headers.set("Content-Type", "text/html; charset=utf-8")?;
            headers.set(
                "Content-Disposition",
                &format!("attachment; filename=\"invoice-{}.html\"", inv.id),
            )?;
            headers.set("Cache-Control", "no-store")?;
            Ok(Response::ok(html)?.with_headers(headers))
        }

        (Method::Post, ["checkout"]) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let tenant = storage::get_tenant(&db, &tenant_id)
                .await?
                .unwrap_or_default();
            if !tenant.plan.is_metered() {
                return super::error(
                    "not_metered",
                    "Complimentary accounts don't buy credits.",
                    400,
                );
            }

            let credits = super::field_i64(&body, "credits")
                .unwrap_or(billing::MIN_CREDITS)
                .clamp(billing::MIN_CREDITS, billing::MAX_CREDITS);

            let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
            let currency = locale.currency.as_str();
            let amount = billing::calculate_total(credits, billing::unit_price_milli(currency));
            if amount <= 0 {
                return super::error(
                    "unpriced_currency",
                    "Credits aren't priced in your currency yet.",
                    503,
                );
            }

            let key_id = env
                .var("RAZORPAY_KEY_ID")
                .map(|v| v.to_string())
                .unwrap_or_default();
            let key_secret = env
                .secret("RAZORPAY_KEY_SECRET")
                .map(|v| v.to_string())
                .unwrap_or_default();
            if key_id.is_empty() || key_secret.is_empty() {
                return super::error(
                    "billing_unconfigured",
                    "Razorpay billing is not configured on this deploy.",
                    503,
                );
            }
            let receipt = generate_id();
            let order = razorpay::create_order_with_notes(
                &key_id,
                &key_secret,
                amount,
                currency,
                &receipt,
                serde_json::json!({
                    "tenant_id": tenant_id,
                    "kind": "reply_credits",
                    "credits": credits.to_string(),
                }),
            )
            .await?;

            super::json(&OrderView {
                order_id: order
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                amount,
                currency: currency.to_string(),
                key_id,
                credits,
            })
        }

        _ => super::not_found("Endpoint"),
    }
}

fn balance_of(bill: &TenantBilling) -> i64 {
    bill.credits.iter().map(|e| e.amount).sum()
}

async fn load_invoices(
    db: &D1Database,
    tenant_id: &str,
    bill: &TenantBilling,
    default_currency: &str,
) -> Result<Vec<CreditView>> {
    let rows = db
        .prepare(
            "SELECT id, amount, credits, currency, created_at
             FROM payments
             WHERE tenant_id = ? AND status = 'captured'
             ORDER BY created_at DESC",
        )
        .bind(&[tenant_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    if !rows.is_empty() {
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let total_minor = row.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
            let credits = row.get("credits").and_then(|v| v.as_i64()).unwrap_or(0);
            let currency = row
                .get("currency")
                .and_then(|v| v.as_str())
                .unwrap_or(default_currency)
                .to_string();
            let granted_at = row
                .get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            out.push(CreditView {
                id,
                amount: credits,
                total_minor,
                currency,
                source: "purchase".to_string(),
                expires_at: None,
                granted_at,
            });
        }
        return Ok(out);
    }

    let unit_milli = billing::unit_price_milli(default_currency);
    Ok(bill
        .credits
        .iter()
        .enumerate()
        .map(|(idx, e)| CreditView {
            id: format!("credit-{idx}"),
            amount: e.amount,
            total_minor: billing::calculate_total(e.amount, unit_milli),
            currency: default_currency.to_string(),
            source: format!("{:?}", e.source).to_lowercase(),
            expires_at: e.expires_at.clone(),
            granted_at: e.granted_at.clone(),
        })
        .collect())
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_invoice_html(
    inv: &CreditView,
    business: &crate::types::BusinessInfo,
    email: &str,
) -> String {
    let date_only: String = inv.granted_at.chars().take(10).collect();
    let major = inv.total_minor / 100;
    let minor = (inv.total_minor % 100).abs();
    let formatted_total = match inv.currency.as_str() {
        "INR" => format!("₹{major}.{minor:02}"),
        "USD" => format!("${major}.{minor:02}"),
        other => format!("{other} {major}.{minor:02}"),
    };
    let buyer_name = if !business.name.trim().is_empty() {
        escape_html(business.name.trim())
    } else if !email.trim().is_empty() {
        escape_html(email.trim())
    } else {
        "Customer".to_string()
    };
    let mut details = Vec::new();
    if !business.address.trim().is_empty() {
        details.push(escape_html(business.address.trim()));
    }
    let state_pin = format!("{} {}", business.state.trim(), business.pincode.trim())
        .trim()
        .to_string();
    if !state_pin.is_empty() {
        details.push(escape_html(&state_pin));
    }
    if !business.pan.trim().is_empty() {
        details.push(format!("PAN: {}", escape_html(business.pan.trim())));
    }
    if !business.gstin.trim().is_empty() {
        details.push(format!("GSTIN: {}", escape_html(business.gstin.trim())));
    }
    let buyer_meta = if details.is_empty() {
        String::new()
    } else {
        format!("<p>{}</p>", details.join("<br>"))
    };
    let inv_id = escape_html(&inv.id);
    let date_esc = escape_html(&date_only);

    format!(
        "<!doctype html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <title>Invoice {inv_id} — replybots</title>\n\
         <style>\n\
         body {{ font-family: system-ui, -apple-system, sans-serif; color: #111; max-width: 680px; margin: 40px auto; padding: 24px; border: 2px solid #111; }}\n\
         h1 {{ margin: 0 0 4px; font-size: 24px; }}\n\
         .meta {{ color: #555; font-size: 14px; margin-bottom: 24px; }}\n\
         table {{ width: 100%; border-collapse: collapse; margin-top: 24px; }}\n\
         th, td {{ text-align: left; padding: 10px 8px; border-bottom: 1px solid #ddd; }}\n\
         th {{ border-bottom: 2px solid #111; font-size: 13px; text-transform: uppercase; }}\n\
         .total td {{ font-weight: 700; border-top: 2px solid #111; border-bottom: 0; }}\n\
         </style>\n\
         </head>\n\
         <body>\n\
         <h1>replybots — Receipt / Invoice</h1>\n\
         <div class=\"meta\">Invoice ID: {inv_id} · Date: {date_esc}</div>\n\
         <div><strong>Billed to:</strong><p>{buyer_name}</p>{buyer_meta}</div>\n\
         <table>\n\
         <thead><tr><th>Description</th><th>Credits</th><th>Amount</th></tr></thead>\n\
         <tbody>\n\
         <tr><td>Prepaid WhatsApp auto-reply credits</td><td>{credits}</td><td>{formatted_total}</td></tr>\n\
         <tr class=\"total\"><td colspan=\"2\">Total paid</td><td>{formatted_total}</td></tr>\n\
         </tbody>\n\
         </table>\n\
         </body>\n\
         </html>\n",
        inv_id = inv_id,
        date_esc = date_esc,
        buyer_name = buyer_name,
        buyer_meta = buyer_meta,
        credits = inv.amount,
        formatted_total = formatted_total,
    )
}
