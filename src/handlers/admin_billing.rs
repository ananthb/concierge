//! Tenant-facing billing — view balance, buy credits via Razorpay.

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
    _method: Method,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    let sub = path
        .strip_prefix("/admin/billing")
        .unwrap_or("")
        .trim_start_matches('/');

    let method = req.method();

    match (method, sub) {
        // Billing overview
        (Method::Get, "" | "/") => {
            let mut bill = storage::get_tenant_billing(&kv, tenant_id).await?;
            crate::billing::refresh_billing(&mut bill);
            storage::save_tenant_billing(&kv, tenant_id, &bill).await?;
            let country = req.headers().get("cf-ipcountry")?.unwrap_or_default();
            let currency = if country == "IN" { "INR" } else { "USD" };
            let packs = storage::get_active_credit_packs(&db).await?;

            Response::from_html(tmpl::billing_overview_html(
                &bill, &packs, currency, base_url,
            ))
        }

        // Create Razorpay order
        (Method::Post, "checkout") => {
            let form: serde_json::Value = req.json().await?;
            let credits = form
                .get("credits")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(500);

            let country = req.headers().get("cf-ipcountry")?.unwrap_or_default();
            let currency = if country == "IN" { "INR" } else { "USD" };

            let packs = storage::get_active_credit_packs(&db).await?;
            let pack = match packs.iter().find(|p| p.replies == credits) {
                Some(p) => p,
                None => return Response::error("Invalid pack", 400),
            };
            let amount = pack.price(currency);

            let key_id = env.secret("RAZORPAY_KEY_ID")?.to_string();
            let key_secret = env.secret("RAZORPAY_KEY_SECRET")?.to_string();

            let receipt = generate_id();
            let order =
                razorpay::create_order(&key_id, &key_secret, amount, currency, &receipt).await?;

            let order_id = order.get("id").and_then(|v| v.as_str()).unwrap_or("");

            Response::from_html(tmpl::checkout_html(
                order_id, amount, currency, credits, &key_id, tenant_id, base_url,
            ))
        }

        // Payment verification — only validates signature, does NOT grant credits.
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
