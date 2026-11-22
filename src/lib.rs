//! # Concierge
//!
//! WhatsApp auto-reply platform for small businesses.

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
mod pipeline;
mod prompt;
mod risk;
mod safety;
mod safety_queue;
mod shell;
mod storage;
mod types;
mod whatsapp;

pub use durable_objects::ReplyBufferDO;
pub use types::*;

/// Meta Graph API version used across all Facebook/WhatsApp API calls.
pub const META_API_VERSION: &str = "v21.0";

const LOGO_SVG: &str = include_str!("../assets/logo.svg");
const WEBMANIFEST: &str = include_str!("../assets/site.webmanifest");
const BROWSERCONFIG: &str = include_str!("../assets/browserconfig.xml");
const FAVICON_16: &[u8] = include_bytes!("../assets/favicon-16.png");
const FAVICON_32: &[u8] = include_bytes!("../assets/favicon-32.png");
const APPLE_TOUCH_ICON: &[u8] = include_bytes!("../assets/apple-touch-icon.png");
const LOGO_192: &[u8] = include_bytes!("../assets/logo-192.png");
const LOGO_512: &[u8] = include_bytes!("../assets/logo-512.png");
const MSTILE_150: &[u8] = include_bytes!("../assets/mstile-150x150.png");

fn add_security_headers(resp: &mut Response, is_dev: bool) -> Result<()> {
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
         script-src 'self' https://checkout.razorpay.com https://connect.facebook.net; \
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
    let is_dev = dev_bypass::active(&env);
    let mut resp = handle_request(req, env).await?;
    let is_html = resp
        .headers()
        .get("Content-Type")
        .ok()
        .flatten()
        .is_some_and(|ct| ct.contains("text/html"));
    if is_html {
        add_security_headers(&mut resp, is_dev)?;
    }
    Ok(resp)
}

fn serve_shell(status: u16) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Content-Type", "text/html; charset=utf-8")?;
    headers.set("Cache-Control", "no-store")?;
    Ok(Response::ok(shell::html())?
        .with_status(status)
        .with_headers(headers))
}

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
            | "/dashboard/channels"
            | "/dashboard/billing"
            | "/dashboard/settings"
    ) || path.starts_with("/wizard/")
}

async fn handle_request(req: Request, env: Env) -> Result<Response> {
    let url = req.url()?;
    let path = url.path();
    let method = req.method();

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

    match path {
        "/robots.txt" => {
            let body = format!(
                "User-agent: *\nAllow: /\nAllow: /features\nAllow: /pricing\nAllow: /terms\nAllow: /privacy\nDisallow: /api\nDisallow: /dashboard\nDisallow: /wizard\nDisallow: /auth\nDisallow: /webhook\nDisallow: /whatsapp\n\nSitemap: {req_base}/sitemap.txt\n"
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

    if path == "/api" || path.starts_with("/api/") {
        return api::handle(req, env, path).await;
    }

    if path == "/webhook/razorpay" && method == Method::Post {
        return billing::webhook::handle_razorpay_webhook(req, env).await;
    }
    if path.starts_with("/webhook/") {
        return handlers::handle_webhook(req, env, path, method).await;
    }

    if path == "/data-deletion" {
        return handlers::handle_data_deletion(req, env, method).await;
    }

    let needs_essentials = path.starts_with("/auth") || path.starts_with("/whatsapp/signup");
    if needs_essentials && !handlers::health::essentials_missing(&env).is_empty() {
        let headers = Headers::new();
        headers.set("Retry-After", "60")?;
        headers.set("Cache-Control", "no-store")?;
        return Ok(Response::error("Temporarily unavailable", 503)?.with_headers(headers));
    }

    if path.starts_with("/auth/") {
        return handlers::handle_auth(req, env, path, method).await;
    }

    if path.starts_with("/whatsapp/signup/") {
        return handlers::handle_whatsapp_signup(req, env, path, method).await;
    }

    if is_app_route(path) {
        return serve_shell(200);
    }

    serve_shell(404)
}

/// Content safety classifier consumer. See `src/safety_queue.rs`.
#[event(queue)]
async fn queue_handler(
    batch: MessageBatch<safety_queue::SafetyJob>,
    env: Env,
    _ctx: Context,
) -> Result<()> {
    safety_queue::handle_batch(batch, env).await
}
