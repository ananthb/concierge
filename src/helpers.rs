//! Small shared utilities.
//!
//! Number and money formatting used to live here, backed by icu4x and
//! rusty-money. It moved to the frontend: `Intl.NumberFormat` produces the
//! same Indian lakh grouping (₹1,00,000) from the browser's own locale data,
//! which saved carrying compiled ICU data in the wasm bundle. The API returns
//! amounts in minor units and an ISO 4217 code; the frontend renders them.

use worker::*;

/// Generate a unique ID
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Generate a secure token
pub fn generate_token() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes)
        .map_err(|e| Error::from(format!("getrandom failed: {}", e)))?;
    Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
}

/// Get current ISO timestamp
pub fn now_iso() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_else(|| String::from("1970-01-01T00:00:00.000Z"))
}

/// Get ISO 8601 timestamp `days` from now.
pub fn days_from_now(days: i64) -> String {
    let ms = js_sys::Date::now() + (days as f64 * 86_400_000.0);
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
    d.to_iso_string()
        .as_string()
        .unwrap_or_else(|| String::from("2099-12-31T23:59:59.000Z"))
}

/// HTML escape for XSS prevention.
///
/// Only the schema-reseed report renders HTML now; everything else answers
/// JSON, where `serde_json` does the escaping.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Truncate string to max characters (Unicode-safe).
pub fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Hex SHA-256 of a string. Used to detect drift in safety-checked content.
pub fn sha256_hex(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let bytes = h.finalize();
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes.iter() {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Independently computed SHA-256 digests: this is the hash that
    /// detects drift in safety-checked persona content, so a change in
    /// its output would silently invalidate every stored prompt_hash.
    #[test]
    fn sha256_hex_matches_known_digests() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // Multi-byte input, to pin that we hash UTF-8 bytes.
        assert_eq!(sha256_hex("ஸுபா").len(), 64);
        assert_ne!(sha256_hex("abc"), sha256_hex("abd"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("hello"), "hello");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(html_escape("it's"), "it&#x27;s");
    }

    #[test]
    fn truncate_is_unicode_safe() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 2), "he");
        // Multi-byte: a naive byte slice would panic mid-codepoint here.
        assert_eq!(truncate("ஸுபா", 2), "ஸு");
    }
}
