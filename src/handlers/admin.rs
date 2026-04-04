//! Admin handlers

use worker::*;

use super::get_base_url;
use crate::storage::*;
use crate::templates::*;

/// Unified admin handler - session-protected
pub async fn handle_admin(req: Request, env: Env, path: &str, method: Method) -> Result<Response> {
    let kv = env.kv("CALENDARS_KV")?;

    // Resolve tenant from session cookie
    let tenant_id = match super::auth::resolve_tenant_id(&req, &kv).await {
        Some(id) => id,
        None => {
            let access_user = req
                .headers()
                .get("Cf-Access-Authenticated-User-Email")
                .ok()
                .flatten();

            let is_dev = env
                .var("ENVIRONMENT")
                .map(|v| v.to_string() == "development")
                .unwrap_or(false);

            match access_user {
                Some(email) => email,
                None if is_dev => "default".to_string(),
                None => {
                    let headers = Headers::new();
                    headers.set("Location", "/auth/login")?;
                    return Ok(Response::empty()?.with_status(302).with_headers(headers));
                }
            }
        }
    };

    let base_url = get_base_url(&req);

    if path == "/admin/settings" && method == Method::Get {
        return Response::from_html(admin_settings_html(&base_url));
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

    if path.starts_with("/admin/whatsapp") {
        return super::admin_whatsapp::handle_whatsapp_admin(
            req, env, path, method, &base_url, &tenant_id,
        )
        .await;
    }

    if path.starts_with("/admin/lead-forms") {
        return super::admin_lead_forms::handle_lead_forms_admin(
            req, env, path, method, &base_url, &tenant_id,
        )
        .await;
    }

    if path.starts_with("/admin/instagram") {
        return super::admin_instagram::handle_instagram_admin(
            req, env, path, method, &base_url, &tenant_id,
        )
        .await;
    }

    if path == "/admin" || path == "/admin/" {
        let calendars_kv = env.kv("CALENDARS_KV")?;
        let whatsapp_accounts = list_whatsapp_accounts(&calendars_kv, &tenant_id).await?;
        let instagram_accounts = list_instagram_accounts(&calendars_kv, &tenant_id).await?;
        let lead_forms = list_lead_forms(&calendars_kv, &tenant_id).await?;

        let mut resp = Response::from_html(admin_dashboard_html(
            &whatsapp_accounts,
            &instagram_accounts,
            &lead_forms,
            &base_url,
        ))?;
        resp.headers_mut().set("Cache-Control", "no-store")?;
        return Ok(resp);
    }

    Response::error("Not Found", 404)
}
