//! Translation layer: Fluent message bundles, one per supported locale,
//! loaded once at first use. Templates and handlers call `t(locale, key)`
//! to look up a localized string.
//!
//! Supported locales are baked in via `include_str!`; adding a new locale
//! is a two-step change (add FTL file under `assets/locales/{tag}/`,
//! register in `Translator::new`).
//!
//! Lookup falls back: requested locale → en-IN (canonical bundle) → the
//! key itself, which is also what the build-time check enforces no key is
//! missing from en-IN.

use std::sync::OnceLock;

use fluent::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::langid;

use crate::locale::Locale;

/// FTL source for each baked locale. `include_str!` so adding a string in
/// the source tree is a build-time guarantee, not a deploy-time fetch.
const EN_IN_FTL: &str = include_str!("../assets/locales/en-IN/messages.ftl");
const EN_US_FTL: &str = include_str!("../assets/locales/en-US/messages.ftl");

/// Canonical bundle: every key MUST exist here; other bundles can override.
const CANONICAL_TAG: &str = "en-IN";

pub struct Translator {
    en_in: FluentBundle<FluentResource>,
    en_us: FluentBundle<FluentResource>,
}

impl Translator {
    fn new() -> Self {
        let en_in = make_bundle(langid!("en-IN"), EN_IN_FTL);
        let en_us = make_bundle(langid!("en-US"), EN_US_FTL);
        Self { en_in, en_us }
    }

    fn bundle_for(&self, locale: &Locale) -> &FluentBundle<FluentResource> {
        match locale.langid.to_string().as_str() {
            "en-US" => &self.en_us,
            _ => &self.en_in,
        }
    }

    fn canonical(&self) -> &FluentBundle<FluentResource> {
        &self.en_in
    }

    /// Look up `key` in the locale's bundle, falling back to the canonical
    /// (en-IN) bundle, then the key itself if neither has it. Args are
    /// substituted via Fluent's `{ $name }` syntax.
    pub fn t(&self, locale: &Locale, key: &str, args: Option<&FluentArgs>) -> String {
        if let Some(s) = format_from(self.bundle_for(locale), key, args) {
            return s;
        }
        if let Some(s) = format_from(self.canonical(), key, args) {
            return s;
        }
        key.to_string()
    }
}

fn format_from(
    bundle: &FluentBundle<FluentResource>,
    key: &str,
    args: Option<&FluentArgs>,
) -> Option<String> {
    let msg = bundle.get_message(key)?;
    let pattern = msg.value()?;
    let mut errors = Vec::new();
    let result = bundle.format_pattern(pattern, args, &mut errors);
    if !errors.is_empty() {
        // worker::console_log! is wasm-bindgen-backed and panics on the
        // host targets used by `cargo test`. Gate it to wasm only so a
        // missing-arg bug surfaces as a benign log in production but
        // doesn't abort the host test runner.
        #[cfg(target_arch = "wasm32")]
        worker::console_log!("i18n: {key} formatting issues: {errors:?}");
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!("i18n: {key} formatting issues: {errors:?}");
    }
    Some(result.into_owned())
}

fn make_bundle(
    langid: unic_langid::LanguageIdentifier,
    src: &'static str,
) -> FluentBundle<FluentResource> {
    let resource = FluentResource::try_new(src.to_string())
        .unwrap_or_else(|(_, errs)| panic!("FTL parse failed for {langid}: {errs:?}"));
    let mut bundle = FluentBundle::new(vec![langid.clone()]);
    // Workers AI doesn't ship Unicode bidi isolates the way browsers do; the
    // extra characters Fluent inserts around variables (\u2068 / \u2069)
    // would render as garbage in plain HTML output.
    bundle.set_use_isolating(false);
    bundle
        .add_resource(resource)
        .unwrap_or_else(|errs| panic!("FTL load failed for {langid}: {errs:?}"));
    bundle
}

/// `Sync` wrapper for the translator. `FluentBundle` carries an internal
/// `RefCell` for custom transforms, so it isn't auto-`Sync`. WASM workers
/// are single-threaded by spec so the unsafe impl is sound: no two
/// concurrent borrows can exist. Same pattern as `worker::Queue`.
struct SyncTranslator(Translator);
unsafe impl Sync for SyncTranslator {}
unsafe impl Send for SyncTranslator {}

/// Singleton translator. Built lazily on first use.
pub fn translator() -> &'static Translator {
    static T: OnceLock<SyncTranslator> = OnceLock::new();
    &T.get_or_init(|| SyncTranslator(Translator::new())).0
}

/// Sugar for the common no-args case.
pub fn t(locale: &Locale, key: &str) -> String {
    translator().t(locale, key, None)
}

/// Look up `key` and substitute `{ $name }` placeholders from `args`.
/// Each tuple is `(name, value)`. Values are coerced to FluentValue::String.
pub fn t_args(locale: &Locale, key: &str, args: &[(&str, &str)]) -> String {
    let mut fa = FluentArgs::new();
    for (k, v) in args {
        fa.set(*k, *v);
    }
    translator().t(locale, key, Some(&fa))
}

/// Asserts that the canonical bundle has a key for every supplied id.
/// Used by the build-time integration test in `templates::base` to fail
/// the build when a `t(..)` reference points at a missing FTL key.
#[cfg(test)]
pub fn canonical_has_key(key: &str) -> bool {
    translator().canonical().get_message(key).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_loads_and_formats() {
        let l = Locale::default_inr();
        assert_eq!(t(&l, "nav-features"), "Features");
        assert_eq!(t(&l, "footer-licence"), "AGPL-3.0");
    }

    #[test]
    fn missing_key_returns_key() {
        let l = Locale::default_inr();
        assert_eq!(t(&l, "no-such-key"), "no-such-key");
    }

    #[test]
    fn en_us_falls_back_to_canonical() {
        // en-US bundle is empty; lookups should fall through to en-IN.
        let l = Locale::default_usd();
        assert_eq!(t(&l, "nav-pricing"), "Pricing");
    }

    #[test]
    fn t_args_substitutes_price_placeholders() {
        let l = Locale::default_inr();
        // Real key from the FTL. Must contain { $inr } and { $usd }.
        let s = t_args(
            &l,
            "features-card-pay-body",
            &[("inr", "₹0.10"), ("usd", "$0.001")],
        );
        assert!(s.contains("₹0.10"), "missing inr substitution: {s}");
        assert!(s.contains("$0.001"), "missing usd substitution: {s}");
        // Sanity: placeholder syntax should not leak into output.
        assert!(!s.contains("{ $"), "placeholder leaked: {s}");
    }

    #[test]
    fn t_args_with_custom_prices() {
        let l = Locale::default_inr();
        let s = t_args(
            &l,
            "pricing-meta-description",
            &[
                ("inr", "₹0.50"),
                ("usd", "$0.005"),
                ("addr_inr", "₹199"),
                ("addr_usd", "$2"),
                ("pack_size", "5"),
            ],
        );
        assert!(s.contains("₹0.50"));
        assert!(s.contains("$0.005"));
        assert!(s.contains("₹199"));
        assert!(s.contains("$2"));
    }
}
