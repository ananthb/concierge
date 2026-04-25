//! # Concierge
//!
//! Messaging automation for small businesses: WhatsApp auto-replies,
//! Instagram DM auto-replies, and embeddable lead capture forms.
//!
//! This is a Cloudflare Worker built with Rust + WebAssembly. It handles:
//!
//! - **WhatsApp webhooks**: incoming messages trigger auto-replies (static or AI)
//! - **Instagram DM webhooks**: same auto-reply pattern via Facebook Pages API
//! - **Lead capture forms**: embeddable phone number forms that send WhatsApp messages
//! - **Admin dashboard**: HTMX-powered UI for managing accounts and forms
//! - **OAuth**: Google and Facebook sign-in with multi-provider account linking
//!
//! ## Architecture
//!
//! - `types`: Core data structures (Tenant, WhatsAppAccount, InstagramAccount, LeadCaptureForm)
//! - `storage`: Cloudflare KV and D1 operations
//! - `ai`: Cloudflare Workers AI integration for auto-reply generation
//! - `whatsapp`: Meta Graph API client for sending WhatsApp messages
//! - `instagram`: Facebook Login OAuth and Instagram DM sending
//! - `crypto`: AES-256-GCM encryption and HMAC-SHA256 verification
//! - `helpers`: ID generation, HTML escaping, CORS, template interpolation

use wasm_bindgen::prelude::*;
use worker::*;

mod ai;
mod billing;
mod channel;
mod crypto;
mod discord;
mod durable_objects;
mod email;
mod handlers;
mod helpers;
mod instagram;
mod legal;
mod management;
mod pipeline;
mod scheduled;
mod storage;
mod templates;
mod types;
mod whatsapp;

pub use durable_objects::ReplyBufferDO;
pub use types::*;

/// Meta Graph API version used across all Facebook/WhatsApp/Instagram API calls.
pub const META_API_VERSION: &str = "v21.0";

// --- Email event handler via wasm_bindgen ---

#[wasm_bindgen]
extern "C" {
    pub type IncomingEmailMessage;

    #[wasm_bindgen(method, getter)]
    fn from(this: &IncomingEmailMessage) -> String;

    #[wasm_bindgen(method, getter)]
    fn to(this: &IncomingEmailMessage) -> String;

    /// `raw` is a ReadableStream of the message's RFC 2822 bytes: NOT a Promise.
    /// Treating it as a Promise (awaiting it) hangs forever.
    #[wasm_bindgen(method, getter)]
    fn raw(this: &IncomingEmailMessage) -> web_sys::ReadableStream;

    #[wasm_bindgen(method, js_name = "setReject")]
    fn set_reject(this: &IncomingEmailMessage, reason: &str);
}

#[wasm_bindgen]
pub async fn email(
    message: IncomingEmailMessage,
    env: JsValue,
    _ctx: JsValue,
) -> std::result::Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let from = message.from();
    let to = message.to();

    // `message.raw` is a ReadableStream. Wrap it in a Response to consume the
    // bytes: `Response#arrayBuffer()` reads the stream to completion.
    let raw_stream = message.raw();
    let response = web_sys::Response::new_with_opt_readable_stream(Some(&raw_stream))
        .map_err(|e| JsValue::from_str(&format!("Response from raw stream: {e:?}")))?;
    let buf_promise = response
        .array_buffer()
        .map_err(|e| JsValue::from_str(&format!("arrayBuffer: {e:?}")))?;
    let buf_value = wasm_bindgen_futures::JsFuture::from(buf_promise).await?;
    let uint8 = js_sys::Uint8Array::new(&buf_value);
    let mut raw_bytes = vec![0u8; uint8.length() as usize];
    uint8.copy_to(&mut raw_bytes);

    let worker_env: Env = env.into();

    let result = email::handler::handle_email(&from, &to, &raw_bytes, &worker_env)
        .await
        .map_err(|e| JsValue::from_str(&format!("Email handler error: {e}")))?;

    match result {
        email::handler::EmailResult::Send(outbound) => {
            email::send::send_outbound(&worker_env, &outbound)
                .await
                .map_err(|e| JsValue::from_str(&format!("send_outbound: {e}")))?;
        }
        email::handler::EmailResult::Reject(reason) => {
            message.set_reject(&reason);
        }
        email::handler::EmailResult::Drop => {
            // Do nothing: silently consume
        }
    }

    Ok(())
}

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

/// Add security headers to an HTML response.
fn add_security_headers(resp: &mut Response) -> Result<()> {
    let headers = resp.headers_mut();
    headers.set("X-Frame-Options", "DENY")?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("Referrer-Policy", "strict-origin-when-cross-origin")?;
    headers.set(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self' https://unpkg.com https://checkout.razorpay.com https://connect.facebook.net 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src https://fonts.gstatic.com; img-src 'self' data: https:; connect-src 'self'"
    )?;
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
    let mut resp = handle_request(req, env).await?;
    // Add security headers to all HTML responses
    if resp
        .headers()
        .get("Content-Type")
        .ok()
        .flatten()
        .map_or(false, |ct| ct.contains("text/html"))
    {
        add_security_headers(&mut resp)?;
    }
    Ok(resp)
}

async fn handle_request(req: Request, env: Env) -> Result<Response> {
    let url = req.url()?;
    let path = url.path();
    let method = req.method();

    // Redirect cncg.email (and subdomains) to the main site
    let host = url.host_str().unwrap_or("");
    let email_base = env
        .var("EMAIL_BASE_DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_default();
    if !email_base.is_empty() && (host == email_base || host.ends_with(&format!(".{email_base}"))) {
        let headers = Headers::new();
        headers.set("Location", "https://concierge.calculon.tech")?;
        return Ok(Response::empty()?.with_status(301).with_headers(headers));
    }

    // Static assets
    match path {
        "/robots.txt" => {
            return serve_text(
                "User-agent: *\nAllow: /\nAllow: /features\nAllow: /pricing\nAllow: /terms\nAllow: /privacy\nDisallow: /admin\nDisallow: /manage\nDisallow: /auth\nDisallow: /webhook\nDisallow: /discord\nDisallow: /instagram\nDisallow: /whatsapp\n\nSitemap: https://concierge.calculon.tech/sitemap.txt\n",
                "text/plain",
            );
        }
        "/sitemap.txt" => {
            return serve_text(
                "https://concierge.calculon.tech/\nhttps://concierge.calculon.tech/features\nhttps://concierge.calculon.tech/pricing\nhttps://concierge.calculon.tech/terms\nhttps://concierge.calculon.tech/privacy\nhttps://ananthb.github.io/concierge/\n",
                "text/plain",
            );
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

    // Routes that actually need secrets to function (login + the signed-in
    // app + the third-party install/auth callbacks) get a branded
    // maintenance page when essentials are missing. Static marketing pages
    // (/, /pricing, /features, /terms, /privacy), webhooks, /health, and
    // /manage/* (Cloudflare Access protected: how the operator recovers)
    // are unaffected.
    let needs_essentials = path.starts_with("/auth")
        || path.starts_with("/admin")
        || path.starts_with("/whatsapp/signup")
        || path.starts_with("/instagram/");
    if needs_essentials && !handlers::health::essentials_missing(&env).is_empty() {
        let mut resp = Response::from_html(templates::base::maintenance_html())?.with_status(503);
        let h = resp.headers_mut();
        h.set("Retry-After", "60")?;
        h.set("Cache-Control", "no-store")?;
        return Ok(resp);
    }

    // Terms of Service
    if path == "/terms" {
        return Response::from_html(legal::terms_of_service_html());
    }

    // Privacy Policy
    if path == "/privacy" {
        return Response::from_html(legal::privacy_policy_html());
    }

    // Marketing features overview
    if path == "/features" {
        return Response::from_html(templates::features::features_html());
    }

    // Pricing page. ?c=usd|inr overrides the geo-IP default so the toggle
    // buttons work.
    if path == "/pricing" {
        let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
        let currency = query.get("c").map(|s| s.to_string()).unwrap_or_else(|| {
            let country = req
                .headers()
                .get("cf-ipcountry")
                .ok()
                .flatten()
                .unwrap_or_default();
            if country == "IN" {
                "inr".into()
            } else {
                "usd".into()
            }
        });
        return Response::from_html(templates::onboarding::pricing_html(&currency));
    }

    // Data deletion callback (Facebook requirement)
    if path == "/data-deletion" {
        return handlers::handle_data_deletion(req, env, method).await;
    }

    // Notification-recipient verification: opened from an emailed link.
    if let Some(token) = path.strip_prefix("/email/verify/") {
        return handle_email_verify(env, token).await;
    }

    // Auth routes (login, callback, logout)
    if path.starts_with("/auth/") {
        return handlers::handle_auth(req, env, path, method).await;
    }

    // WhatsApp Embedded Signup callback
    if path.starts_with("/whatsapp/signup/") {
        return handlers::handle_whatsapp_signup(req, env, path, method).await;
    }

    // Management panel (Cloudflare Access protected)
    if path.starts_with("/manage") {
        return management::handle_management(req, env, path, method).await;
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

    // Discord interaction endpoint
    if path == "/discord/interactions" && method == Method::Post {
        return discord::handle_interaction(req, env).await;
    }

    // Discord webhook events (MESSAGE_CREATE, etc.): inbound channel for AI auto-reply.
    if path == "/discord/events" && method == Method::Post {
        return discord::events::handle_event(req, env).await;
    }

    // Razorpay payment webhook
    if path == "/webhook/razorpay" && method == Method::Post {
        return billing::webhook::handle_razorpay_webhook(req, env).await;
    }

    // Webhook routes (WhatsApp + Instagram incoming messages)
    if path.starts_with("/webhook/") {
        return handlers::handle_webhook(req, env, path, method).await;
    }

    // Landing → dashboard if already signed in, otherwise welcome page
    if path == "/" || path == "/index.html" {
        let kv = env.kv("KV")?;
        if handlers::auth::resolve_tenant_id(&req, &kv).await.is_some() {
            let headers = Headers::new();
            headers.set("Location", "/admin")?;
            return Ok(Response::empty()?.with_status(302).with_headers(headers));
        }
        return Response::from_html(templates::onboarding::welcome_html(""));
    }

    Response::error("Not Found", 404)
}

async fn handle_email_verify(env: Env, token: &str) -> Result<Response> {
    use templates::admin_email::email_verify_result_html;

    if token.is_empty() {
        return Response::from_html(email_verify_result_html(
            "That verification link is invalid.",
        ));
    }

    let kv = env.kv("KV")?;
    let payload = match storage::get_email_verification_token(&kv, token).await? {
        Some(p) => p,
        None => {
            return Response::from_html(email_verify_result_html(
                "This link has expired or was already used. If you still need to confirm, ask the account owner to add you again.",
            ))
        }
    };

    let mut addr =
        match storage::get_email_address(&kv, &payload.tenant_id, &payload.local_part).await? {
            Some(a) => a,
            None => {
                let _ = storage::delete_email_verification_token(&kv, token).await;
                return Response::from_html(email_verify_result_html(
                    "The address this notification was for is no longer active.",
                ));
            }
        };

    let now = helpers::now_iso();
    let mut found = false;
    for r in addr.notification_recipients.iter_mut() {
        if r.id == payload.recipient_id {
            r.status = types::RecipientStatus::Verified;
            r.verified_at = Some(now.clone());
            found = true;
            break;
        }
    }
    let _ = storage::delete_email_verification_token(&kv, token).await;

    if !found {
        return Response::from_html(email_verify_result_html(
            "This recipient has been removed. No further action needed.",
        ));
    }

    addr.updated_at = now;
    storage::save_email_address(&kv, &payload.tenant_id, &addr).await?;
    Response::from_html(email_verify_result_html(
        "You're verified: replies sent from this Concierge address will now copy you.",
    ))
}

#[event(scheduled)]
async fn scheduled_handler(event: ScheduledEvent, env: Env, ctx: ScheduleContext) {
    scheduled::handle_scheduled(event, env, ctx).await;
}
