//! Locale model: BCP-47 language identifier + display currency, with
//! request-time resolution from tenant > Accept-Language > cf-ipcountry.
//!
//! All number / currency / date rendering goes through `helpers::format_*`
//! which takes a `&Locale`. Templates and handlers carry a `Locale` rather
//! than a raw `currency: String` so adding a third currency or a new
//! language is a single-file change here, not a sweep across the codebase.

use unic_langid::{langid, LanguageIdentifier};
use worker::Request;

/// Display currency. Bound to a small fixed set since each currency requires
/// its own pricing config in `billing/mod.rs` and routing in Razorpay; this
/// is not a bag of arbitrary ISO-4217 codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    Inr,
    Usd,
}

impl Currency {
    pub fn as_str(self) -> &'static str {
        match self {
            Currency::Inr => "INR",
            Currency::Usd => "USD",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Currency::Inr => "\u{20B9}", // ₹
            Currency::Usd => "$",
        }
    }

    pub fn parse(s: &str) -> Currency {
        match s.to_ascii_uppercase().as_str() {
            "USD" => Currency::Usd,
            _ => Currency::Inr,
        }
    }
}

/// Resolved per-request locale used everywhere strings are rendered.
#[derive(Debug, Clone)]
pub struct Locale {
    pub langid: LanguageIdentifier,
    pub currency: Currency,
}

impl Locale {
    /// Hardcoded final fallback. Matches the historical INR-default.
    pub fn default_inr() -> Self {
        Self {
            langid: langid!("en-IN"),
            currency: Currency::Inr,
        }
    }

    pub fn default_usd() -> Self {
        Self {
            langid: langid!("en-US"),
            currency: Currency::Usd,
        }
    }

    /// Build a locale from a stored `Tenant.locale` tag plus an optional
    /// stored `Tenant.currency` override. Tenant-level configuration always
    /// wins over request-time signals.
    pub fn from_tenant(tag: &str, currency_override: Option<&str>) -> Self {
        let langid = parse_supported(tag).unwrap_or_else(|| langid!("en-IN"));
        let currency = currency_override
            .map(Currency::parse)
            .unwrap_or_else(|| currency_for_langid(&langid));
        Self { langid, currency }
    }

    /// Build a locale from request-time signals only. Used before we know
    /// who the tenant is (signup, public marketing pages).
    pub fn from_request(req: &Request) -> Self {
        // Accept-Language wins if it parses to a supported tag.
        if let Ok(Some(al)) = req.headers().get("Accept-Language") {
            if let Some(langid) = best_match_from_accept_language(&al) {
                let currency = currency_for_langid(&langid);
                return Self { langid, currency };
            }
        }
        // cf-ipcountry as the next-best signal.
        if let Ok(Some(country)) = req.headers().get("cf-ipcountry") {
            let langid = country_to_langid(&country);
            let currency = currency_for_langid(&langid);
            return Self { langid, currency };
        }
        Self::default_inr()
    }
}

/// Pick the best supported langid from a possibly-ranked Accept-Language
/// header. Falls back to `None` if nothing supported is mentioned.
fn best_match_from_accept_language(header: &str) -> Option<LanguageIdentifier> {
    let supported = ["en-IN", "en-US"];
    let parsed = accept_language::intersection(header, &supported);
    parsed.into_iter().next().and_then(|tag| tag.parse().ok())
}

/// Map an ISO-3166 country code to a supported langid. Conservative: only
/// India is mapped to `en-IN`; everything else falls through to `en-US`.
/// Expand here as new locales come online.
fn country_to_langid(country: &str) -> LanguageIdentifier {
    match country.to_ascii_uppercase().as_str() {
        "IN" => langid!("en-IN"),
        _ => langid!("en-US"),
    }
}

/// Default display currency for a langid. Tenant override on the settings
/// page bypasses this.
fn currency_for_langid(langid: &LanguageIdentifier) -> Currency {
    match langid.region.as_ref().map(|r| r.as_str()).unwrap_or("") {
        "IN" => Currency::Inr,
        _ => Currency::Usd,
    }
}

/// Parse a stored locale tag, returning Some only if it's one we ship data
/// for. Unknown tags fall back to en-IN at the call site.
fn parse_supported(tag: &str) -> Option<LanguageIdentifier> {
    let parsed: LanguageIdentifier = tag.parse().ok()?;
    let key = parsed.to_string();
    match key.as_str() {
        "en-IN" | "en-US" => Some(parsed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_parse() {
        assert_eq!(Currency::parse("INR"), Currency::Inr);
        assert_eq!(Currency::parse("USD"), Currency::Usd);
        assert_eq!(Currency::parse("usd"), Currency::Usd);
        assert_eq!(Currency::parse("eur"), Currency::Inr); // unknown -> default
    }

    #[test]
    fn from_tenant_with_override() {
        let l = Locale::from_tenant("en-IN", Some("USD"));
        assert_eq!(l.langid.to_string(), "en-IN");
        assert_eq!(l.currency, Currency::Usd);
    }

    #[test]
    fn from_tenant_unknown_tag_falls_back() {
        let l = Locale::from_tenant("klingon", None);
        assert_eq!(l.langid.to_string(), "en-IN");
        assert_eq!(l.currency, Currency::Inr);
    }

    #[test]
    fn country_mapping() {
        assert_eq!(country_to_langid("IN").to_string(), "en-IN");
        assert_eq!(country_to_langid("US").to_string(), "en-US");
        assert_eq!(country_to_langid("XX").to_string(), "en-US");
    }

    #[test]
    fn currency_for_region() {
        assert_eq!(currency_for_langid(&langid!("en-IN")), Currency::Inr);
        assert_eq!(currency_for_langid(&langid!("en-US")), Currency::Usd);
    }
}
