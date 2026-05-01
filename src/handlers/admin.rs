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
        return Response::from_html(admin_settings_html(
            &tenant,
            &base_url,
            &google_client_id,
            &meta_app_id,
            &wa,
            &ig,
            dc.as_ref(),
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
        crate::billing::refresh_billing_async(&db, &mut billing).await;

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
