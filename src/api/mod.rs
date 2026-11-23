//! JSON API. The frontend's only interface to the worker.

use serde::Serialize;
use worker::*;

mod account;
mod billing;
mod bootstrap;
pub mod demo;
mod settings;
mod whatsapp;
mod wizard;

const REQUEST_HEADER: &str = "X-Replybots-Request";

/// Dispatch an `/api/*` request. `path` is the full request path.
pub async fn handle(req: Request, env: Env, path: &str) -> Result<Response> {
    let method = req.method();
    let sub = path.strip_prefix("/api/").unwrap_or("");

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
        ["billing", rest @ ..] => billing::handle(req, env, rest).await,
        ["settings", rest @ ..] => settings::handle(req, env, rest).await,

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

pub fn not_found(what: &str) -> Result<Response> {
    error("not_found", &format!("{what} not found."), 404)
}

pub fn invalid(message: &str) -> Result<Response> {
    error("invalid", message, 400)
}

pub fn no_content() -> Result<Response> {
    let headers = Headers::new();
    headers.set("Cache-Control", "no-store")?;
    Ok(Response::empty()?.with_status(204).with_headers(headers))
}

// ---------------------------------------------------------------------------
// Auth & Body helpers
// ---------------------------------------------------------------------------

pub async fn require_tenant(
    req: &Request,
    env: &Env,
) -> std::result::Result<String, Result<Response>> {
    let db = match env.d1("DB") {
        Ok(db) => db,
        Err(e) => return Err(Err(e)),
    };
    match crate::handlers::auth::resolve_tenant_id(req, &db).await {
        Some(id) => Ok(id),
        None => Err(error("unauthenticated", "Sign in to continue.", 401)),
    }
}

pub async fn read_json(
    req: &mut Request,
) -> std::result::Result<serde_json::Value, Result<Response>> {
    match req.json::<serde_json::Value>().await {
        Ok(v) => Ok(v),
        Err(_) => Err(invalid("Malformed request body.")),
    }
}

pub fn field(body: &serde_json::Value, key: &str) -> String {
    body.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

pub fn field_i64(body: &serde_json::Value, key: &str) -> Option<i64> {
    let v = body.get(key)?;
    v.as_i64().or_else(|| v.as_str()?.trim().parse().ok())
}
