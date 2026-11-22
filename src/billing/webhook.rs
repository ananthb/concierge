//! Razorpay webhook handler.
//!
//! Fails closed if `RAZORPAY_WEBHOOK_SECRET` is unset or empty, verifies the
//! HMAC-SHA256 signature, deduplicates by `razorpay_payment_id`, records the
//! actual monetary amount and credits in D1 `payments`, and grants purchased
//! credits to the tenant.

use wasm_bindgen::JsValue;
use worker::*;

use super::razorpay;
use crate::helpers::generate_id;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CapturedPayment {
    pub payment_id: String,
    pub order_id: Option<String>,
    pub tenant_id: String,
    pub amount: i64,
    pub credits: i64,
    pub currency: String,
}

/// Parse a Razorpay `payment.captured` webhook payload into a validated struct.
/// Returns `None` if `tenant_id`, `payment_id`, or positive `credits` are missing.
pub(crate) fn parse_captured_payment(payload: &serde_json::Value) -> Option<CapturedPayment> {
    let payment = payload.pointer("/payload/payment/entity")?;
    let payment_id = payment
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    let order_id = payment
        .get("order_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    let tenant_id = payment
        .pointer("/notes/tenant_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    let credits = payment
        .pointer("/notes/credits")
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.trim().parse::<i64>().ok()))
        })
        .unwrap_or(0);
    if credits <= 0 {
        return None;
    }
    let amount = payment.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
    let currency = payment
        .get("currency")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(super::DEFAULT_CURRENCY)
        .to_string();

    Some(CapturedPayment {
        payment_id,
        order_id,
        tenant_id,
        amount,
        credits,
        currency,
    })
}

/// Handle `POST /webhook/razorpay`
pub async fn handle_razorpay_webhook(mut req: Request, env: Env) -> Result<Response> {
    let webhook_secret = env
        .secret("RAZORPAY_WEBHOOK_SECRET")
        .map(|s| s.to_string())
        .unwrap_or_default();
    if webhook_secret.trim().is_empty() {
        console_log!("RAZORPAY_WEBHOOK_SECRET is not configured");
        return Response::error("Webhook secret not configured", 500);
    }

    let signature = req
        .headers()
        .get("X-Razorpay-Signature")?
        .unwrap_or_default();
    let body = req.text().await?;

    if !razorpay::verify_webhook_signature(&body, &signature, &webhook_secret) {
        console_log!("Invalid Razorpay webhook signature");
        return Response::error("Invalid signature", 401);
    }

    let payload: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| Error::from(format!("JSON parse error: {e}")))?;

    let event = payload.get("event").and_then(|v| v.as_str()).unwrap_or("");

    match event {
        "payment.captured" => {
            let Some(captured) = parse_captured_payment(&payload) else {
                console_log!("Webhook missing tenant_id, payment_id, or positive credits");
                return Response::ok("OK");
            };

            let db = env.d1("DB")?;

            if is_payment_processed(&db, &captured.payment_id).await? {
                console_log!(
                    "Payment {} already processed, skipping",
                    captured.payment_id
                );
                return Response::ok("OK");
            }

            record_payment(&db, &captured).await?;
            super::grant_purchased(&db, &captured.tenant_id, captured.credits).await?;
            console_log!(
                "Granted {} replies to {} (payment {})",
                captured.credits,
                captured.tenant_id,
                captured.payment_id
            );

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

async fn record_payment(db: &D1Database, captured: &CapturedPayment) -> Result<()> {
    let id = generate_id();
    let order_val: JsValue = match &captured.order_id {
        Some(oid) => oid.as_str().into(),
        None => JsValue::NULL,
    };
    let stmt = db.prepare(
        "INSERT INTO payments (id, tenant_id, razorpay_payment_id, razorpay_order_id, amount, credits, currency, status)
         VALUES (?, ?, ?, ?, ?, ?, ?, 'captured')",
    );
    stmt.bind(&[
        id.as_str().into(),
        captured.tenant_id.as_str().into(),
        captured.payment_id.as_str().into(),
        order_val,
        JsValue::from(captured.amount as f64),
        JsValue::from(captured.credits as f64),
        captured.currency.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_valid_captured_payment() {
        let payload = json!({
            "event": "payment.captured",
            "payload": {
                "payment": {
                    "entity": {
                        "id": "pay_123",
                        "order_id": "order_456",
                        "amount": 5000,
                        "currency": "INR",
                        "notes": {
                            "tenant_id": "tenant-1",
                            "kind": "reply_credits",
                            "credits": "500"
                        }
                    }
                }
            }
        });
        assert_eq!(
            parse_captured_payment(&payload),
            Some(CapturedPayment {
                payment_id: "pay_123".to_string(),
                order_id: Some("order_456".to_string()),
                tenant_id: "tenant-1".to_string(),
                amount: 5000,
                credits: 500,
                currency: "INR".to_string(),
            })
        );
    }

    #[test]
    fn rejects_missing_notes_or_zero_credits() {
        let missing_tenant = json!({
            "payload": { "payment": { "entity": {
                "id": "pay_1",
                "amount": 1000,
                "notes": { "credits": "100" }
            }}}
        });
        assert_eq!(parse_captured_payment(&missing_tenant), None);

        let zero_credits = json!({
            "payload": { "payment": { "entity": {
                "id": "pay_1",
                "amount": 1000,
                "notes": { "tenant_id": "t1", "credits": "0" }
            }}}
        });
        assert_eq!(parse_captured_payment(&zero_credits), None);
    }
}
