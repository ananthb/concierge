//! Razorpay API client for payment processing.

use worker::*;

const RAZORPAY_API: &str = "https://api.razorpay.com/v1";

/// Create a Razorpay order for a credit top-up.
pub async fn create_order(
    key_id: &str,
    key_secret: &str,
    amount: i64,    // in paise (INR) or cents (USD)
    currency: &str, // "INR" or "USD"
    receipt: &str,  // our internal order ID
) -> Result<serde_json::Value> {
    let payload = serde_json::json!({
        "amount": amount,
        "currency": currency,
        "receipt": receipt,
    });

    razorpay_post(key_id, key_secret, "/orders", &payload).await
}

/// Create a Razorpay customer (for repeat purchases).
pub async fn create_customer(
    key_id: &str,
    key_secret: &str,
    email: &str,
    name: Option<&str>,
) -> Result<serde_json::Value> {
    let mut payload = serde_json::json!({
        "email": email,
    });
    if let Some(n) = name {
        payload["name"] = serde_json::Value::String(n.to_string());
    }

    razorpay_post(key_id, key_secret, "/customers", &payload).await
}

/// Verify a Razorpay payment signature (constant-time).
pub fn verify_payment_signature(
    order_id: &str,
    payment_id: &str,
    signature: &str,
    key_secret: &str,
) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use subtle::ConstantTimeEq;

    type HmacSha256 = Hmac<Sha256>;

    let message = format!("{order_id}|{payment_id}");
    let mut mac =
        HmacSha256::new_from_slice(key_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(message.as_bytes());

    let expected = hex_encode(&mac.finalize().into_bytes());
    expected.as_bytes().ct_eq(signature.as_bytes()).into()
}

/// Verify a Razorpay webhook signature (constant-time).
pub fn verify_webhook_signature(body: &str, signature: &str, webhook_secret: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use subtle::ConstantTimeEq;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(webhook_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body.as_bytes());

    let expected = hex_encode(&mac.finalize().into_bytes());
    expected.as_bytes().ct_eq(signature.as_bytes()).into()
}

/// Create a Razorpay plan (idempotent — call on every deploy or first use).
pub async fn create_plan(
    key_id: &str,
    key_secret: &str,
    amount: i64,
    currency: &str,
    period: &str,
    interval: i32,
    name: &str,
) -> Result<serde_json::Value> {
    let payload = serde_json::json!({
        "period": period,
        "interval": interval,
        "item": {
            "name": name,
            "amount": amount,
            "currency": currency,
        },
    });

    razorpay_post(key_id, key_secret, "/plans", &payload).await
}

/// Create a Razorpay subscription for an email subdomain.
pub async fn create_subscription(
    key_id: &str,
    key_secret: &str,
    plan_id: &str,
    tenant_id: &str,
    subdomain: &str,
) -> Result<serde_json::Value> {
    let payload = serde_json::json!({
        "plan_id": plan_id,
        "total_count": 0,
        "customer_notify": 1,
        "notes": {
            "tenant_id": tenant_id,
            "subdomain": subdomain,
            "type": "email_subdomain",
        },
    });

    razorpay_post(key_id, key_secret, "/subscriptions", &payload).await
}

/// Cancel a Razorpay subscription (at end of current billing period).
pub async fn cancel_subscription(
    key_id: &str,
    key_secret: &str,
    subscription_id: &str,
) -> Result<serde_json::Value> {
    let payload = serde_json::json!({
        "cancel_at_cycle_end": 1,
    });

    let url = format!("{RAZORPAY_API}/subscriptions/{subscription_id}/cancel");
    let body =
        serde_json::to_string(&payload).map_err(|e| Error::from(format!("JSON error: {e}")))?;

    let auth = base64_encode(&format!("{key_id}:{key_secret}"));

    let headers = Headers::new();
    headers.set("Authorization", &format!("Basic {auth}"))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&body)));

    let request = Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(request).send().await?;

    if resp.status_code() >= 400 {
        let err = resp.text().await.unwrap_or_default();
        return Err(Error::from(format!(
            "Razorpay cancel subscription error {}: {}",
            resp.status_code(),
            err
        )));
    }

    resp.json().await
}

/// Fetch a subscription's current status from Razorpay.
pub async fn get_subscription_status(
    key_id: &str,
    key_secret: &str,
    subscription_id: &str,
) -> Result<String> {
    let resp = razorpay_get(
        key_id,
        key_secret,
        &format!("/subscriptions/{subscription_id}"),
    )
    .await?;
    Ok(resp
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string())
}

// --- Internal helpers ---

async fn razorpay_get(key_id: &str, key_secret: &str, path: &str) -> Result<serde_json::Value> {
    let url = format!("{RAZORPAY_API}{path}");
    let auth = base64_encode(&format!("{key_id}:{key_secret}"));

    let headers = Headers::new();
    headers.set("Authorization", &format!("Basic {auth}"))?;

    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(headers);

    let request = Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(request).send().await?;

    if resp.status_code() >= 400 {
        let err = resp.text().await.unwrap_or_default();
        return Err(Error::from(format!(
            "Razorpay API error {}: {}",
            resp.status_code(),
            err
        )));
    }

    resp.json().await
}

async fn razorpay_post(
    key_id: &str,
    key_secret: &str,
    path: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value> {
    let url = format!("{RAZORPAY_API}{path}");
    let body =
        serde_json::to_string(payload).map_err(|e| Error::from(format!("JSON error: {e}")))?;

    let auth = base64_encode(&format!("{key_id}:{key_secret}"));

    let headers = Headers::new();
    headers.set("Authorization", &format!("Basic {auth}"))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&body)));

    let request = Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(request).send().await?;

    if resp.status_code() >= 400 {
        let err = resp.text().await.unwrap_or_default();
        return Err(Error::from(format!(
            "Razorpay API error {}: {}",
            resp.status_code(),
            err
        )));
    }

    resp.json().await
}

fn base64_encode(input: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(input.as_bytes())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
