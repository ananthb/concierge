//! Session and account endpoints.
//!
//! `GET /api/session` is what the SPA polls after a sign-in redirect lands
//! back on the app. [`SessionView`] is also embedded in `/api/bootstrap`,
//! so a cold load needs exactly one request.

use serde::Serialize;
use worker::*;

use crate::storage;

/// Who is signed in, and where the app should send them.
#[derive(Serialize)]
pub struct SessionView {
    pub tenant_id: String,
    pub email: String,
    pub name: Option<String>,
    pub locale: String,
    pub currency: String,
    /// True for a paying (metered) account; false for complimentary ones,
    /// which can't buy credits.
    pub metered: bool,
    /// Where the app should route this session: the wizard until it's
    /// sealed, the dashboard after.
    pub destination: &'static str,
    /// Wizard progress. `None` once onboarding is complete.
    pub onboarding_step: Option<String>,
    /// AI replies are blocked while this is false — the persona's safety
    /// verdict is pending or rejected. The dashboard surfaces it as a
    /// banner, so it belongs in the session rather than behind a second
    /// request.
    pub ai_ready: bool,
}

/// Resolve the caller's session, or `None` when signed out.
///
/// Returns `Ok(None)` rather than an error for a signed-out caller:
/// `/api/bootstrap` is public and being signed out is not a failure there.
pub async fn resolve_session(req: &Request, env: &Env) -> Result<Option<SessionView>> {
    let kv = env.kv("KV")?;
    let Some(tenant_id) = crate::handlers::auth::resolve_tenant_id(req, &kv).await else {
        return Ok(None);
    };
    let db = env.d1("DB")?;
    let Some(tenant) = storage::get_tenant(&db, &tenant_id).await? else {
        // Session cookie outlived the tenant record. Treat as signed out;
        // the stale cookie is harmless and expires on its own.
        return Ok(None);
    };
    let state = storage::get_onboarding(&kv, &tenant_id).await?;

    // The persona's safety verdict gates AI replies. Resolving it needs the
    // archetype's voice prompt, because the approved-hash comparison is over
    // the *composed* prompt.
    let voice_prompt = match &state.persona.source {
        crate::types::PersonaSource::Builder(b) => {
            match storage::get_archetype_cached(&kv, &db, &b.archetype_slug).await {
                Ok(Some(a)) => a.voice_prompt,
                _ => String::new(),
            }
        }
        crate::types::PersonaSource::Custom(_) => String::new(),
    };

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
        ai_ready: state.persona.is_safe_to_use(&voice_prompt),
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
///
/// Deletes the KV session so the token is dead server-side, then clears the
/// cookie. Answers 204 either way: signing out twice is not an error.
pub async fn logout(req: Request, env: Env) -> Result<Response> {
    let kv = env.kv("KV")?;
    if let Some(token) = crate::handlers::auth::get_session_cookie(&req) {
        let _ = storage::delete_session(&kv, &token).await;
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
///
/// Irreversible. Wipes KV and D1 records per
/// [`storage::delete_tenant_data`]; payment rows survive with a null
/// tenant_id for tax and dispute records.
pub async fn delete(mut req: Request, env: Env) -> Result<Response> {
    let tenant_id = match super::require_tenant(&req, &env).await {
        Ok(t) => t,
        Err(r) => return r,
    };
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    // Typed confirmation, same guard the old form had: the client must echo
    // the account's own email address.
    let body = match super::read_json(&mut req).await {
        Ok(b) => b,
        Err(r) => return r,
    };
    let confirm = super::field(&body, "confirm_email").to_lowercase();
    let tenant = match storage::get_tenant(&db, &tenant_id).await? {
        Some(t) => t,
        None => return super::not_found("Account"),
    };
    if confirm != tenant.email.to_lowercase() {
        return super::invalid("Type your account's email address to confirm.");
    }

    storage::delete_tenant_data(&kv, &db, &tenant_id).await?;
    if let Some(token) = crate::handlers::auth::get_session_cookie(&req) {
        let _ = storage::delete_session(&kv, &token).await;
    }

    let headers = Headers::new();
    headers.set("Cache-Control", "no-store")?;
    headers.set(
        "Set-Cookie",
        "session=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0",
    )?;
    Ok(Response::empty()?.with_status(204).with_headers(headers))
}
