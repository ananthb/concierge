use worker::*;

/// Generate a unique ID
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Generate a URL-friendly slug
pub fn generate_slug() -> Result<String> {
    let adjectives = [
        "swift", "bright", "calm", "bold", "warm", "cool", "soft", "keen", "quick", "light",
        "fresh", "clear", "smart", "sharp", "neat", "fine",
    ];
    let nouns = [
        "fox", "owl", "bear", "wolf", "hawk", "deer", "swan", "dove", "lynx", "crow", "hare",
        "seal", "wren", "lark", "moth", "newt",
    ];

    let adj_idx = (js_sys::Math::random() * adjectives.len() as f64) as usize;
    let noun_idx = (js_sys::Math::random() * nouns.len() as f64) as usize;
    let mut rng_bytes = [0u8; 3];
    getrandom::getrandom(&mut rng_bytes)
        .map_err(|e| Error::from(format!("getrandom failed: {}", e)))?;
    let suffix: String = rng_bytes
        .iter()
        .map(|b| {
            let chars = b"abcdefghijklmnopqrstuvwxyz0123456789";
            chars[(*b as usize) % chars.len()] as char
        })
        .collect();

    Ok(format!(
        "{}-{}-{}",
        adjectives[adj_idx], nouns[noun_idx], suffix
    ))
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

/// Get current year-month as "YYYY-MM"
pub fn current_month() -> String {
    let d = js_sys::Date::new_0();
    let y = d.get_utc_full_year();
    let m = d.get_utc_month() + 1; // 0-indexed
    format!("{y:04}-{m:02}")
}

/// Get ISO 8601 timestamp for the last moment of the current UTC month.
pub fn end_of_month() -> String {
    let d = js_sys::Date::new_0();
    let y = d.get_utc_full_year() as i32;
    let m = (d.get_utc_month() + 1) as u32; // 1-indexed
    let last_day = days_in_month(y, m);
    format!("{y:04}-{m:02}-{last_day:02}T23:59:59Z")
}

/// Get ISO 8601 timestamp `days` from now.
pub fn days_from_now(days: i64) -> String {
    let ms = js_sys::Date::now() + (days as f64 * 86_400_000.0);
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
    d.to_iso_string()
        .as_string()
        .unwrap_or_else(|| String::from("2099-12-31T23:59:59.000Z"))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// HTML escape for XSS prevention
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Normalize an origin for comparison (lowercase, no trailing slash)
fn normalize_origin(origin: &str) -> String {
    origin.to_lowercase().trim_end_matches('/').to_string()
}

/// Check if origin is allowed
pub fn is_origin_allowed(origin: &str, allowed_origins: &[String]) -> bool {
    if allowed_origins.is_empty() {
        return true;
    }
    let normalized_origin = normalize_origin(origin);
    allowed_origins
        .iter()
        .any(|allowed| normalize_origin(allowed) == normalized_origin)
}

/// Add CORS headers to response
pub fn with_cors(
    mut response: Response,
    origin: Option<&str>,
    allowed_origins: &[String],
) -> Response {
    if let Some(origin) = origin {
        if is_origin_allowed(origin, allowed_origins) {
            let headers = response.headers_mut();
            let _ = headers.set("Access-Control-Allow-Origin", origin);
            let _ = headers.set(
                "Access-Control-Allow-Methods",
                "GET, POST, PUT, DELETE, OPTIONS",
            );
            let _ = headers.set(
                "Access-Control-Allow-Headers",
                "Content-Type, HX-Request, HX-Target, HX-Current-URL, HX-Trigger",
            );
            let _ = headers.set("Access-Control-Max-Age", "86400");
            let _ = headers.set("Vary", "Origin");
        }
    }

    response
}

/// Handle CORS preflight
pub fn cors_preflight(origin: Option<&str>, allowed_origins: &[String]) -> Result<Response> {
    let mut response = Response::empty()?.with_status(204);

    if let Some(origin) = origin {
        if is_origin_allowed(origin, allowed_origins) {
            let headers = response.headers_mut();
            let _ = headers.set("Access-Control-Allow-Origin", origin);
            let _ = headers.set(
                "Access-Control-Allow-Methods",
                "GET, POST, PUT, DELETE, OPTIONS",
            );
            let _ = headers.set(
                "Access-Control-Allow-Headers",
                "Content-Type, HX-Request, HX-Target, HX-Current-URL, HX-Trigger",
            );
            let _ = headers.set("Access-Control-Max-Age", "86400");
            let _ = headers.set("Vary", "Origin");
        }
    }

    Ok(response)
}

/// Interpolate template variables like {{field_name}} with values
pub fn interpolate_template(
    template: &str,
    fields: &serde_json::Map<String, serde_json::Value>,
) -> String {
    let mut result = template.to_string();
    for (key, value) in fields {
        let placeholder = format!("{{{{{}}}}}", key);
        let replacement = match value {
            serde_json::Value::String(s) => s.clone(),
            _ => value.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}

/// Truncate string to max characters (Unicode-safe).
pub fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Format a count for display in the user's locale. Indian locales (en-IN,
/// hi-IN, ...) get last-3-then-2s grouping (1,00,000); Western locales get
/// thousands grouping (100,000). Backed by icu's `FixedDecimalFormatter`.
pub fn format_count(n: i64, locale: &crate::locale::Locale) -> String {
    use icu::decimal::{options::FixedDecimalFormatterOptions, FixedDecimalFormatter};
    use icu::locid::Locale as IcuLocale;

    // unic_langid -> icu_locid via string round-trip; both are BCP-47.
    let icu_locale: IcuLocale = locale
        .langid
        .to_string()
        .parse()
        .unwrap_or_else(|_| icu::locid::locale!("en-IN"));
    let formatter =
        FixedDecimalFormatter::try_new(&icu_locale.into(), FixedDecimalFormatterOptions::default())
            .expect("locale supported by compiled_data");
    let value: fixed_decimal::FixedDecimal = n.into();
    formatter.format(&value).to_string()
}

/// Format a money amount in the smallest currency unit (paise / cents) for
/// display in the user's locale. Indian-locale outputs use ₹ + Indian
/// grouping; US-locale outputs use $ + thousands grouping with two
/// decimals. The currency comes from the locale; the amount is always in
/// minor units.
pub fn format_money(amount_minor: i64, locale: &crate::locale::Locale) -> String {
    use crate::locale::Currency;
    match locale.currency {
        Currency::Inr => {
            // Whole rupees only on display (we don't price in fractional
            // rupees anywhere). Round half-up to keep parity with the
            // historical `amount / 100` truncation behavior.
            let rupees = amount_minor.div_euclid(100);
            format!("{}{}", Currency::Inr.symbol(), format_count(rupees, locale))
        }
        Currency::Usd => {
            let dollars = amount_minor / 100;
            let cents = (amount_minor % 100).abs();
            format!(
                "{}{}.{:02}",
                Currency::Usd.symbol(),
                format_count(dollars, locale),
                cents
            )
        }
    }
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

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("hello"), "hello");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(html_escape("it's"), "it&#x27;s");
    }

    #[test]
    fn test_is_origin_allowed() {
        assert!(is_origin_allowed("https://example.com", &[]));

        let allowed = vec!["https://example.com".to_string()];
        assert!(is_origin_allowed("https://example.com", &allowed));
        assert!(is_origin_allowed("https://EXAMPLE.COM", &allowed));
        assert!(is_origin_allowed("https://example.com/", &allowed));
        assert!(!is_origin_allowed("https://other.com", &allowed));
    }

    use crate::locale::Locale;

    #[test]
    fn test_format_count_indian() {
        let l = Locale::default_inr();
        assert_eq!(format_count(0, &l), "0");
        assert_eq!(format_count(999, &l), "999");
        assert_eq!(format_count(1_000, &l), "1,000");
        assert_eq!(format_count(100_000, &l), "1,00,000");
        assert_eq!(format_count(12_345_678, &l), "1,23,45,678");
    }

    #[test]
    fn test_format_count_western() {
        let l = Locale::default_usd();
        assert_eq!(format_count(1_000, &l), "1,000");
        assert_eq!(format_count(100_000, &l), "100,000");
        assert_eq!(format_count(1_234_567, &l), "1,234,567");
    }

    #[test]
    fn test_format_money_inr() {
        let l = Locale::default_inr();
        // amount in paise; ₹1,00,000.00 → 1 lakh rupees = 1_00_00_000 paise
        assert_eq!(format_money(1_00_00_000, &l), "₹1,00,000");
        assert_eq!(format_money(20_000_00, &l), "₹20,000");
        assert_eq!(format_money(2_00, &l), "₹2");
    }

    #[test]
    fn test_format_money_usd() {
        let l = Locale::default_usd();
        // amount in cents
        assert_eq!(format_money(2, &l), "$0.02");
        assert_eq!(format_money(50, &l), "$0.50");
        assert_eq!(format_money(2_50, &l), "$2.50");
        assert_eq!(format_money(20_000_00, &l), "$20,000.00");
    }

    #[test]
    fn test_interpolate_template() {
        let mut fields = serde_json::Map::new();
        fields.insert(
            "name".to_string(),
            serde_json::Value::String("Alice".to_string()),
        );
        fields.insert("count".to_string(), serde_json::Value::Number(42.into()));

        assert_eq!(
            interpolate_template("Hello {{name}}!", &fields),
            "Hello Alice!"
        );
        assert_eq!(
            interpolate_template("{{name}} has {{count}} items", &fields),
            "Alice has 42 items"
        );
    }
}
