//! Per-tenant settings: conversation window, handoff notification email,
//! and locale/currency pair.

use serde::Serialize;
use worker::*;

use crate::locale::Currency;
use crate::storage;
use crate::types::{ConversationConfig, NotificationConfig};

const IDLE_GAP_RANGE: (u32, u32) = (5, 24 * 60);
const HANDOFF_COOLDOWN_RANGE: (u32, u32) = (5, 24 * 60);
const MAX_HISTORY_RANGE: (u32, u32) = (4, 200);

const SUPPORTED_LOCALES: &[&str] = &["en-IN", "en-US"];
const SUPPORTED_CURRENCIES: &[&str] = &["INR", "USD"];

#[derive(Serialize)]
pub struct SettingsView {
    conversation: ConversationView,
    notifications: NotificationsView,
    locale: String,
    currency: String,
    locales: &'static [&'static str],
    currencies: &'static [&'static str],
}

#[derive(Serialize)]
struct NotificationsView {
    handoff_email: bool,
}

#[derive(Serialize)]
struct ConversationView {
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
    let db = env.d1("DB")?;
    let state = storage::get_onboarding(&db, tenant_id).await?;
    let Some(tenant) = storage::get_tenant(&db, tenant_id).await? else {
        return super::not_found("Account");
    };
    super::json(&view(
        &state.conversation,
        &state.notifications,
        &tenant.locale,
        tenant.currency,
    ))
}

async fn put(env: &Env, tenant_id: &str, body: &serde_json::Value) -> Result<Response> {
    let db = env.d1("DB")?;
    let mut state = storage::get_onboarding(&db, tenant_id).await?;
    let Some(mut tenant) = storage::get_tenant(&db, tenant_id).await? else {
        return super::not_found("Account");
    };

    let mut onboarding_changed = false;
    if let Some(patch) = body.get("conversation") {
        match apply_conversation(&state.conversation, patch) {
            Ok(next) => {
                state.conversation = next;
                onboarding_changed = true;
            }
            Err(message) => return super::invalid(&message),
        }
    }

    if let Some(notif) = body.get("notifications") {
        if let Some(v) = notif.get("handoff_email").and_then(|v| v.as_bool()) {
            state.notifications.handoff_email = v;
            onboarding_changed = true;
        }
    }

    if onboarding_changed {
        storage::save_onboarding(&db, tenant_id, &state).await?;
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

    super::json(&view(
        &state.conversation,
        &state.notifications,
        &tenant.locale,
        tenant.currency,
    ))
}

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

fn view(
    conversation: &ConversationConfig,
    notifications: &NotificationConfig,
    locale: &str,
    currency: Currency,
) -> SettingsView {
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
        notifications: NotificationsView {
            handoff_email: notifications.handoff_email,
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
    fn a_number_sets_the_override_and_null_clears_it() {
        let next = apply_conversation(&unset(), &json!({ "idle_gap_mins": 120 })).unwrap();
        assert_eq!(next.idle_gap_mins, Some(120));
        let cleared = apply_conversation(&next, &json!({ "idle_gap_mins": null })).unwrap();
        assert_eq!(cleared.idle_gap_mins, None);
    }

    #[test]
    fn a_cooldown_at_or_past_the_gap_is_refused() {
        let err = apply_conversation(
            &unset(),
            &json!({ "idle_gap_mins": 60, "handoff_cooldown_mins": 60 }),
        )
        .unwrap_err();
        assert!(err.contains("shorter than the idle gap"), "{err}");
    }

    #[test]
    fn locale_and_currency_accept_only_what_ships() {
        assert_eq!(parse_locale(&json!("en-US")).unwrap(), "en-US");
        assert!(parse_locale(&json!("fr-FR")).is_err());
        assert_eq!(parse_currency(&json!("usd")).unwrap(), Currency::Usd);
        assert!(parse_currency(&json!("EUR")).is_err());
    }
}
