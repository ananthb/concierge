//! Per-tenant settings the dashboard can change: the conversation window,
//! and the locale/currency pair.
//!
//! These were readable but not writable before — the pipeline honoured
//! `ConversationConfig`, and nothing could set it, so every tenant ran on
//! the `prompt::DEFAULT_*` constants. Locale was fixed at signup.
//!
//! The three conversation knobs are stored as `Option`s and resolve to the
//! defaults when unset, so the wire format distinguishes three cases:
//!
//! * field absent      — leave whatever is stored
//! * field is `null`   — clear the override, go back to the default
//! * field is a number — set the override
//!
//! Without that a form could only ever set values, never return one to its
//! default, and "unset" is the state most tenants should be in.

use serde::Serialize;
use worker::*;

use crate::locale::Currency;
use crate::storage;
use crate::types::ConversationConfig;

/// Accepted bounds for each knob, mirroring the invariants `prompt.rs`
/// const-asserts for the defaults. Inclusive on both ends.
const IDLE_GAP_RANGE: (u32, u32) = (5, 24 * 60);
const HANDOFF_COOLDOWN_RANGE: (u32, u32) = (5, 24 * 60);
const MAX_HISTORY_RANGE: (u32, u32) = (4, 200);

/// The locale tags `locale::parse_supported` accepts. Anything else falls
/// back to en-IN silently, which would make a saved value look accepted and
/// then not apply, so the endpoint rejects it instead.
const SUPPORTED_LOCALES: &[&str] = &["en-IN", "en-US"];
const SUPPORTED_CURRENCIES: &[&str] = &["INR", "USD"];

#[derive(Serialize)]
pub struct SettingsView {
    conversation: ConversationView,
    locale: String,
    currency: String,
    locales: &'static [&'static str],
    currencies: &'static [&'static str],
}

#[derive(Serialize)]
struct ConversationView {
    /// `None` means "no override" — the UI shows the default as a
    /// placeholder rather than pre-filling it, so saving an untouched form
    /// doesn't silently pin today's default forever.
    idle_gap_mins: Option<u32>,
    handoff_cooldown_mins: Option<u32>,
    max_history_messages: Option<u32>,
    defaults: ConversationBounds,
    min: ConversationBounds,
    max: ConversationBounds,
}

#[derive(Serialize)]
struct ConversationBounds {
    idle_gap_mins: u32,
    handoff_cooldown_mins: u32,
    max_history_messages: u32,
}

pub async fn handle(mut req: Request, env: Env, rest: &[&str]) -> Result<Response> {
    if !rest.is_empty() {
        return super::not_found("Endpoint");
    }
    let tenant_id = match super::require_tenant(&req, &env).await {
        Ok(t) => t,
        Err(r) => return r,
    };
    match req.method() {
        Method::Get => get(&env, &tenant_id).await,
        Method::Put => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            put(&env, &tenant_id, &body).await
        }
        _ => super::not_found("Endpoint"),
    }
}

async fn get(env: &Env, tenant_id: &str) -> Result<Response> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;
    let state = storage::get_onboarding(&kv, tenant_id).await?;
    let Some(tenant) = storage::get_tenant(&db, tenant_id).await? else {
        return super::not_found("Account");
    };
    super::json(&view(&state.conversation, &tenant.locale, tenant.currency))
}

async fn put(env: &Env, tenant_id: &str, body: &serde_json::Value) -> Result<Response> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;
    let mut state = storage::get_onboarding(&kv, tenant_id).await?;
    let Some(mut tenant) = storage::get_tenant(&db, tenant_id).await? else {
        return super::not_found("Account");
    };

    if let Some(patch) = body.get("conversation") {
        match apply_conversation(&state.conversation, patch) {
            Ok(next) => {
                state.conversation = next;
                storage::save_onboarding(&kv, tenant_id, &state).await?;
            }
            Err(message) => return super::invalid(&message),
        }
    }

    let mut tenant_changed = false;
    if let Some(value) = body.get("locale") {
        match parse_locale(value) {
            Ok(tag) => {
                tenant.locale = tag;
                tenant_changed = true;
            }
            Err(message) => return super::invalid(&message),
        }
    }
    if let Some(value) = body.get("currency") {
        match parse_currency(value) {
            Ok(currency) => {
                tenant.currency = currency;
                tenant_changed = true;
            }
            Err(message) => return super::invalid(&message),
        }
    }
    if tenant_changed {
        tenant.updated_at = crate::helpers::now_iso();
        storage::save_tenant(&db, &tenant).await?;
    }

    super::json(&view(&state.conversation, &tenant.locale, tenant.currency))
}

/// Apply a `conversation` patch to what is stored.
///
/// Pure, and separate from the handler, because everything interesting about
/// this endpoint lives here: the three-way absent/null/number reading, the
/// bounds, and the ordering invariant. Behind `require_tenant` none of it is
/// reachable from a test without a session, so it is tested directly.
fn apply_conversation(
    current: &ConversationConfig,
    patch: &serde_json::Value,
) -> std::result::Result<ConversationConfig, String> {
    let mut next = current.clone();
    for (key, range, label) in [
        ("idle_gap_mins", IDLE_GAP_RANGE, "The idle gap"),
        (
            "handoff_cooldown_mins",
            HANDOFF_COOLDOWN_RANGE,
            "The handoff cooldown",
        ),
        ("max_history_messages", MAX_HISTORY_RANGE, "The history cap"),
    ] {
        let parsed = match patch.get(key) {
            None => continue,
            Some(serde_json::Value::Null) => None,
            Some(_) => {
                let Some(n) = super::field_i64(patch, key) else {
                    return Err(format!("{label} must be a whole number."));
                };
                let (lo, hi) = range;
                if n < lo as i64 || n > hi as i64 {
                    return Err(format!("{label} must be between {lo} and {hi}."));
                }
                Some(n as u32)
            }
        };
        match key {
            "idle_gap_mins" => next.idle_gap_mins = parsed,
            "handoff_cooldown_mins" => next.handoff_cooldown_mins = parsed,
            _ => next.max_history_messages = parsed,
        }
    }

    // The pipeline treats silence longer than the idle gap as the end of a
    // conversation, and the handoff cooldown is how long it keeps answering
    // in the holding voice. A cooldown that outlives the gap would keep
    // holding a conversation that has already ended, so the resolved pair
    // has to preserve the ordering `prompt.rs` asserts for the defaults.
    // Checked after resolution, since either side may be an override or a
    // default.
    let window = next.resolve();
    if window.handoff_cooldown_mins >= window.idle_gap_mins {
        return Err(
            "The handoff cooldown has to be shorter than the idle gap, or a handed-over conversation would end before the cooldown does."
                .to_string(),
        );
    }
    Ok(next)
}

fn parse_locale(value: &serde_json::Value) -> std::result::Result<String, String> {
    let tag = value.as_str().unwrap_or("").trim().to_string();
    if !SUPPORTED_LOCALES.contains(&tag.as_str()) {
        return Err(format!(
            "Unsupported locale. Pick one of: {}.",
            SUPPORTED_LOCALES.join(", ")
        ));
    }
    Ok(tag)
}

/// `Currency::parse` falls back to INR for anything it doesn't recognise, so
/// an unknown code would look accepted and quietly mean something else. This
/// rejects instead.
fn parse_currency(value: &serde_json::Value) -> std::result::Result<Currency, String> {
    let code = value.as_str().unwrap_or("").trim().to_ascii_uppercase();
    if !SUPPORTED_CURRENCIES.contains(&code.as_str()) {
        return Err(format!(
            "Unsupported currency. Pick one of: {}.",
            SUPPORTED_CURRENCIES.join(", ")
        ));
    }
    Ok(Currency::parse(&code))
}

fn view(conversation: &ConversationConfig, locale: &str, currency: Currency) -> SettingsView {
    SettingsView {
        conversation: ConversationView {
            idle_gap_mins: conversation.idle_gap_mins,
            handoff_cooldown_mins: conversation.handoff_cooldown_mins,
            max_history_messages: conversation.max_history_messages,
            defaults: ConversationBounds {
                idle_gap_mins: crate::prompt::DEFAULT_CONVERSATION_IDLE_GAP_MINS as u32,
                handoff_cooldown_mins: crate::prompt::DEFAULT_HANDOFF_COOLDOWN_MINS as u32,
                max_history_messages: crate::prompt::DEFAULT_CONVERSATION_MAX_MESSAGES,
            },
            min: ConversationBounds {
                idle_gap_mins: IDLE_GAP_RANGE.0,
                handoff_cooldown_mins: HANDOFF_COOLDOWN_RANGE.0,
                max_history_messages: MAX_HISTORY_RANGE.0,
            },
            max: ConversationBounds {
                idle_gap_mins: IDLE_GAP_RANGE.1,
                handoff_cooldown_mins: HANDOFF_COOLDOWN_RANGE.1,
                max_history_messages: MAX_HISTORY_RANGE.1,
            },
        },
        locale: locale.to_string(),
        currency: currency.as_str().to_string(),
        locales: SUPPORTED_LOCALES,
        currencies: SUPPORTED_CURRENCIES,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn unset() -> ConversationConfig {
        ConversationConfig::default()
    }

    /// The bounds have to admit the defaults, or a tenant who has never
    /// touched the form couldn't save one back.
    #[test]
    fn defaults_sit_inside_the_accepted_ranges() {
        let d = unset().resolve();
        assert!(d.idle_gap_mins >= IDLE_GAP_RANGE.0 as i64);
        assert!(d.idle_gap_mins <= IDLE_GAP_RANGE.1 as i64);
        assert!(d.handoff_cooldown_mins >= HANDOFF_COOLDOWN_RANGE.0 as i64);
        assert!(d.handoff_cooldown_mins <= HANDOFF_COOLDOWN_RANGE.1 as i64);
        assert!(d.max_history_messages >= MAX_HISTORY_RANGE.0);
        assert!(d.max_history_messages <= MAX_HISTORY_RANGE.1);
    }

    #[test]
    fn default_cooldown_is_shorter_than_the_default_gap() {
        let d = unset().resolve();
        assert!(d.handoff_cooldown_mins < d.idle_gap_mins);
    }

    #[test]
    fn a_number_sets_the_override() {
        // Above the default cooldown: a gap below it is refused by the
        // ordering check, which `the_invariant_holds_against_defaults…`
        // covers.
        let next = apply_conversation(&unset(), &json!({ "idle_gap_mins": 120 })).unwrap();
        assert_eq!(next.idle_gap_mins, Some(120));
    }

    /// The case a PATCH-shaped body exists for: leave the rest alone.
    #[test]
    fn an_absent_field_leaves_the_stored_value() {
        let current = ConversationConfig {
            idle_gap_mins: Some(120),
            handoff_cooldown_mins: Some(30),
            max_history_messages: Some(12),
        };
        let next = apply_conversation(&current, &json!({ "max_history_messages": 8 })).unwrap();
        assert_eq!(next.idle_gap_mins, Some(120));
        assert_eq!(next.handoff_cooldown_mins, Some(30));
        assert_eq!(next.max_history_messages, Some(8));
    }

    /// And the case that makes `null` load-bearing: going back to the
    /// default is not expressible any other way.
    #[test]
    fn null_clears_an_override_back_to_the_default() {
        let current = ConversationConfig {
            idle_gap_mins: Some(120),
            ..Default::default()
        };
        let next = apply_conversation(&current, &json!({ "idle_gap_mins": null })).unwrap();
        assert_eq!(next.idle_gap_mins, None);
        assert_eq!(
            next.resolve().idle_gap_mins,
            crate::prompt::DEFAULT_CONVERSATION_IDLE_GAP_MINS
        );
    }

    #[test]
    fn a_string_number_is_accepted_because_the_form_sends_text() {
        let next = apply_conversation(&unset(), &json!({ "max_history_messages": "16" })).unwrap();
        assert_eq!(next.max_history_messages, Some(16));
    }

    #[test]
    fn a_non_number_is_refused_rather_than_ignored() {
        let err = apply_conversation(&unset(), &json!({ "idle_gap_mins": "soon" })).unwrap_err();
        assert!(err.contains("whole number"), "{err}");
    }

    #[test]
    fn values_outside_the_range_are_refused_at_both_ends() {
        for value in [IDLE_GAP_RANGE.0 - 1, IDLE_GAP_RANGE.1 + 1] {
            let err = apply_conversation(&unset(), &json!({ "idle_gap_mins": value })).unwrap_err();
            assert!(err.contains("between"), "{err}");
        }
    }

    /// The invariant that matters at runtime: a cooldown outliving the gap
    /// would hold a conversation that has already ended.
    #[test]
    fn a_cooldown_at_or_past_the_gap_is_refused() {
        let err = apply_conversation(
            &unset(),
            &json!({ "idle_gap_mins": 60, "handoff_cooldown_mins": 60 }),
        )
        .unwrap_err();
        assert!(err.contains("shorter than the idle gap"), "{err}");

        let err = apply_conversation(
            &unset(),
            &json!({ "idle_gap_mins": 60, "handoff_cooldown_mins": 90 }),
        )
        .unwrap_err();
        assert!(err.contains("shorter than the idle gap"), "{err}");
    }

    /// Checked against the *resolved* pair, so lowering only the gap can
    /// collide with a default cooldown the request never mentioned.
    #[test]
    fn the_invariant_holds_against_defaults_the_request_did_not_send() {
        let below_default_cooldown = crate::prompt::DEFAULT_HANDOFF_COOLDOWN_MINS - 1;
        let err = apply_conversation(
            &unset(),
            &json!({ "idle_gap_mins": below_default_cooldown }),
        )
        .unwrap_err();
        assert!(err.contains("shorter than the idle gap"), "{err}");
    }

    #[test]
    fn locale_and_currency_accept_only_what_ships() {
        assert_eq!(parse_locale(&json!("en-US")).unwrap(), "en-US");
        assert!(parse_locale(&json!("fr-FR")).is_err());
        assert!(parse_locale(&json!(null)).is_err());

        assert_eq!(parse_currency(&json!("usd")).unwrap(), Currency::Usd);
        // The one that would otherwise pass silently: `Currency::parse`
        // answers INR for anything unknown.
        assert!(parse_currency(&json!("EUR")).is_err());
    }

    #[test]
    fn every_supported_currency_parses_back_to_itself() {
        for code in SUPPORTED_CURRENCIES {
            assert_eq!(Currency::parse(code).as_str(), *code);
        }
    }
}
