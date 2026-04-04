use worker::*;

mod ai;
mod crypto;
mod handlers;
mod helpers;
mod instagram;
mod landing;
mod scheduled;
mod storage;
mod templates;
mod types;
mod whatsapp;

pub use types::*;

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

    let url = req.url()?;
    let path = url.path();
    let method = req.method();

    // Static assets
    match path {
        "/logo.svg" => return serve_text(LOGO_SVG, "image/svg+xml"),
        "/site.webmanifest" => return serve_text(WEBMANIFEST, "application/manifest+json"),
        "/browserconfig.xml" => return serve_text(BROWSERCONFIG, "application/xml"),
        "/favicon-16.png" => return serve_png(FAVICON_16),
        "/favicon-32.png" => return serve_png(FAVICON_32),
        "/apple-touch-icon.png" => return serve_png(APPLE_TOUCH_ICON),
        "/logo-192.png" => return serve_png(LOGO_192),
        "/logo-512.png" => return serve_png(LOGO_512),
        "/mstile-150x150.png" => return serve_png(MSTILE_150),
        "/health" => return Response::ok("OK"),
        _ => {}
    }

    // Terms of Service
    if path == "/terms" {
        let headers = Headers::new();
        headers.set("Content-Type", "text/html; charset=utf-8")?;
        headers.set("Cache-Control", "public, max-age=3600")?;
        return Ok(Response::ok(landing::terms_of_service_html())?.with_headers(headers));
    }

    // Privacy Policy
    if path == "/privacy" {
        let headers = Headers::new();
        headers.set("Content-Type", "text/html; charset=utf-8")?;
        headers.set("Cache-Control", "public, max-age=3600")?;
        return Ok(Response::ok(landing::privacy_policy_html())?.with_headers(headers));
    }

    // Data deletion callback (Facebook requirement)
    if path == "/data-deletion" {
        return handlers::handle_data_deletion(req, env, method).await;
    }

    // Auth routes (login, callback, logout)
    if path.starts_with("/auth/") {
        return handlers::handle_auth(req, env, path, method).await;
    }

    // WhatsApp Embedded Signup callback
    if path.starts_with("/whatsapp/signup/") {
        return handlers::handle_whatsapp_signup(req, env, path, method).await;
    }

    // Admin routes (session-protected)
    if path.starts_with("/admin") {
        return handlers::handle_admin(req, env, path, method).await;
    }

    // Lead capture form routes (public)
    if path.starts_with("/lead/") {
        return handlers::handle_lead_form(req, env, path, method).await;
    }

    // Instagram OAuth routes
    if path.starts_with("/instagram/") {
        return handlers::handle_instagram(req, env, path, method).await;
    }

    // Webhook routes (WhatsApp + Instagram incoming messages)
    if path.starts_with("/webhook/") {
        return handlers::handle_webhook(req, env, path, method).await;
    }

    // Landing page
    if path == "/" || path == "/index.html" {
        let headers = Headers::new();
        headers.set("Content-Type", "text/html; charset=utf-8")?;
        headers.set("Cache-Control", "public, max-age=3600")?;
        return Ok(Response::ok(landing::landing_page_html())?.with_headers(headers));
    }

    Response::error("Not Found", 404)
}

#[event(scheduled)]
async fn scheduled_handler(event: ScheduledEvent, env: Env, ctx: ScheduleContext) {
    scheduled::handle_scheduled(event, env, ctx).await;
}
