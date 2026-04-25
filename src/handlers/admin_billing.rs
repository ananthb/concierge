//! Tenant-facing billing: view balance, buy credits via Razorpay.

use worker::*;

use crate::billing;
use crate::billing::razorpay;
use crate::helpers::*;
use crate::storage;
use crate::templates::billing as tmpl;

pub async fn handle_billing_admin(
    mut req: Request,
    env: Env,
    path: &str,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let _kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    let sub = path
        .strip_prefix("/admin/billing")
        .unwrap_or("")
        .trim_start_matches('/');

    let method = req.method();

    match (method, sub) {
        // Billing overview
        (Method::Get, "" | "/") => {
            let mut bill = storage::get_tenant_billing(&db, tenant_id).await?;
            crate::billing::refresh_billing(&mut bill);
            storage::save_tenant_billing(&db, tenant_id, &bill).await?;
            let tenant = storage::get_tenant(&db, tenant_id)
                .await?
                .unwrap_or_default();
            let currency = if tenant.currency == "USD" {
                "USD"
            } else {
                "INR"
            };
            let kv = env.kv("KV")?;
            let addrs = storage::get_email_addresses(&kv, tenant_id).await?;
            Response::from_html(tmpl::billing_overview_with_addresses_html(
                &bill,
                currency,
                base_url,
                addrs.len() as u32,
                tenant.email_address_quota(),
            ))
        }

        // Create Razorpay order: flat per-reply rate, any quantity.
        (Method::Post, "checkout") => {
            let form: serde_json::Value = req.json().await?;
            let credits_raw = form
                .get("credits")
                .and_then(|v| {
                    v.as_str()
                        .map(|s| s.to_string())
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                })
                .unwrap_or_default();
            let credits = credits_raw
                .parse::<i64>()
                .unwrap_or(billing::MIN_CREDITS)
                .clamp(billing::MIN_CREDITS, billing::MAX_CREDITS);

            // Accept a return_to path (used by the wizard to send users back
            // to /admin/wizard/launch after payment). Restrict to same-origin
            // paths to avoid open redirects.
            let return_to = form
                .get("return_to")
                .and_then(|v| v.as_str())
                .filter(|p| p.starts_with('/') && !p.starts_with("//"))
                .unwrap_or("/admin/billing")
                .to_string();

            let tenant = storage::get_tenant(&db, tenant_id)
                .await?
                .unwrap_or_default();
            let currency = if tenant.currency == "USD" {
                "USD"
            } else {
                "INR"
            };
            let amount = credits * billing::unit_price(currency);

            let key_id = env.secret("RAZORPAY_KEY_ID")?.to_string();
            let key_secret = env.secret("RAZORPAY_KEY_SECRET")?.to_string();

            let receipt = generate_id();
            let order =
                razorpay::create_order(&key_id, &key_secret, amount, currency, &receipt).await?;

            let order_id = order.get("id").and_then(|v| v.as_str()).unwrap_or("");

            Response::from_html(tmpl::checkout_html(
                order_id, amount, currency, credits, &key_id, tenant_id, &return_to, base_url,
            ))
        }

        // Email address pack (5 addresses, one-time). Builds an order with
        // notes.kind="address_pack" so the webhook knows to bump the
        // tenant's address quota.
        (Method::Post, "address-packs") => {
            let tenant = storage::get_tenant(&db, tenant_id)
                .await?
                .unwrap_or_default();
            let currency = if tenant.currency == "USD" {
                "USD"
            } else {
                "INR"
            };
            let amount = billing::address_pack_price(currency);

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
                    "kind": "address_pack",
                    "packs": "1",
                }),
            )
            .await?;
            let order_id = order.get("id").and_then(|v| v.as_str()).unwrap_or("");
            Response::from_html(tmpl::address_pack_checkout_html(
                order_id, amount, currency, &key_id, tenant_id, base_url,
            ))
        }

        // Payment verification: only validates signature, does NOT grant credits.
        // Credits are granted exclusively by the Razorpay webhook handler.
        (Method::Post, "verify") => {
            let form: serde_json::Value = req.json().await?;
            let order_id = form
                .get("razorpay_order_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let payment_id = form
                .get("razorpay_payment_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let signature = form
                .get("razorpay_signature")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let key_secret = env.secret("RAZORPAY_KEY_SECRET")?.to_string();

            if !razorpay::verify_payment_signature(order_id, payment_id, signature, &key_secret) {
                return Response::from_html(
                    r#"<div class="error">Payment verification failed.</div>"#.to_string(),
                );
            }

            // Redirect to billing page. Webhook will handle crediting.
            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/admin/billing"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        _ => Response::error("Not Found", 404),
    }
}
