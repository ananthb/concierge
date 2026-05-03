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
mod approval;
mod approvals;
mod billing;
mod channel;
mod crypto;
mod discord;
mod durable_objects;
mod email;
mod escalations;
mod handlers;
mod helpers;
mod i18n;
mod instagram;
mod legal;
mod locale;
mod management;
mod personas;
mod pipeline;
mod prompt;
mod safety;
mod safety_queue;
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

/// Placeholder string emitted by templates wherever they need a CSP nonce
/// (`<script nonce="__CSP_NONCE__">`, `<style nonce="__CSP_NONCE__">`). The
/// fetch wrapper swaps it for a per-response random nonce and writes the
/// matching CSP header. Centralizing it here means individual handlers
/// don't have to thread a nonce argument through every render call.
pub const CSP_NONCE_PLACEHOLDER: &str = "__CSP_NONCE__";

/// Add security headers to an HTML response.
///
/// CSP rationale (per directive):
/// - **script-src**: only nonced inline scripts run. We keep `'unsafe-eval'`
///   because Alpine.js parses `x-show="a === b"`-style expressions via
///   `new Function()`; switching to the Alpine CSP build would force us to
///   refactor every Alpine usage to property-access only.
/// - **style-src**: keeps `'unsafe-inline'` because we have many `style="…"`
///   attrs; the per-script nonce alone wouldn't cover them.
/// - **connect-src**: FB Embedded Signup posts to `*.facebook.com` (login
///   dialog + impression telemetry; the SDK silently aborts when these
///   are blocked, which is how the WhatsApp button mysteriously "cancelled"
///   on production); Razorpay's verify call goes to `api.razorpay.com`.
/// - **frame-src**: FB login popup + Razorpay checkout iframe.
fn add_security_headers(resp: &mut Response, nonce: &str) -> Result<()> {
    let headers = resp.headers_mut();
    headers.set("X-Frame-Options", "DENY")?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("Referrer-Policy", "strict-origin-when-cross-origin")?;
    let csp = format!(
        "default-src 'self'; \
         script-src 'self' 'nonce-{nonce}' 'unsafe-eval' https://unpkg.com https://checkout.razorpay.com https://connect.facebook.net; \
         style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
         font-src https://fonts.gstatic.com; \
         img-src 'self' data: https:; \
         connect-src 'self' https://www.facebook.com https://graph.facebook.com https://*.facebook.com https://api.razorpay.com; \
         frame-src https://www.facebook.com https://*.facebook.com https://api.razorpay.com https://checkout.razorpay.com; \
         base-uri 'self'; \
         form-action 'self' https://www.facebook.com https://accounts.google.com; \
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
    let mut resp = handle_request(req, env).await?;
    let is_html = resp
        .headers()
        .get("Content-Type")
        .ok()
        .flatten()
        .map_or(false, |ct| ct.contains("text/html"));
    if !is_html {
        return Ok(resp);
    }
    // Generate a per-request nonce, swap every `__CSP_NONCE__` placeholder
    // for it in the rendered body, and stamp the matching value into the
    // CSP header. Templates emit the placeholder on every inline `<script>`
    // and `<style>` tag they produce; nonced tags then satisfy a strict
    // `script-src 'nonce-…'` (no `'unsafe-inline'`).
    let status = resp.status_code();
    let mut headers = resp.headers().clone();
    let body = resp.text().await?;
    let nonce = helpers::generate_token()?;
    let body = body.replace(CSP_NONCE_PLACEHOLDER, &nonce);
    headers.set("Content-Length", &body.len().to_string())?;
    let mut new_resp = Response::ok(body)?
        .with_status(status)
        .with_headers(headers);
    add_security_headers(&mut new_resp, &nonce)?;
    Ok(new_resp)
}

async fn handle_request(req: Request, env: Env) -> Result<Response> {
    let url = req.url()?;
    let path = url.path();
    let method = req.method();

    // PUBLIC_BASE_URL is required for every response path: it's used in
    // approval-digest emails, the email-domain landing page, and anywhere
    // the worker has to emit an absolute URL outside of a request context.
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

    // Render the email-domain landing page when the request comes in on
    // EMAIL_DOMAIN (or any subdomain). These visitors landed in a browser
    // by accident and we want to nudge them back to the main site.
    let host = url.host_str().unwrap_or("");
    let email_base = env
        .var("EMAIL_DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_default();
    if !email_base.is_empty() && (host == email_base || host.ends_with(&format!(".{email_base}"))) {
        return Response::from_html(templates::email_landing::email_landing_html(&public_base));
    }

    // Static assets
    let req_base = format!("{}://{}", url.scheme(), host);
    match path {
        "/robots.txt" => {
            let body = format!(
                "User-agent: *\nAllow: /\nAllow: /features\nAllow: /pricing\nAllow: /terms\nAllow: /privacy\nDisallow: /admin\nDisallow: /manage\nDisallow: /auth\nDisallow: /webhook\nDisallow: /discord\nDisallow: /instagram\nDisallow: /whatsapp\n\nSitemap: {req_base}/sitemap.txt\n"
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
    let request_locale = locale::Locale::from_request(&req);
    if needs_essentials && !handlers::health::essentials_missing(&env).is_empty() {
        let mut resp = Response::from_html(templates::base::maintenance_html(&request_locale))?
            .with_status(503);
        let h = resp.headers_mut();
        h.set("Retry-After", "60")?;
        h.set("Cache-Control", "no-store")?;
        return Ok(resp);
    }

    // Terms of Service
    if path == "/terms" {
        return Response::from_html(legal::terms_of_service_html(&request_locale));
    }

    // Privacy Policy
    if path == "/privacy" {
        return Response::from_html(legal::privacy_policy_html(&request_locale));
    }

    // Marketing features overview
    if path == "/features" {
        let db = env.d1("DB")?;
        let cfg = storage::get_pricing(&db).await;
        return Response::from_html(templates::features::features_html(&request_locale, &cfg));
    }

    // Pricing page. ?c=usd|inr overrides the geo-IP default so the toggle
    // buttons work.
    if path == "/pricing" {
        let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
        let currency_str = query.get("c").map(|s| s.to_string()).unwrap_or_else(|| {
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

        let db = env.d1("DB")?;
        let cfg = storage::get_pricing(&db).await;
        return Response::from_html(templates::onboarding::pricing_html(
            &currency_str,
            &request_locale,
            &cfg,
        ));
    }

    // Data deletion callback (Facebook requirement)
    if path == "/data-deletion" {
        return handlers::handle_data_deletion(req, env, method).await;
    }

    // Notification-recipient verification: opened from an emailed link.
    if let Some(token) = path.strip_prefix("/email/verify/") {
        return handle_email_verify(env, token, &request_locale).await;
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

    // Public live-demo chat (anyone can hit this from the welcome page).
    if path == "/demo/chat" && method == Method::Post {
        return handlers::handle_demo_chat(req, env).await;
    }
    // Demo persona picker. D1 catalog filtered to Approved.
    if path == "/demo/personas" && method == Method::Get {
        return handlers::handle_demo_personas(req, env).await;
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
        let locale = locale::Locale::from_request(&req);
        return Response::from_html(templates::onboarding::welcome_html("", &locale));
    }

    Response::error("Not Found", 404)
}

async fn handle_email_verify(env: Env, token: &str, locale: &locale::Locale) -> Result<Response> {
    use templates::admin_email::email_verify_result_html;

    if token.is_empty() {
        return Response::from_html(email_verify_result_html(
            "That verification link is invalid.",
            locale,
        ));
    }

    let kv = env.kv("KV")?;
    let payload = match storage::get_email_verification_token(&kv, token).await? {
        Some(p) => p,
        None => {
            return Response::from_html(email_verify_result_html(
                "This link has expired or was already used. If you still need to confirm, ask the account owner to add you again.",
                locale,
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
                    locale,
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
            locale,
        ));
    }

    addr.updated_at = now;
    storage::save_email_address(&kv, &payload.tenant_id, &addr).await?;
    Response::from_html(email_verify_result_html(
        "You're verified: replies sent from this Concierge address will now copy you.",
        locale,
    ))
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
