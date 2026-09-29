//! JSON API. The Elm frontend's only interface to the worker.
//!
//! Everything under `/api/` answers JSON and never HTML. The rules:
//!
//! - **Envelope.** Success returns the payload at the top level. Failure
//!   returns `{"error": {"code": "...", "message": "..."}}` with a matching
//!   HTTP status. `code` is a stable machine string the frontend matches on;
//!   `message` is shown to the user as-is.
//! - **Auth.** The session cookie set by `/auth/callback` is the only
//!   credential. [`require_tenant`] resolves it or answers 401 — the frontend
//!   treats a 401 as "show the login screen", which is why signed-out state is
//!   a status code rather than a redirect.
//! - **CSRF.** State-changing requests must carry `X-Concierge-Request: 1`.
//!   The session cookie is `SameSite=Lax`, so a cross-site form POST can't
//!   reach here; a custom header can't be set cross-origin without CORS
//!   preflight, which we never grant. This replaces the per-tenant CSRF token
//!   the HTML forms used, which existed because those were real form posts.
//! - **No caching.** Every response is `no-store`.
//!
//! Redirect-based flows stay outside this module: Google OAuth
//! (`/auth/*`), the WhatsApp Embedded Signup callback (`/whatsapp/signup/*`)
//! and the webhooks all need to speak their counterparty's protocol.

use serde::Serialize;
use worker::*;

mod account;
mod billing;
mod bootstrap;
mod demo;
mod manage;
mod persona;
mod settings;
mod whatsapp;
mod wizard;

/// Header a state-changing request must carry. Any value is accepted; its
/// presence is the signal.
const REQUEST_HEADER: &str = "X-Concierge-Request";

/// Dispatch an `/api/*` request. `path` is the full request path.
pub async fn handle(req: Request, env: Env, path: &str) -> Result<Response> {
    let method = req.method();
    let sub = path.strip_prefix("/api/").unwrap_or("");

    // Reject state-changing requests without the custom header before any
    // handler runs, so a missing header can never be mistaken for a
    // validation error on the payload.
    if !matches!(method, Method::Get | Method::Head)
        && req.headers().get(REQUEST_HEADER).ok().flatten().is_none()
    {
        return error(
            "missing_request_header",
            "Refusing an unmarked request.",
            403,
        );
    }

    let segments: Vec<&str> = sub.split('/').filter(|s| !s.is_empty()).collect();

    match segments.as_slice() {
        // --- public ---------------------------------------------------
        ["bootstrap"] => bootstrap::get(req, env).await,
        ["demo", "chat"] if method == Method::Post => demo::chat(req, env).await,

        // --- session --------------------------------------------------
        ["session"] if method == Method::Get => account::get_session(req, env).await,
        ["session"] if method == Method::Delete => account::logout(req, env).await,
        ["account"] if method == Method::Delete => account::delete(req, env).await,

        // --- onboarding wizard ----------------------------------------
        ["wizard", rest @ ..] => wizard::handle(req, env, rest).await,

        // --- signed-in app --------------------------------------------
        ["whatsapp", rest @ ..] => whatsapp::handle(req, env, rest).await,
        ["persona", rest @ ..] => persona::handle(req, env, rest).await,
        ["archetypes"] if method == Method::Get => persona::list_archetypes_handler(req, env).await,
        ["billing", rest @ ..] => billing::handle(req, env, rest).await,

        // Conversation window + locale. Readable and writable by the
        // tenant; the operator API does not duplicate these.
        ["settings", rest @ ..] => settings::handle(req, env, rest).await,

        // --- operator (Cloudflare Access) -----------------------------
        ["manage", rest @ ..] => manage::handle(req, env, rest).await,

        _ => error("not_found", "No such endpoint.", 404),
    }
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    error: ErrorBody<'a>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
}

/// Serialize `body` as a 200 JSON response.
pub fn json<T: Serialize>(body: &T) -> Result<Response> {
    json_status(body, 200)
}

/// Serialize `body` as a JSON response with an explicit status.
pub fn json_status<T: Serialize>(body: &T, status: u16) -> Result<Response> {
    let serialized = serde_json::to_string(body)
        .map_err(|e| Error::from(format!("Failed to serialize response: {e}")))?;
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Cache-Control", "no-store")?;
    Ok(Response::ok(serialized)?
        .with_status(status)
        .with_headers(headers))
}

/// A machine-matchable error. `code` is stable; `message` is user-facing.
pub fn error(code: &str, message: &str, status: u16) -> Result<Response> {
    json_status(
        &ErrorEnvelope {
            error: ErrorBody { code, message },
        },
        status,
    )
}

/// 404 for a resource the caller may or may not be allowed to see.
///
/// Used for another tenant's records too: distinguishing "doesn't exist"
/// from "not yours" would leak which ids are real.
pub fn not_found(what: &str) -> Result<Response> {
    error("not_found", &format!("{what} not found."), 404)
}

/// 400 with a field-level validation message.
pub fn invalid(message: &str) -> Result<Response> {
    error("invalid", message, 400)
}

/// Empty 204, for deletes and other calls with nothing to return.
pub fn no_content() -> Result<Response> {
    let headers = Headers::new();
    headers.set("Cache-Control", "no-store")?;
    Ok(Response::empty()?.with_status(204).with_headers(headers))
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

/// Resolve the caller's tenant from the session cookie.
///
/// `Err(response)` is a ready-to-return 401. Callers use
/// `let tenant_id = match require_tenant(&req, &env).await { Ok(t) => t,
/// Err(r) => return r };` — the frontend flips to the login screen on 401.
pub async fn require_tenant(
    req: &Request,
    env: &Env,
) -> std::result::Result<String, Result<Response>> {
    let kv = match env.kv("KV") {
        Ok(kv) => kv,
        Err(e) => return Err(Err(e)),
    };
    match crate::handlers::auth::resolve_tenant_id(req, &kv).await {
        Some(id) => Ok(id),
        None => Err(error("unauthenticated", "Sign in to continue.", 401)),
    }
}

/// Parse a JSON request body, or answer 400.
pub async fn read_json(
    req: &mut Request,
) -> std::result::Result<serde_json::Value, Result<Response>> {
    match req.json::<serde_json::Value>().await {
        Ok(v) => Ok(v),
        Err(_) => Err(invalid("Malformed request body.")),
    }
}

/// Read a trimmed string field from a JSON object. Absent or non-string
/// fields read as empty.
pub fn field(body: &serde_json::Value, key: &str) -> String {
    body.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Read a bool field, defaulting to false.
pub fn field_bool(body: &serde_json::Value, key: &str) -> bool {
    body.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Read an integer field, accepting both `5` and `"5"` so the frontend can
/// send whatever its decoder produces.
pub fn field_i64(body: &serde_json::Value, key: &str) -> Option<i64> {
    let v = body.get(key)?;
    v.as_i64().or_else(|| v.as_str()?.trim().parse().ok())
}
