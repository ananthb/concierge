//! Admin handlers

use worker::*;

use super::get_base_url;
use crate::storage::*;
use crate::templates::*;

/// Unified admin handler - session-protected
pub async fn handle_admin(req: Request, env: Env, path: &str, method: Method) -> Result<Response> {
    let kv = env.kv("KV")?;

    // Resolve tenant from session cookie only: no header fallback
    let tenant_id = match super::auth::resolve_tenant_id(&req, &kv).await {
        Some(id) => id,
        None => {
            let headers = Headers::new();
            headers.set("Location", "/auth/login")?;
            return Ok(Response::empty()?.with_status(302).with_headers(headers));
        }
    };

    let base_url = get_base_url(&req);
    let locale = crate::locale::Locale::from_request(&req);

    // CSRF validation on state-changing requests
    if matches!(method, Method::Post | Method::Put | Method::Delete) {
        if let Err(e) = super::auth::validate_csrf(&req, &kv, &tenant_id).await {
            return Response::error(format!("CSRF validation failed: {e}"), 403);
        }
    }

    if path == "/admin/settings" && method == Method::Get {
        let db = env.d1("DB")?;
        let tenant = get_tenant(&db, &tenant_id)
            .await?
            .unwrap_or_else(|| crate::types::Tenant {
                id: tenant_id.clone(),
                email: tenant_id.clone(),
                ..Default::default()
            });
        let google_client_id = env
            .secret("GOOGLE_OAUTH_CLIENT_ID")
            .map(|s| s.to_string())
            .unwrap_or_default();
        let meta_app_id = env
            .secret("META_APP_ID")
            .map(|s| s.to_string())
            .unwrap_or_default();
        let wa = list_whatsapp_accounts(&kv, &tenant_id).await?;
        let ig = list_instagram_accounts(&kv, &tenant_id).await?;
        let dc = get_discord_config_by_tenant(&kv, &tenant_id).await?;
        let onboarding = get_onboarding(&kv, &tenant_id).await?;
        return Response::from_html(admin_settings_html(
            &tenant,
            &base_url,
            &google_client_id,
            &meta_app_id,
            &wa,
            &ig,
            dc.as_ref(),
            &onboarding.conversation,
            &tenant_id,
            &locale,
        ));
    }

    if path == "/admin/settings/currency" && method == Method::Put {
        let db = env.d1("DB")?;
        let mut req = req;
        let form: serde_json::Value = req.json().await?;
        // Currency and locale are independent: a tenant can read English-IN
        // copy with USD prices, or vice versa. Both are accepted in the same
        // PUT so the settings page can offer them as paired controls.
        let currency = form
            .get("currency")
            .and_then(|v| v.as_str())
            .map(crate::locale::Currency::parse);
        let locale_str = form
            .get("locale")
            .and_then(|v| v.as_str())
            .filter(|s| matches!(*s, "en-IN" | "en-US"));

        if let Some(mut tenant) = get_tenant(&db, &tenant_id).await? {
            let mut changed = false;
            if let Some(c) = currency {
                if tenant.currency != c {
                    tenant.currency = c;
                    changed = true;
                }
            }
            if let Some(l) = locale_str {
                if tenant.locale != l {
                    tenant.locale = l.to_string();
                    changed = true;
                }
            }
            if changed {
                tenant.updated_at = crate::helpers::now_iso();
                save_tenant(&db, &tenant).await?;
            }
        }

        return Response::from_html("<div class=\"success\">Settings updated.</div>".to_string());
    }

    if path == "/admin/settings/conversation" && method == Method::Put {
        return save_conversation_settings(req, &env, &kv, &tenant_id, &locale).await;
    }

    if path == "/admin/delete-account" && method == Method::Delete {
        let db = env.d1("DB")?;
        delete_tenant_data(&kv, &db, &tenant_id).await?;

        // Clear session cookie
        let headers = Headers::new();
        headers.set("Location", "/")?;
        headers.set(
            "Set-Cookie",
            "session=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0",
        )?;
        return Ok(Response::empty()?.with_status(302).with_headers(headers));
    }

    if path.starts_with("/admin/billing") {
        return super::admin_billing::handle_billing_admin(req, env, path, &base_url, &tenant_id)
            .await;
    }

    if path.starts_with("/admin/whatsapp") {
        return super::admin_whatsapp::handle_whatsapp_admin(req, env, path, &base_url, &tenant_id)
            .await;
    }

    if path.starts_with("/admin/lead-forms") {
        return super::admin_lead_forms::handle_lead_forms_admin(
            req, env, path, &base_url, &tenant_id,
        )
        .await;
    }

    if path.starts_with("/admin/instagram") {
        return super::admin_instagram::handle_instagram_admin(
            req, env, path, &base_url, &tenant_id,
        )
        .await;
    }

    if path.starts_with("/admin/email") {
        return super::admin_email::handle_email_admin(req, env, path, &base_url, &tenant_id).await;
    }

    if path.starts_with("/admin/discord") {
        return super::discord_oauth::handle_discord_admin(req, env, path, &base_url, &tenant_id)
            .await;
    }

    if path.starts_with("/admin/wizard") {
        return super::onboarding::handle_wizard(req, env, path, &base_url, &tenant_id).await;
    }

    if path.starts_with("/admin/persona") {
        return super::admin_persona::handle_persona_admin(req, env, path, &base_url, &tenant_id)
            .await;
    }

    if path.starts_with("/admin/rules/") {
        return super::admin_rules::handle_rules(req, env, path, &base_url, &tenant_id).await;
    }

    if path == "/admin/approvals" || path.starts_with("/admin/approvals/") {
        return super::admin_approvals::handle_approvals(req, env, path, &base_url, &tenant_id)
            .await;
    }

    if path == "/admin/risk-gate-banner/dismiss" && method == Method::Post {
        let mut state = crate::storage::get_onboarding(&kv, &tenant_id).await?;
        if !state.risk_gate_banner_dismissed {
            state.risk_gate_banner_dismissed = true;
            crate::storage::save_onboarding(&kv, &tenant_id, &state).await?;
        }
        // HTMX swaps the banner element out by replacing it with empty.
        return Response::ok("");
    }

    if path == "/admin" || path == "/admin/" {
        let kv = env.kv("KV")?;

        // Redirect to onboarding if not completed
        let onboarding = crate::storage::get_onboarding(&kv, &tenant_id).await?;
        if !onboarding.completed {
            let headers = Headers::new();
            headers.set("Location", &format!("{}/admin/wizard", base_url))?;
            return Ok(Response::empty()?.with_status(302).with_headers(headers));
        }

        let whatsapp_accounts = list_whatsapp_accounts(&kv, &tenant_id).await?;
        let instagram_accounts = list_instagram_accounts(&kv, &tenant_id).await?;
        let lead_forms = list_lead_forms(&kv, &tenant_id).await?;
        let email_addrs = crate::storage::get_email_addresses(&kv, &tenant_id).await?;
        let db = env.d1("DB")?;
        let mut billing = crate::storage::get_tenant_billing(&db, &tenant_id).await?;
        crate::billing::refresh_billing(&mut billing);

        let mut resp = Response::from_html(admin_dashboard_html(
            &whatsapp_accounts,
            &instagram_accounts,
            &lead_forms,
            &billing,
            &email_addrs,
            &base_url,
            !onboarding.risk_gate_banner_dismissed,
            &locale,
        ))?;
        resp.headers_mut().set("Cache-Control", "no-store")?;
        return Ok(resp);
    }

    Response::error("Not Found", 404)
}

/// PUT /admin/settings/conversation
///
/// Updates the tenant's `ConversationConfig` on `OnboardingState`.
/// Each of the three fields is optional: a missing or empty value
/// clears the override (falls back to the prompt-default at runtime).
/// Validation enforces sane bounds and the
/// `idle_gap_mins > handoff_cooldown_mins` invariant — otherwise an
/// active handoff could be wiped before its cooldown ended. On
/// failure we render an error toast and don't write.
async fn save_conversation_settings(
    mut req: Request,
    env: &Env,
    kv: &kv::KvStore,
    tenant_id: &str,
    locale: &crate::locale::Locale,
) -> Result<Response> {
    use crate::i18n::t;
    let _ = env;

    let form: serde_json::Value = req.json().await?;

    fn parse_optional_u32(
        form: &serde_json::Value,
        key: &str,
    ) -> std::result::Result<Option<u32>, ()> {
        let raw = form.get(key);
        match raw {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(serde_json::Value::String(s)) => {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    Ok(None)
                } else {
                    trimmed.parse::<u32>().map(Some).map_err(|_| ())
                }
            }
            Some(serde_json::Value::Number(n)) => match n.as_u64() {
                Some(v) if v <= u32::MAX as u64 => Ok(Some(v as u32)),
                _ => Err(()),
            },
            _ => Err(()),
        }
    }

    let idle_gap_mins = match parse_optional_u32(&form, "idle_gap_mins") {
        Ok(v) => v,
        Err(()) => {
            return error_toast(&t(locale, "admin-settings-conversation-error-idle-bounds"));
        }
    };
    let handoff_cooldown_mins = match parse_optional_u32(&form, "handoff_cooldown_mins") {
        Ok(v) => v,
        Err(()) => {
            return error_toast(&t(
                locale,
                "admin-settings-conversation-error-cooldown-bounds",
            ));
        }
    };
    let max_history_messages = match parse_optional_u32(&form, "max_history_messages") {
        Ok(v) => v,
        Err(()) => {
            return error_toast(&t(
                locale,
                "admin-settings-conversation-error-history-bounds",
            ));
        }
    };

    if let Some(v) = idle_gap_mins {
        if !(5..=1440).contains(&v) {
            return error_toast(&t(locale, "admin-settings-conversation-error-idle-bounds"));
        }
    }
    if let Some(v) = handoff_cooldown_mins {
        if !(5..=1440).contains(&v) {
            return error_toast(&t(
                locale,
                "admin-settings-conversation-error-cooldown-bounds",
            ));
        }
    }
    if let Some(v) = max_history_messages {
        if !(1..=200).contains(&v) {
            return error_toast(&t(
                locale,
                "admin-settings-conversation-error-history-bounds",
            ));
        }
    }

    // Cross-field invariant: a non-default idle gap must still be
    // strictly larger than the (effective) cooldown, otherwise the
    // conversation can end mid-handoff and wipe the holding-pattern
    // state. Compare against effective values so the form catches
    // even the case where one field is set and the other isn't.
    let effective_idle = idle_gap_mins
        .map(|v| v as i64)
        .unwrap_or(crate::prompt::DEFAULT_CONVERSATION_IDLE_GAP_MINS);
    let effective_cooldown = handoff_cooldown_mins
        .map(|v| v as i64)
        .unwrap_or(crate::prompt::DEFAULT_HANDOFF_COOLDOWN_MINS);
    if effective_idle <= effective_cooldown {
        return error_toast(&t(
            locale,
            "admin-settings-conversation-error-idle-vs-cooldown",
        ));
    }

    let mut state = crate::storage::get_onboarding(kv, tenant_id).await?;
    state.conversation = crate::types::ConversationConfig {
        idle_gap_mins,
        handoff_cooldown_mins,
        max_history_messages,
    };
    crate::storage::save_onboarding(kv, tenant_id, &state).await?;

    Response::from_html(format!(
        "<div class=\"success\">{}</div>",
        t(locale, "admin-settings-conversation-saved")
    ))
}

fn error_toast(message: &str) -> Result<Response> {
    Response::from_html(format!(
        "<div class=\"error\">{}</div>",
        crate::helpers::html_escape(message)
    ))
}
