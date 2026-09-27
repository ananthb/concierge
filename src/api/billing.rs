//! `/api/billing/*` — credit balance and Razorpay checkout.
//!
//! Flat prepaid credits: one credit per AI reply, bought in any quantity,
//! no tiers and no packs. Canned replies are free and never touch this.
//!
//! The HTML version of these endpoints returned a rendered page that loaded
//! Razorpay's script and called `new Razorpay(...)` inline. Here they return
//! the order parameters and the frontend opens the checkout itself, which is
//! why `key_id` is in the payload — it's the publishable key, and Razorpay's
//! browser SDK requires it client-side. `key_secret` never leaves the worker.
//!
//! **Credits are granted only by the Razorpay webhook**
//! ([`crate::billing::webhook`]). `POST /api/billing/verify` proves the
//! payment signature is genuine and flips the tenant's verification flag; it
//! deliberately grants nothing, so a forged client call can't mint credit.

use serde::Serialize;
use worker::*;

use crate::billing;
use crate::billing::razorpay;
use crate::helpers::generate_id;
use crate::storage;
use crate::types::{CreditEntry, TenantBilling};

#[derive(Serialize)]
struct BillingView {
    /// Usable credits, expired entries already pruned.
    balance: i64,
    replies_used: i64,
    /// Ledger entries, soonest-expiring first — the order they're spent in.
    credits: Vec<CreditView>,
    unit_price_milli: i64,
    currency: String,
    min_credits: i64,
    max_credits: i64,
    /// Complimentary accounts can't buy credits and aren't charged.
    metered: bool,
    verified: bool,
}

#[derive(Serialize)]
struct CreditView {
    amount: i64,
    source: String,
    expires_at: Option<String>,
    granted_at: String,
}

/// Everything the browser needs to open Razorpay's checkout.
#[derive(Serialize)]
struct OrderView {
    order_id: String,
    /// Charge in minor units (paise / cents).
    amount: i64,
    currency: String,
    /// Razorpay publishable key. Safe to expose; the secret is not.
    key_id: String,
    /// Credits being bought. Zero for the verification charge.
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
            // Reading the balance prunes expired entries and reorders the
            // ledger, so persist the pruned form rather than recomputing it
            // on every read.
            let mut bill = storage::get_tenant_billing(&db, &tenant_id).await?;
            billing::refresh_billing(&mut bill);
            storage::save_tenant_billing(&db, &tenant_id, &bill).await?;

            let tenant = storage::get_tenant(&db, &tenant_id)
                .await?
                .unwrap_or_default();
            let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
            let cfg = storage::get_pricing(&db).await;
            let code = locale.currency.as_str();

            super::json(&BillingView {
                balance: balance_of(&bill),
                replies_used: bill.replies_used,
                credits: bill.credits.iter().map(credit_view).collect(),
                unit_price_milli: cfg.unit_price_milli(code),
                currency: code.to_string(),
                min_credits: cfg.min_credits,
                max_credits: cfg.max_credits,
                metered: tenant.plan.is_metered(),
                verified: tenant.verified_at.is_some(),
            })
        }

        // Change the display currency. Affects what future orders are
        // quoted in, so it's a tenant record write, not a UI preference.
        (Method::Put, ["currency"]) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let currency = crate::locale::Currency::parse(&super::field(&body, "currency"));
            if let Some(mut tenant) = storage::get_tenant(&db, &tenant_id).await? {
                if tenant.currency != currency {
                    tenant.currency = currency;
                    tenant.updated_at = crate::helpers::now_iso();
                    storage::save_tenant(&db, &tenant).await?;
                }
            }
            super::no_content()
        }

        // Buy credits.
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

            let cfg = storage::get_pricing(&db).await;
            let credits = super::field_i64(&body, "credits")
                .unwrap_or(cfg.min_credits)
                .clamp(cfg.min_credits, cfg.max_credits);

            let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
            let currency = locale.currency.as_str();
            let amount = billing::calculate_total(credits, cfg.unit_price_milli(currency));
            if amount <= 0 {
                return super::error(
                    "unpriced_currency",
                    "Credits aren't priced in your currency yet.",
                    503,
                );
            }

            let key_id = env.secret("RAZORPAY_KEY_ID")?.to_string();
            let key_secret = env.secret("RAZORPAY_KEY_SECRET")?.to_string();
            let receipt = generate_id();
            let order =
                razorpay::create_order(&key_id, &key_secret, amount, currency, &receipt).await?;

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

        // Sign-up verification charge: a small refundable payment that
        // proves a working card before the wizard will finish. The webhook
        // records the capture, flips `verified_at`, and auto-refunds it.
        (Method::Post, ["verification"]) => {
            let tenant = storage::get_tenant(&db, &tenant_id)
                .await?
                .unwrap_or_default();
            if !tenant.plan.is_metered() {
                return super::error(
                    "not_metered",
                    "Complimentary accounts skip verification.",
                    400,
                );
            }

            let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
            let currency = locale.currency.as_str();
            let cfg = storage::get_pricing(&db).await;
            let amount = cfg.verification_amount(currency);

            let key_id = env.secret("RAZORPAY_KEY_ID")?.to_string();
            let key_secret = env.secret("RAZORPAY_KEY_SECRET")?.to_string();
            let receipt = generate_id();
            let order = razorpay::create_order_with_notes(
                &key_id,
                &key_secret,
                amount,
                currency,
                &receipt,
                serde_json::json!({
                    "tenant_id": tenant_id,
                    "kind": "verification",
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
                credits: 0,
            })
        }

        // Confirm a completed checkout. Validates the signature only.
        (Method::Post, ["verify"]) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let order_id = super::field(&body, "razorpay_order_id");
            let payment_id = super::field(&body, "razorpay_payment_id");
            let signature = super::field(&body, "razorpay_signature");
            let key_secret = env.secret("RAZORPAY_KEY_SECRET")?.to_string();

            if !razorpay::verify_payment_signature(&order_id, &payment_id, &signature, &key_secret)
            {
                return super::error("signature_invalid", "We couldn't verify that payment.", 400);
            }

            // A signed payment proves a working card, which is exactly what
            // the wizard's gate checks. Flip it synchronously so a user
            // returning from checkout doesn't have to race the webhook. The
            // webhook flips it too; the guard makes that idempotent.
            db.prepare(
                "UPDATE tenants \
                    SET verified_at = datetime('now'), updated_at = datetime('now') \
                    WHERE id = ? AND verified_at IS NULL",
            )
            .bind(&[tenant_id.clone().into()])?
            .run()
            .await?;

            super::no_content()
        }

        _ => super::not_found("Endpoint"),
    }
}

/// Usable credits. Assumes [`billing::refresh_billing`] has already pruned
/// expired entries.
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
