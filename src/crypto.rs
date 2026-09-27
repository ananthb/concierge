//! Webhook signature verification.
//!
//! The AES-256-GCM token encryption that used to live here went with the
//! Instagram channel: it existed to seal per-tenant Instagram long-lived
//! tokens in KV. WhatsApp uses one platform-wide token from a secret, so
//! nothing needs sealing and `ENCRYPTION_KEY` is no longer read.

use worker::*;

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Compute HMAC-SHA256 and return as hex
pub fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> Result<String> {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    let mut mac =
        Hmac::<Sha256>::new_from_slice(key).map_err(|_| Error::from("Invalid HMAC key"))?;
    mac.update(data);
    let result = mac.finalize();
    Ok(hex_encode(&result.into_bytes()))
}

/// Verify a Meta webhook signature (X-Hub-Signature-256 header)
/// Returns true if signature is valid
pub fn verify_meta_signature(app_secret: &str, body: &[u8], signature_header: &str) -> bool {
    let expected = signature_header.strip_prefix("sha256=").unwrap_or("");
    if expected.is_empty() {
        return false;
    }
    let computed = match hmac_sha256_hex(app_secret.as_bytes(), body) {
        Ok(c) => c,
        Err(_) => return false,
    };
    // Constant-time comparison
    use subtle::ConstantTimeEq;
    if computed.len() != expected.len() {
        return false;
    }
    computed.as_bytes().ct_eq(expected.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 4231 §4.2 — HMAC-SHA-256 test case 1.
    #[test]
    fn hmac_sha256_matches_rfc4231_case_1() {
        let key = [0x0b_u8; 20];
        assert_eq!(
            hmac_sha256_hex(&key, b"Hi There").unwrap(),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    /// RFC 4231 §4.3 — test case 2, a key shorter than the block size.
    #[test]
    fn hmac_sha256_matches_rfc4231_case_2() {
        assert_eq!(
            hmac_sha256_hex(b"Jefe", b"what do ya want for nothing?").unwrap(),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    /// RFC 4231 §4.6 — test case 5 truncates its expected value, so the
    /// interesting property here is the over-long key (131 bytes) taking
    /// the hash-the-key branch.
    #[test]
    fn hmac_sha256_accepts_a_key_longer_than_the_block() {
        let key = [0xaa_u8; 131];
        assert_eq!(
            hmac_sha256_hex(
                &key,
                b"Test Using Larger Than Block-Size Key - Hash Key First"
            )
            .unwrap(),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn hmac_sha256_accepts_an_empty_key_and_empty_data() {
        assert_eq!(
            hmac_sha256_hex(b"", b"").unwrap(),
            "b613679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad"
        );
    }

    #[test]
    fn meta_signature_accepts_the_signature_it_computes() {
        let secret = "app-secret";
        let body = br#"{"object":"whatsapp_business_account"}"#;
        let sig = format!(
            "sha256={}",
            hmac_sha256_hex(secret.as_bytes(), body).unwrap()
        );
        assert!(verify_meta_signature(secret, body, &sig));
    }

    #[test]
    fn meta_signature_rejects_a_wrong_secret_body_or_prefix() {
        let secret = "app-secret";
        let body = br#"{"object":"whatsapp_business_account"}"#;
        let sig = format!(
            "sha256={}",
            hmac_sha256_hex(secret.as_bytes(), body).unwrap()
        );

        assert!(!verify_meta_signature("other-secret", body, &sig));
        assert!(!verify_meta_signature(secret, b"tampered", &sig));
        // Header without the algorithm prefix, and with the wrong one.
        let bare = sig.strip_prefix("sha256=").unwrap();
        assert!(!verify_meta_signature(secret, body, bare));
        assert!(!verify_meta_signature(
            secret,
            body,
            &format!("sha1={bare}")
        ));
    }

    #[test]
    fn meta_signature_rejects_an_empty_or_truncated_signature() {
        let secret = "app-secret";
        let body = b"payload";
        assert!(!verify_meta_signature(secret, body, ""));
        assert!(!verify_meta_signature(secret, body, "sha256="));
        let full = hmac_sha256_hex(secret.as_bytes(), body).unwrap();
        assert!(!verify_meta_signature(
            secret,
            body,
            &format!("sha256={}", &full[..32])
        ));
    }
}
