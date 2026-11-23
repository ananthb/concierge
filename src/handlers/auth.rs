//! Meta / WhatsApp sign-in and local dev login.

use worker::*;

use crate::helpers::*;
use crate::storage::*;
use crate::types::*;

const SESSION_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;
const REQUEST_HEADER: &str = "X-Replybots-Request";

/// Handle auth routes (`/auth/*`).
pub async fn handle_auth(req: Request, env: Env, path: &str, method: Method) -> Result<Response> {
    match (method, path) {
        (Method::Post, "/auth/whatsapp/start") => {
            if req.headers().get(REQUEST_HEADER).ok().flatten().is_none() {
                return crate::api::error(
                    "missing_request_header",
                    "Refusing an unmarked request.",
                    403,
                );
            }
            let db = env.d1("DB")?;
            match crate::api::whatsapp::SignupView::resolve(&env, &db, "").await? {
                Some(view) => crate::api::json(&view),
                None => crate::api::error(
                    "signup_not_configured",
                    "WhatsApp login is not configured on this deploy.",
                    503,
                ),
            }
        }

        // Dev-only login shortcut gated on `DEV_MODE` (`dev_bypass::active`).
        (Method::Post, "/auth/dev-login") => {
            if !crate::dev_bypass::active(&env) {
                return Response::error("Not Found", 404);
            }
            let mut req = req;
            let email_raw = req
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("email").and_then(|e| e.as_str()).map(str::to_string))
                .unwrap_or_default();
            let email = if email_raw.trim().is_empty() {
                "dev@local.test".to_string()
            } else {
                email_raw.trim().to_lowercase()
            };

            let db = env.d1("DB")?;
            let tenant = match get_tenant_by_email(&db, &email).await? {
                Some(t) => t,
                None => {
                    let signup_locale = crate::locale::Locale::from_request(&req);
                    let now = now_iso();
                    let t = Tenant {
                        id: generate_id(),
                        email: email.clone(),
                        name: Some(email.split('@').next().unwrap_or("Dev").to_string()),
                        facebook_id: None,
                        plan: Plan::Paid,
                        locale: signup_locale.langid.to_string(),
                        currency: signup_locale.currency,
                        created_at: now.clone(),
                        updated_at: now,
                    };
                    save_tenant(&db, &t).await?;
                    t
                }
            };

            create_session_and_redirect(&req, &db, &tenant.id).await
        }

        _ => Response::error("Not Found", 404),
    }
}

pub(super) async fn mint_session_cookie(
    req: &Request,
    db: &D1Database,
    tenant_id: &str,
) -> Result<String> {
    let session_token = generate_token()?;
    save_session(db, &session_token, tenant_id, SESSION_TTL_SECONDS).await?;

    let is_https = req
        .url()
        .ok()
        .map(|u| u.scheme() == "https")
        .unwrap_or(true);
    let secure_attr = if is_https { "; Secure" } else { "" };

    Ok(format!(
        "session={token}; Path=/; HttpOnly{secure}; SameSite=Lax; Max-Age={ttl}",
        token = session_token,
        secure = secure_attr,
        ttl = SESSION_TTL_SECONDS,
    ))
}

pub(super) async fn create_session_and_redirect(
    req: &Request,
    db: &D1Database,
    tenant_id: &str,
) -> Result<Response> {
    let cookie = mint_session_cookie(req, db, tenant_id).await?;

    let headers = Headers::new();
    headers.set("Location", "/")?;
    headers.set("Cache-Control", "no-store")?;
    headers.set("Set-Cookie", &cookie)?;
    Ok(Response::empty()?.with_status(302).with_headers(headers))
}

pub fn get_cookie(req: &Request, name: &str) -> Option<String> {
    let cookie_header = req.headers().get("Cookie").ok()??;
    let prefix = format!("{name}=");
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix(&prefix) {
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub fn get_session_cookie(req: &Request) -> Option<String> {
    get_cookie(req, "session")
}

pub async fn resolve_tenant_id(req: &Request, db: &D1Database) -> Option<String> {
    let token = get_session_cookie(req)?;
    get_session(db, &token).await.ok()?
}
