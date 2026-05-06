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
        .strip_prefix("/dashboard/billing")
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
            let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
            let kv = env.kv("KV")?;
            let addrs = storage::get_email_addresses(&kv, tenant_id).await?;

            let cfg = storage::get_pricing(&db).await;
            let code = locale.currency.as_str();
            let milli_price = cfg.unit_price_milli(code);
            let address_price = cfg.address_price(code);

            Response::from_html(tmpl::billing_overview_with_addresses_html(
                &bill,
                &locale,
                base_url,
                addrs.len() as u32,
                tenant.email_address_quota(),
                milli_price,
                address_price,
                cfg.email_pack_size,
                cfg.min_credits,
                cfg.max_credits,
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
            let cfg = storage::get_pricing(&db).await;
            let credits = credits_raw
                .parse::<i64>()
                .unwrap_or(cfg.min_credits)
                .clamp(cfg.min_credits, cfg.max_credits);

            // Accept a return_to path (used by the wizard to send users back
            // to /dashboard/wizard/launch after payment). Restrict to same-origin
            // paths to avoid open redirects.
            let return_to = form
                .get("return_to")
                .and_then(|v| v.as_str())
                .filter(|p| p.starts_with('/') && !p.starts_with("//"))
                .unwrap_or("/dashboard/billing")
                .to_string();

            let tenant = storage::get_tenant(&db, tenant_id)
                .await?
                .unwrap_or_default();
            let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
            let currency = locale.currency.as_str();

            let milli_price = cfg.unit_price_milli(locale.currency.as_str());
            let amount = billing::calculate_total(credits, milli_price);

            let key_id = env.secret("RAZORPAY_KEY_ID")?.to_string();
            let key_secret = env.secret("RAZORPAY_KEY_SECRET")?.to_string();

            let receipt = generate_id();
            let order =
                razorpay::create_order(&key_id, &key_secret, amount, currency, &receipt).await?;

            let order_id = order.get("id").and_then(|v| v.as_str()).unwrap_or("");

            Response::from_html(tmpl::checkout_html(
                order_id, amount, &locale, credits, &key_id, tenant_id, &return_to, base_url,
            ))
        }

        // Sign-up verification charge. Creates a Razorpay order with
        // notes.kind="verification" so the webhook records the capture,
        // marks the tenant verified, and auto-refunds. Used by the wizard
        // launch step when the tenant hasn't already paid for anything.
        (Method::Post, "verification") => {
            let form: serde_json::Value = req.json().await.unwrap_or(serde_json::json!({}));
            let return_to = form
                .get("return_to")
                .and_then(|v| v.as_str())
                .filter(|p| p.starts_with('/') && !p.starts_with("//"))
                .unwrap_or("/dashboard/wizard/launch")
                .to_string();

            let tenant = storage::get_tenant(&db, tenant_id)
                .await?
                .unwrap_or_default();
            let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
            let currency = locale.currency.as_str();

            let cfg = storage::get_pricing(&db).await;
            let amount = cfg.verification_amount(locale.currency.as_str());

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
            let order_id = order.get("id").and_then(|v| v.as_str()).unwrap_or("");

            Response::from_html(tmpl::verification_checkout_html(
                order_id, amount, &locale, &key_id, tenant_id, &return_to, base_url,
            ))
        }

        // Buy a reply-email subscription pack. Price + pack size come from
        // pricing_config (defaults ₹99 / $1 per pack/month, 5 addresses).
        // The order carries notes.kind="address" so the Razorpay webhook
        // bumps the tenant's email_address_extras_purchased by the pack size.
        (Method::Post, "address") => {
            let tenant = storage::get_tenant(&db, tenant_id)
                .await?
                .unwrap_or_default();
            let locale = crate::locale::Locale::from_tenant(&tenant.locale, Some(tenant.currency));
            let currency = locale.currency.as_str();

            let cfg = storage::get_pricing(&db).await;
            let amount = cfg.address_price(locale.currency.as_str());

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
                    "kind": "address",
                    // Omit "extras": the webhook falls back to the
                    // configured email_pack_size (default 5) so adjusting
                    // the pack size from /manage takes effect on the
                    // next purchase without a code change here.
                }),
            )
            .await?;
            let order_id = order.get("id").and_then(|v| v.as_str()).unwrap_or("");
            Response::from_html(tmpl::address_checkout_html(
                order_id, amount, &locale, &key_id, tenant_id, base_url,
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

            // A signed payment proves the tenant has a working card, which
            // is what the wizard's verify gate is checking. Flip the flag
            // here synchronously so a user returning to the launch page
            // doesn't have to race the webhook. The webhook also flips it
            // (idempotent), and it stays canonical for granting credits +
            // auto-refunding the verification charge.
            db.prepare(
                "UPDATE tenants \
                    SET verified_at = datetime('now'), updated_at = datetime('now') \
                    WHERE id = ? AND verified_at IS NULL",
            )
            .bind(&[tenant_id.into()])?
            .run()
            .await?;

            // Redirect to billing page. Webhook will handle crediting.
            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/dashboard/billing"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        _ => Response::error("Not Found", 404),
    }
}
