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
        ));
    }

    if path == "/admin/settings/currency" && method == Method::Put {
        let db = env.d1("DB")?;
        let mut req = req;
        let form: serde_json::Value = req.json().await?;
        let currency = form
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("INR");
        let currency = if currency == "USD" { "USD" } else { "INR" };

        if let Some(mut tenant) = get_tenant(&db, &tenant_id).await? {
            tenant.currency = currency.to_string();
            tenant.updated_at = crate::helpers::now_iso();
            save_tenant(&db, &tenant).await?;
        }

        return Response::from_html("<div class=\"success\">Currency updated.</div>".to_string());
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
        ))?;
        resp.headers_mut().set("Cache-Control", "no-store")?;
        return Ok(resp);
    }

    Response::error("Not Found", 404)
}
