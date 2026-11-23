//! Session and account endpoints.

use serde::Serialize;
use worker::*;

use crate::storage;

#[derive(Serialize)]
pub struct SessionView {
    pub tenant_id: String,
    pub email: String,
    pub name: Option<String>,
    pub locale: String,
    pub currency: String,
    pub metered: bool,
    pub destination: &'static str,
    pub onboarding_step: Option<String>,
    pub ai_ready: bool,
    pub safety_status: String,
    pub safety_reason: Option<String>,
}

pub async fn resolve_session(req: &Request, env: &Env) -> Result<Option<SessionView>> {
    let db = env.d1("DB")?;
    let Some(tenant_id) = crate::handlers::auth::resolve_tenant_id(req, &db).await else {
        return Ok(None);
    };
    let Some(tenant) = storage::get_tenant(&db, &tenant_id).await? else {
        return Ok(None);
    };
    let state = storage::get_onboarding(&db, &tenant_id).await?;

    Ok(Some(SessionView {
        tenant_id: tenant.id.clone(),
        email: tenant.email.clone(),
        name: tenant.name.clone(),
        locale: tenant.locale.clone(),
        currency: tenant.currency.as_str().to_string(),
        metered: tenant.plan.is_metered(),
        destination: if state.completed {
            "dashboard"
        } else {
            "wizard"
        },
        onboarding_step: if state.completed {
            None
        } else {
            Some(state.step.as_str().to_string())
        },
        ai_ready: state.profile_safety.is_approved_for(&state.business.name),
        safety_status: state.profile_safety.status.as_str().to_string(),
        safety_reason: state.profile_safety.vague_reason,
    }))
}

/// `GET /api/session`
pub async fn get_session(req: Request, env: Env) -> Result<Response> {
    match resolve_session(&req, &env).await? {
        Some(s) => super::json(&s),
        None => super::error("unauthenticated", "Sign in to continue.", 401),
    }
}

/// `DELETE /api/session` — sign out.
pub async fn logout(req: Request, env: Env) -> Result<Response> {
    let db = env.d1("DB")?;
    if let Some(token) = crate::handlers::auth::get_session_cookie(&req) {
        let _ = storage::delete_session(&db, &token).await;
    }
    let headers = Headers::new();
    headers.set("Cache-Control", "no-store")?;
    headers.set(
        "Set-Cookie",
        "session=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0",
    )?;
    Ok(Response::empty()?.with_status(204).with_headers(headers))
}

/// `DELETE /api/account` — delete the tenant and everything keyed to it.
pub async fn delete(mut req: Request, env: Env) -> Result<Response> {
    let tenant_id = match super::require_tenant(&req, &env).await {
        Ok(t) => t,
        Err(r) => return r,
    };
    let db = env.d1("DB")?;

    let body = match super::read_json(&mut req).await {
        Ok(b) => b,
        Err(r) => return r,
    };
    let confirm = super::field(&body, "confirm_email");
    let confirm = confirm.trim();
    let tenant = match storage::get_tenant(&db, &tenant_id).await? {
        Some(t) => t,
        None => return super::not_found("Account"),
    };
    let confirmed = if tenant.email.trim().is_empty() {
        confirm.eq_ignore_ascii_case("DELETE")
    } else {
        confirm.eq_ignore_ascii_case(&tenant.email) || confirm.eq_ignore_ascii_case("DELETE")
    };
    if !confirmed {
        let msg = if tenant.email.trim().is_empty() {
            "Type DELETE to confirm."
        } else {
            "Type your account's email address to confirm."
        };
        return super::invalid(msg);
    }

    storage::delete_tenant_data(&db, &tenant_id).await?;
    if let Some(token) = crate::handlers::auth::get_session_cookie(&req) {
        let _ = storage::delete_session(&db, &token).await;
    }

    let headers = Headers::new();
    headers.set("Cache-Control", "no-store")?;
    headers.set(
        "Set-Cookie",
        "session=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0",
    )?;
    Ok(Response::empty()?.with_status(204).with_headers(headers))
}
