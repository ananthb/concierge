//! Google OAuth sign-in.
//!
//! Stays outside the JSON API because OAuth is a browser redirect dance:
//! the frontend links to `/auth/login`, Google bounces back to
//! `/auth/callback`, and we set a session cookie and redirect to `/`. The
//! Elm app then calls `/api/bootstrap` and finds itself signed in.
//!
//! There is no login *page* here any more. The Elm app renders that and
//! links to `/auth/login`, which now redirects straight to Google's consent
//! screen instead of rendering a provider chooser.
//!
//! Facebook Login as a sign-in method went with the Instagram channel. Meta
//! identity still arrives through WhatsApp Embedded Signup, which does its
//! own token exchange in [`super::whatsapp_signup`] and can create a tenant
//! the same way this does.

use serde::Deserialize;
use worker::*;

use super::get_base_url;
use crate::helpers::*;
use crate::storage::*;
use crate::types::*;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

const SESSION_TTL_SECONDS: u64 = 7 * 24 * 60 * 60; // 7 days

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    email: String,
    name: Option<String>,
}

/// Handle auth routes (`/auth/*`).
pub async fn handle_auth(req: Request, env: Env, path: &str, method: Method) -> Result<Response> {
    let base_url = get_base_url(&req);

    match (method, path) {
        // Straight to Google. Already signed in? Send them to the app
        // rather than minting a second session.
        (Method::Get, "/auth/login") => {
            let kv = env.kv("KV")?;
            if resolve_tenant_id(&req, &kv).await.is_some() {
                return redirect("/");
            }

            let client_id = env
                .secret("GOOGLE_OAUTH_CLIENT_ID")
                .map(|s| s.to_string())
                .unwrap_or_default();
            if client_id.is_empty() {
                return Response::error("Sign-in is not configured on this deploy.", 503);
            }
            let redirect_uri = format!("{base_url}/auth/callback");
            let consent = format!(
                "{GOOGLE_AUTH_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=online&prompt=select_account",
                urlencoding::encode(&client_id),
                urlencoding::encode(&redirect_uri),
                urlencoding::encode("openid email profile"),
            );
            redirect(&consent)
        }

        (Method::Get, "/auth/callback") => {
            let url = req.url()?;
            let query: std::collections::HashMap<_, _> = url.query_pairs().collect();

            let code = match query.get("code") {
                Some(c) => c.to_string(),
                None => {
                    // The user declined, or Google refused. Back to the app,
                    // which renders the login screen again.
                    let reason = query
                        .get("error")
                        .map(|e| e.to_string())
                        .unwrap_or_default();
                    console_log!("OAuth callback without a code: {reason}");
                    return redirect("/?auth=failed");
                }
            };

            let client_id = env.secret("GOOGLE_OAUTH_CLIENT_ID")?.to_string();
            let client_secret = env.secret("GOOGLE_OAUTH_CLIENT_SECRET")?.to_string();
            let redirect_uri = format!("{base_url}/auth/callback");

            let token_body = format!(
                "code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code",
                urlencoding::encode(&code),
                urlencoding::encode(&client_id),
                urlencoding::encode(&client_secret),
                urlencoding::encode(&redirect_uri),
            );

            let headers = Headers::new();
            headers.set("Content-Type", "application/x-www-form-urlencoded")?;
            let mut init = RequestInit::new();
            init.with_method(Method::Post)
                .with_headers(headers)
                .with_body(Some(wasm_bindgen::JsValue::from_str(&token_body)));

            let token_req = Request::new_with_init(GOOGLE_TOKEN_URL, &init)?;
            let mut token_resp = Fetch::Request(token_req).send().await?;
            let token_text = token_resp.text().await?;

            if token_resp.status_code() != 200 {
                console_log!("Google token exchange failed: {token_text}");
                return redirect("/?auth=failed");
            }

            let tokens: TokenResponse = serde_json::from_str(&token_text)
                .map_err(|e| Error::from(format!("Failed to parse token response: {e}")))?;

            let headers = Headers::new();
            headers.set("Authorization", &format!("Bearer {}", tokens.access_token))?;
            let mut init = RequestInit::new();
            init.with_method(Method::Get).with_headers(headers);

            let userinfo_req = Request::new_with_init(GOOGLE_USERINFO_URL, &init)?;
            let mut userinfo_resp = Fetch::Request(userinfo_req).send().await?;
            let userinfo_text = userinfo_resp.text().await?;

            if userinfo_resp.status_code() != 200 {
                console_log!("Google userinfo failed: {userinfo_text}");
                return redirect("/?auth=failed");
            }

            let user: GoogleUserInfo = serde_json::from_str(&userinfo_text)
                .map_err(|e| Error::from(format!("Failed to parse user info: {e}")))?;

            let kv = env.kv("KV")?;
            let db = env.d1("DB")?;

            // Find or create the tenant. New tenants take their locale from
            // Accept-Language > cf-ipcountry > en-IN; currency follows.
            let tenant = match get_tenant_by_email(&db, &user.email).await? {
                Some(t) => t,
                None => {
                    let signup_locale = crate::locale::Locale::from_request(&req);
                    let now = now_iso();
                    let tenant = Tenant {
                        id: generate_id(),
                        email: user.email.clone(),
                        name: user.name,
                        facebook_id: None,
                        plan: crate::types::Plan::Paid,
                        locale: signup_locale.langid.to_string(),
                        currency: signup_locale.currency,
                        verified_at: None,
                        created_at: now.clone(),
                        updated_at: now,
                    };
                    save_tenant(&db, &tenant).await?;
                    tenant
                }
            };

            create_session_and_redirect(&req, &kv, &tenant.id).await
        }

        // Dev-only login shortcut. Mints a session for an arbitrary email
        // without a round-trip to Google, which can't reach localhost.
        // Gated on `dev_bypass::active`; production sets CF_ACCESS_AUD, so
        // this 404s there.
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

            let kv = env.kv("KV")?;
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
                        plan: crate::types::Plan::Paid,
                        locale: signup_locale.langid.to_string(),
                        currency: signup_locale.currency,
                        // Pre-verified: there's no Razorpay checkout to
                        // complete against a local dev deploy.
                        verified_at: Some(now.clone()),
                        created_at: now.clone(),
                        updated_at: now,
                    };
                    save_tenant(&db, &t).await?;
                    t
                }
            };

            create_session_and_redirect(&req, &kv, &tenant.id).await
        }

        _ => Response::error("Not Found", 404),
    }
}

fn redirect(location: &str) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Location", location)?;
    headers.set("Cache-Control", "no-store")?;
    Ok(Response::empty()?.with_status(302).with_headers(headers))
}

/// Mint a session and bounce to the app root.
///
/// Lands on `/` rather than `/dashboard`: the Elm app reads
/// `session.destination` from `/api/bootstrap` and routes to the wizard or
/// the dashboard itself, so the correct landing spot is decided in one place
/// instead of being duplicated here.
///
/// No CSRF cookie any more — the API authenticates state-changing calls with
/// the `X-Concierge-Request` header instead (see [`crate::api`]).
pub(super) async fn create_session_and_redirect(
    req: &Request,
    kv: &kv::KvStore,
    tenant_id: &str,
) -> Result<Response> {
    let session_token = generate_token()?;
    save_session(kv, &session_token, tenant_id, SESSION_TTL_SECONDS).await?;

    // Drop `Secure` on http origins so plain `wrangler dev` still
    // authenticates on browsers that don't treat localhost as secure.
    let is_https = req
        .url()
        .ok()
        .map(|u| u.scheme() == "https")
        .unwrap_or(true);
    let secure_attr = if is_https { "; Secure" } else { "" };

    let headers = Headers::new();
    headers.set("Location", "/")?;
    headers.set("Cache-Control", "no-store")?;
    headers.set(
        "Set-Cookie",
        &format!(
            "session={token}; Path=/; HttpOnly{secure}; SameSite=Lax; Max-Age={ttl}",
            token = session_token,
            secure = secure_attr,
            ttl = SESSION_TTL_SECONDS,
        ),
    )?;
    Ok(Response::empty()?.with_status(302).with_headers(headers))
}

/// Extract a named cookie from a request.
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

/// Extract the session cookie from a request.
pub fn get_session_cookie(req: &Request) -> Option<String> {
    get_cookie(req, "session")
}

/// Resolve `tenant_id` from the session cookie. `None` when signed out.
pub async fn resolve_tenant_id(req: &Request, kv: &kv::KvStore) -> Option<String> {
    let token = get_session_cookie(req)?;
    get_session(kv, &token).await.ok()?
}
