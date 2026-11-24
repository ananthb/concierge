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
use crate::types::{CreditEntry, TenantBilling};

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
    amount: i64,
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
                ledger: bill.credits.iter().map(credit_view).collect(),
            })
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
                .secret("RAZORPAY_KEY_ID")
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

fn credit_view(e: &CreditEntry) -> CreditView {
    CreditView {
        amount: e.amount,
        source: format!("{:?}", e.source).to_lowercase(),
        expires_at: e.expires_at.clone(),
        granted_at: e.granted_at.clone(),
    }
}
