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

/// Verify a Razorpay payment signature.
/// Returns true if the signature matches.
pub fn verify_payment_signature(
    order_id: &str,
    payment_id: &str,
    signature: &str,
    key_secret: &str,
) -> bool {
    // HMAC-SHA256 of "{order_id}|{payment_id}" with key_secret
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let message = format!("{order_id}|{payment_id}");
    let mut mac =
        HmacSha256::new_from_slice(key_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(message.as_bytes());

    let expected = hex_encode(&mac.finalize().into_bytes());
    expected == signature
}

/// Verify a Razorpay webhook signature.
pub fn verify_webhook_signature(body: &str, signature: &str, webhook_secret: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(webhook_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body.as_bytes());

    let expected = hex_encode(&mac.finalize().into_bytes());
    expected == signature
}

// --- Internal helpers ---

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
