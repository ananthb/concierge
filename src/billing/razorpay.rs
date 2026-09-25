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

/// Create a Razorpay order with arbitrary `notes`. The webhook uses these
/// to decide what to grant on `payment.captured`.
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

/// Refund a captured Razorpay payment in full. Used by the webhook to
/// auto-refund the sign-up verification charge once we've recorded the
/// capture. Returns the API response so callers can log the refund id.
pub async fn refund_payment(
    key_id: &str,
    key_secret: &str,
    payment_id: &str,
) -> Result<serde_json::Value> {
    // Empty body refunds the full captured amount; Razorpay accepts an
    // optional `amount` to support partial refunds, but we always want full.
    let payload = serde_json::json!({});
    let path = format!("/payments/{payment_id}/refund");
    razorpay_post(key_id, key_secret, &path, &payload).await
}

/// Verify a Razorpay payment signature (constant-time).
pub fn verify_payment_signature(
    order_id: &str,
    payment_id: &str,
    signature: &str,
    key_secret: &str,
) -> bool {
    use hmac::{Hmac, KeyInit, Mac};
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
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;
    use subtle::ConstantTimeEq;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(webhook_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body.as_bytes());

    let expected = hex_encode(&mac.finalize().into_bytes());
    expected.as_bytes().ct_eq(signature.as_bytes()).into()
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

#[cfg(test)]
mod tests {
    use super::*;

    // Expected digests computed independently (Python `hmac`/`hashlib`),
    // not by calling the functions under test, so these pin the wire
    // format Razorpay signs -- `{order_id}|{payment_id}` for payments,
    // the raw body for webhooks -- as well as the HMAC itself.
    const PAYMENT_SIG: &str = "7ecf420b62aca4b2ca03b4cdb2df2ade2600feeb9faca41d3258f07be04b9f5b";
    const WEBHOOK_SIG: &str = "4f463a57dd128675850163391f0311888616d57bccca75c774c9cdb28134f851";

    #[test]
    fn payment_signature_verifies() {
        assert!(verify_payment_signature(
            "order_ABC123",
            "pay_XYZ789",
            PAYMENT_SIG,
            "rzp_test_secret"
        ));
    }

    #[test]
    fn payment_signature_rejects_tampering_in_every_field() {
        // Swapping order and payment id keeps every byte of the message
        // and only moves the separator: the signature must still fail.
        assert!(!verify_payment_signature(
            "pay_XYZ789",
            "order_ABC123",
            PAYMENT_SIG,
            "rzp_test_secret"
        ));
        assert!(!verify_payment_signature(
            "order_OTHER",
            "pay_XYZ789",
            PAYMENT_SIG,
            "rzp_test_secret"
        ));
        assert!(!verify_payment_signature(
            "order_ABC123",
            "pay_OTHER",
            PAYMENT_SIG,
            "rzp_test_secret"
        ));
        assert!(!verify_payment_signature(
            "order_ABC123",
            "pay_XYZ789",
            PAYMENT_SIG,
            "wrong_secret"
        ));
    }

    #[test]
    fn payment_signature_rejects_empty_and_truncated_signatures() {
        assert!(!verify_payment_signature(
            "order_ABC123",
            "pay_XYZ789",
            "",
            "rzp_test_secret"
        ));
        assert!(!verify_payment_signature(
            "order_ABC123",
            "pay_XYZ789",
            &PAYMENT_SIG[..32],
            "rzp_test_secret"
        ));
    }

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
    }
}
