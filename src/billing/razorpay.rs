//! Razorpay API client for payment processing.

use worker::*;

const RAZORPAY_API: &str = "https://api.razorpay.com/v1";

/// Create a Razorpay order with `notes`. The webhook uses these notes to
/// credit the right tenant on `payment.captured`.
pub async fn create_order_with_notes(
    key_id: &str,
    key_secret: &str,
    amount: i64,
    currency: &str,
    receipt: &str,
    notes: serde_json::Value,
) -> Result<serde_json::Value> {
    let payload = serde_json::json!({
        "amount": amount,
        "currency": currency,
        "receipt": receipt,
        "notes": notes,
    });

    razorpay_post(key_id, key_secret, "/orders", &payload).await
}

/// Verify a Razorpay webhook signature (constant-time).
pub fn verify_webhook_signature(body: &str, signature: &str, webhook_secret: &str) -> bool {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;
    use subtle::ConstantTimeEq;

    if webhook_secret.is_empty() || signature.is_empty() {
        return false;
    }

    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(webhook_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body.as_bytes());

    let expected = hex_encode(&mac.finalize().into_bytes());
    expected.as_bytes().ct_eq(signature.as_bytes()).into()
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

#[cfg(test)]
mod tests {
    use super::*;

    const WEBHOOK_SIG: &str = "4f463a57dd128675850163391f0311888616d57bccca75c774c9cdb28134f851";

    #[test]
    fn webhook_signature_verifies_and_rejects() {
        let body = r#"{"event":"payment.captured"}"#;
        assert!(verify_webhook_signature(body, WEBHOOK_SIG, "whsec_test"));
        assert!(!verify_webhook_signature(body, WEBHOOK_SIG, "whsec_other"));
        assert!(!verify_webhook_signature(
            r#"{"event":"payment.failed"}"#,
            WEBHOOK_SIG,
            "whsec_test"
        ));
        assert!(!verify_webhook_signature(body, "", "whsec_test"));
        assert!(!verify_webhook_signature(body, WEBHOOK_SIG, ""));
    }
}
