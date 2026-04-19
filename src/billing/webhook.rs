//! Razorpay webhook handler — processes payment events.
//! Idempotent: checks payment_id in D1 before granting credits.

use wasm_bindgen::JsValue;
use worker::*;

use super::razorpay;
use crate::helpers::generate_id;

/// Handle POST /webhook/razorpay
pub async fn handle_razorpay_webhook(mut req: Request, env: Env) -> Result<Response> {
    let signature = req
        .headers()
        .get("X-Razorpay-Signature")?
        .unwrap_or_default();
    let body = req.text().await?;

    let webhook_secret = env
        .secret("RAZORPAY_WEBHOOK_SECRET")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if !webhook_secret.is_empty()
        && !razorpay::verify_webhook_signature(&body, &signature, &webhook_secret)
    {
        console_log!("Invalid Razorpay webhook signature");
        return Response::error("Invalid signature", 401);
    }

    let payload: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| Error::from(format!("JSON parse error: {e}")))?;

    let event = payload.get("event").and_then(|v| v.as_str()).unwrap_or("");

    match event {
        "payment.captured" => {
            let payment = match payload.pointer("/payload/payment/entity") {
                Some(p) => p,
                None => return Response::ok("OK"),
            };

            let payment_id = payment.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let tenant_id = payment
                .pointer("/notes/tenant_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let credits = payment
                .pointer("/notes/credits")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);

            if tenant_id.is_empty() || credits <= 0 || payment_id.is_empty() {
                console_log!("Webhook missing tenant_id/credits/payment_id");
                return Response::ok("OK");
            }

            let db = env.d1("DB")?;

            // Idempotency check: has this payment already been processed?
            if is_payment_processed(&db, payment_id).await? {
                console_log!("Payment {payment_id} already processed, skipping");
                return Response::ok("OK");
            }

            // Record the payment first (before granting credits)
            record_payment(&db, payment_id, tenant_id, credits).await?;

            // Grant credits
            let kv = env.kv("CALENDARS_KV")?;
            super::grant_replies(&kv, tenant_id, credits).await?;

            console_log!("Granted {credits} replies to {tenant_id} (payment {payment_id})");
            Response::ok("OK")
        }
        "payment.failed" => {
            console_log!("Razorpay payment failed: {:?}", payload.get("payload"));
            Response::ok("OK")
        }
        _ => {
            console_log!("Unhandled Razorpay event: {event}");
            Response::ok("OK")
        }
    }
}

async fn is_payment_processed(db: &D1Database, payment_id: &str) -> Result<bool> {
    let stmt = db.prepare("SELECT id FROM payments WHERE razorpay_payment_id = ?");
    let result = stmt
        .bind(&[payment_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(result.is_some())
}

async fn record_payment(
    db: &D1Database,
    payment_id: &str,
    tenant_id: &str,
    credits: i64,
) -> Result<()> {
    let id = generate_id();
    let stmt = db.prepare(
        "INSERT INTO payments (id, tenant_id, razorpay_payment_id, amount, currency, status)
         VALUES (?, ?, ?, ?, 'INR', 'captured')",
    );
    stmt.bind(&[
        id.as_str().into(),
        tenant_id.into(),
        payment_id.into(),
        JsValue::from(credits as f64),
    ])?
    .run()
    .await?;
    Ok(())
}
