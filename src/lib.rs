//! # Concierge
//!
//! Messaging automation for small businesses: WhatsApp auto-replies.
//!
//! This is a Cloudflare Worker built with Rust + WebAssembly. It handles:
//!
//! - **WhatsApp webhooks**: incoming messages trigger auto-replies (static or AI)
//! - **JSON API**: `/api/*`, the only interface the frontend has
//! - **OAuth**: Google sign-in
//!
//! The UI is an Elm single-page app in `frontend/`, compiled to
//! `public/app.js` and served by Cloudflare's asset handler. The Worker
//! serves the SPA shell for every non-API, non-webhook route.
//!
//! ## Architecture
//!
//! - `types`: Core data structures (Tenant, WhatsAppAccount)
//! - `storage`: Cloudflare KV and D1 operations
//! - `api`: JSON endpoints, the frontend's only interface
//! - `ai`: Cloudflare Workers AI integration for auto-reply generation
//! - `whatsapp`: Meta Graph API client for sending WhatsApp messages
//! - `crypto`: AES-256-GCM encryption and HMAC-SHA256 verification
//! - `helpers`: ID generation, HTML escaping, CORS, template interpolation

use worker::*;

mod ai;
mod api;
mod billing;
mod channel;
mod crypto;
mod dev_bypass;
mod durable_objects;
mod email;
mod escalations;
mod handlers;
mod helpers;
mod locale;
mod management;
mod personas;
mod pipeline;
mod prompt;
mod risk;
mod safety;
mod safety_queue;
mod scheduled;
mod shell;
mod storage;
mod types;
mod whatsapp;

pub use durable_objects::ReplyBufferDO;
pub use types::*;

/// Meta Graph API version used across all Facebook/WhatsApp/Instagram API calls.
pub const META_API_VERSION: &str = "v21.0";

// Static assets embedded at compile time
const LOGO_SVG: &str = include_str!("../assets/logo.svg");
const WEBMANIFEST: &str = include_str!("../assets/site.webmanifest");
const BROWSERCONFIG: &str = include_str!("../assets/browserconfig.xml");
const FAVICON_16: &[u8] = include_bytes!("../assets/favicon-16.png");
const FAVICON_32: &[u8] = include_bytes!("../assets/favicon-32.png");
const APPLE_TOUCH_ICON: &[u8] = include_bytes!("../assets/apple-touch-icon.png");
const LOGO_192: &[u8] = include_bytes!("../assets/logo-192.png");
const LOGO_512: &[u8] = include_bytes!("../assets/logo-512.png");
const MSTILE_150: &[u8] = include_bytes!("../assets/mstile-150x150.png");

/// Re-exported so handlers and the shell agree on the placeholder.
pub use shell::CSP_NONCE_PLACEHOLDER;

/// Add security headers to an HTML response.
///
/// CSP rationale (per directive):
/// - **script-src**: only nonced inline scripts run. `'unsafe-eval'` is gone
///   with Alpine.js — it needed `new Function()` to evaluate `x-show="a === b"`
///   expressions. Elm compiles to plain JavaScript and evaluates nothing, so
///   the directive is now as tight as it can be while still allowing the two
///   third-party SDKs below.
/// - **style-src**: keeps `'unsafe-inline'` for Elm's `style` attributes,
///   which a nonce can't cover.
/// - **script-src / connect-src / frame-src** allow `connect.facebook.net`
///   and `*.facebook.com` for WhatsApp Embedded Signup (the SDK aborts
///   silently when its login dialog and impression telemetry are blocked,
///   which is how the WhatsApp button once mysteriously "cancelled" in
///   production) and `checkout.razorpay.com` / `api.razorpay.com` for the
///   payment checkout.
///
/// `is_dev` mirrors `dev_bypass::active(env)`. When true, form-action allows
/// localhost + 127.0.0.1 and img-src allows `blob:`.
fn add_security_headers(resp: &mut Response, nonce: &str, is_dev: bool) -> Result<()> {
    let headers = resp.headers_mut();
    headers.set("X-Frame-Options", "DENY")?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("Referrer-Policy", "strict-origin-when-cross-origin")?;
    let (img_extra, form_action_extra) = if is_dev {
        (
            " blob:",
            " http://localhost:* http://127.0.0.1:* https://localhost:* https://127.0.0.1:*",
        )
    } else {
        ("", "")
    };
    let csp = format!(
        "default-src 'self'; \
         script-src 'self' 'nonce-{nonce}' https://checkout.razorpay.com https://connect.facebook.net; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: https:{img_extra}; \
         connect-src 'self' https://www.facebook.com https://graph.facebook.com https://*.facebook.com https://api.razorpay.com; \
         frame-src https://www.facebook.com https://*.facebook.com https://api.razorpay.com https://checkout.razorpay.com; \
         base-uri 'self'; \
         form-action 'self' https://accounts.google.com{form_action_extra}; \
         object-src 'none'"
    );
    headers.set("Content-Security-Policy", &csp)?;
    Ok(())
}

fn serve_text(body: &str, content_type: &str) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Content-Type", content_type)?;
    headers.set("Cache-Control", "public, max-age=31536000")?;
    Ok(Response::ok(body)?.with_headers(headers))
}

fn serve_png(body: &[u8]) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Content-Type", "image/png")?;
    headers.set("Cache-Control", "public, max-age=31536000")?;
    Ok(Response::from_bytes(body.to_vec())?.with_headers(headers))
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    // Captured before `env` is consumed so add_security_headers can pick the
    // dev vs. prod CSP profile.
    let is_dev = dev_bypass::active(&env);
    let mut resp = handle_request(req, env).await?;
    let is_html = resp
        .headers()
        .get("Content-Type")
        .ok()
        .flatten()
        .is_some_and(|ct| ct.contains("text/html"));
    if !is_html {
        return Ok(resp);
    }
    // Generate a per-request nonce, swap every `__CSP_NONCE__` placeholder for
    // it, and stamp the matching value into the CSP header. In practice the
    // only HTML we emit is the SPA shell, but keeping this at the wrapper
    // means a nonced response can't be produced without its header.
    let status = resp.status_code();
    let headers = resp.headers().clone();
    let body = resp.text().await?;
    let nonce = helpers::generate_token()?;
    let body = body.replace(CSP_NONCE_PLACEHOLDER, &nonce);
    headers.set("Content-Length", &body.len().to_string())?;
    let mut new_resp = Response::ok(body)?
        .with_status(status)
        .with_headers(headers);
    add_security_headers(&mut new_resp, &nonce, is_dev)?;
    Ok(new_resp)
}

/// Serve the SPA shell.
///
/// Answers with the requested status so a route the frontend treats as "not
/// found" still returns 404 to a crawler or a monitor, rather than a 200 with
/// an error rendered inside it.
fn serve_shell(status: u16) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Content-Type", "text/html; charset=utf-8")?;
    // The shell carries a per-response nonce, so it must never be cached.
    // `/app.js` is the immutable part and is cached by the asset handler.
    headers.set("Cache-Control", "no-store")?;
    Ok(Response::ok(shell::html())?
        .with_status(status)
        .with_headers(headers))
}

/// Routes the Elm app owns. Anything here is served the shell; the app reads
/// the URL and renders the matching page.
///
/// Listed explicitly rather than falling through on everything so a typo'd
/// path still 404s instead of silently rendering the app's own not-found page
/// under a 200.
fn is_app_route(path: &str) -> bool {
    matches!(
        path,
        "/" | "/index.html"
            | "/pricing"
            | "/features"
            | "/terms"
            | "/privacy"
            | "/login"
            | "/wizard"
            | "/dashboard"
            | "/manage"
    ) || path.starts_with("/wizard/")
        || path.starts_with("/dashboard/")
        || path.starts_with("/manage/")
}

async fn handle_request(req: Request, env: Env) -> Result<Response> {
    let url = req.url()?;
    let path = url.path();
    let method = req.method();

    // PUBLIC_BASE_URL is required on every response path: OAuth redirect URIs
    // and handoff emails all need an absolute URL outside a request context.
    // Refuse to serve anything without it rather than silently degrade.
    let public_base = env
        .var("PUBLIC_BASE_URL")
        .map(|v| v.to_string())
        .unwrap_or_default();
    if public_base.is_empty() {
        console_log!("PUBLIC_BASE_URL is not set");
        return Response::error(
            "Service unavailable: PUBLIC_BASE_URL is not configured",
            503,
        );
    }

    let host = url.host_str().unwrap_or("");
    let req_base = format!("{}://{}", url.scheme(), host);

    // Embedded static assets. These are `include_bytes!`d rather than served
    // from `public/` because they're referenced from the manifest and from
    // emails, so a missing asset should be a build failure, not a 404.
    match path {
        "/robots.txt" => {
            let body = format!(
                "User-agent: *\nAllow: /\nAllow: /features\nAllow: /pricing\nAllow: /terms\nAllow: /privacy\nDisallow: /api\nDisallow: /dashboard\nDisallow: /wizard\nDisallow: /manage\nDisallow: /auth\nDisallow: /webhook\nDisallow: /whatsapp\n\nSitemap: {req_base}/sitemap.txt\n"
            );
            return serve_text(&body, "text/plain");
        }
        "/sitemap.txt" => {
            let body = format!(
                "{req_base}/\n{req_base}/features\n{req_base}/pricing\n{req_base}/terms\n{req_base}/privacy\n"
            );
            return serve_text(&body, "text/plain");
        }
        "/logo.svg" => return serve_text(LOGO_SVG, "image/svg+xml"),
        "/site.webmanifest" => return serve_text(WEBMANIFEST, "application/manifest+json"),
        "/browserconfig.xml" => return serve_text(BROWSERCONFIG, "application/xml"),
        "/favicon-16.png" => return serve_png(FAVICON_16),
        "/favicon-32.png" => return serve_png(FAVICON_32),
        "/apple-touch-icon.png" => return serve_png(APPLE_TOUCH_ICON),
        "/logo-192.png" => return serve_png(LOGO_192),
        "/logo-512.png" => return serve_png(LOGO_512),
        "/mstile-150x150.png" => return serve_png(MSTILE_150),
        "/health" => return handlers::health::handle_health(req, env).await,
        _ => {}
    }

    // The JSON API. Checked before anything else that could shadow it, and
    // before the essentials gate below: an unconfigured deploy should give the
    // frontend a JSON error it can render, not an HTML maintenance page it
    // would fail to decode.
    if path == "/api" || path.starts_with("/api/") {
        return api::handle(req, env, path).await;
    }

    // Webhooks. Counterparties retry on 5xx, so these must answer even while
    // the rest of the deploy is misconfigured.
    if path == "/webhook/razorpay" && method == Method::Post {
        return billing::webhook::handle_razorpay_webhook(req, env).await;
    }
    if path.starts_with("/webhook/") {
        return handlers::handle_webhook(req, env, path, method).await;
    }

    // Facebook's data-deletion callback: a fixed contract with Meta.
    if path == "/data-deletion" {
        return handlers::handle_data_deletion(req, env, method).await;
    }

    // Routes that can't function without secrets get a maintenance response
    // when essentials are missing. Marketing routes, webhooks, /health and
    // /api/manage (Cloudflare Access protected: how the operator recovers)
    // are unaffected.
    let needs_essentials = path.starts_with("/auth") || path.starts_with("/whatsapp/signup");
    if needs_essentials && !handlers::health::essentials_missing(&env).is_empty() {
        let headers = Headers::new();
        headers.set("Retry-After", "60")?;
        headers.set("Cache-Control", "no-store")?;
        return Ok(Response::error("Temporarily unavailable", 503)?.with_headers(headers));
    }

    // Google OAuth: browser redirects, not API calls.
    if path.starts_with("/auth/") {
        return handlers::handle_auth(req, env, path, method).await;
    }

    // WhatsApp Embedded Signup callback: Meta redirects the browser here.
    if path.starts_with("/whatsapp/signup/") {
        return handlers::handle_whatsapp_signup(req, env, path, method).await;
    }

    // Everything the frontend owns.
    if is_app_route(path) {
        return serve_shell(200);
    }

    // Unknown path. Still the shell, so the user gets the app's own
    // not-found page with working navigation rather than a bare string — but
    // under a 404 so crawlers and monitors see the truth.
    serve_shell(404)
}

#[event(scheduled)]
async fn scheduled_handler(event: ScheduledEvent, env: Env, ctx: ScheduleContext) {
    scheduled::handle_scheduled(event, env, ctx).await;
}

/// Persona safety classifier consumer. See `src/safety_queue.rs`.
#[event(queue)]
async fn queue_handler(
    batch: MessageBatch<safety_queue::SafetyJob>,
    env: Env,
    _ctx: Context,
) -> Result<()> {
    safety_queue::handle_batch(batch, env).await
}
