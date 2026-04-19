//! Authentication handlers - Google OAuth + Facebook Login

use serde::Deserialize;
use worker::*;

use super::get_base_url;
use crate::helpers::*;
use crate::storage::*;
use crate::templates::auth_login_html;
use crate::types::*;

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
const GRAPH_API_BASE: &str = "https://graph.facebook.com";

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

/// Handle auth routes (/auth/*)
pub async fn handle_auth(req: Request, env: Env, path: &str, method: Method) -> Result<Response> {
    let base_url = get_base_url(&req);

    match (method, path) {
        (Method::Get, "/auth/login") => {
            let google_client_id = env
                .secret("GOOGLE_OAUTH_CLIENT_ID")
                .map(|s| s.to_string())
                .unwrap_or_default();
            let meta_app_id = env
                .secret("META_APP_ID")
                .map(|s| s.to_string())
                .unwrap_or_default();

            let last_provider = get_cookie(&req, "last_provider");
            let html = auth_login_html(
                &base_url,
                &google_client_id,
                &meta_app_id,
                last_provider.as_deref(),
            );

            // Capture biz name from query param (set during onboarding start)
            let mut resp = Response::from_html(html)?;
            let url = req.url()?;
            if let Some(biz) = url
                .query_pairs()
                .find(|(k, _)| k == "biz")
                .map(|(_, v)| v.to_string())
            {
                if !biz.is_empty() {
                    resp.headers_mut().append(
                        "Set-Cookie",
                        &format!("onboarding_biz={}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=3600",
                            urlencoding::encode(&biz)),
                    )?;
                }
            }
            Ok(resp)
        }

        // Google OAuth callback
        (Method::Get, "/auth/callback") => {
            let url = req.url()?;
            let query: std::collections::HashMap<_, _> = url.query_pairs().collect();

            let code = match query.get("code") {
                Some(c) => c.to_string(),
                None => {
                    let error = query
                        .get("error")
                        .map(|e| e.to_string())
                        .unwrap_or_default();
                    return Response::error(format!("OAuth error: {}", error), 400);
                }
            };

            let client_id = env.secret("GOOGLE_OAUTH_CLIENT_ID")?.to_string();
            let client_secret = env.secret("GOOGLE_OAUTH_CLIENT_SECRET")?.to_string();
            let redirect_uri = format!("{}/auth/callback", base_url);

            // Exchange code for access token
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
                console_log!("Google token exchange failed: {}", token_text);
                return Response::error("Authentication failed. Please try again.", 500);
            }

            let tokens: TokenResponse = serde_json::from_str(&token_text)
                .map_err(|e| Error::from(format!("Failed to parse token response: {}", e)))?;

            // Get user info
            let headers = Headers::new();
            headers.set("Authorization", &format!("Bearer {}", tokens.access_token))?;

            let mut init = RequestInit::new();
            init.with_method(Method::Get).with_headers(headers);

            let userinfo_req = Request::new_with_init(GOOGLE_USERINFO_URL, &init)?;
            let mut userinfo_resp = Fetch::Request(userinfo_req).send().await?;
            let userinfo_text = userinfo_resp.text().await?;

            if userinfo_resp.status_code() != 200 {
                return Response::error("Failed to get user info", 500);
            }

            let user: GoogleUserInfo = serde_json::from_str(&userinfo_text)
                .map_err(|e| Error::from(format!("Failed to parse user info: {}", e)))?;

            let kv = env.kv("KV")?;

            // Check if this is a link request (user already signed in)
            if let Some(tenant_id) = resolve_tenant_id(&req, &kv).await {
                // Link Google to existing account
                if let Some(mut tenant) = get_tenant(&kv, &tenant_id).await? {
                    tenant.email = user.email;
                    if tenant.name.is_none() {
                        tenant.name = user.name;
                    }
                    tenant.updated_at = now_iso();
                    save_tenant(&kv, &tenant).await?;
                }
                let headers = Headers::new();
                headers.set("Location", "/admin/settings?success=google_linked")?;
                return Ok(Response::empty()?.with_status(302).with_headers(headers));
            }

            // Find or create tenant
            let tenant = match get_tenant_by_email(&kv, &user.email).await? {
                Some(t) => t,
                None => {
                    let now = now_iso();
                    let tenant = Tenant {
                        id: generate_id(),
                        email: user.email.clone(),
                        name: user.name,
                        facebook_id: None,
                        plan: "free".to_string(),
                        created_at: now.clone(),
                        updated_at: now,
                    };
                    save_tenant(&kv, &tenant).await?;
                    tenant
                }
            };

            create_session_and_redirect(&kv, &tenant.id, "google").await
        }

        // Facebook OAuth callback
        (Method::Get, "/auth/facebook/callback") => {
            let url = req.url()?;
            let query: std::collections::HashMap<_, _> = url.query_pairs().collect();

            let code = match query.get("code") {
                Some(c) => c.to_string(),
                None => {
                    let error = query
                        .get("error")
                        .map(|e| e.to_string())
                        .unwrap_or_default();
                    return Response::error(format!("Facebook OAuth error: {}", error), 400);
                }
            };

            let app_id = env.secret("META_APP_ID")?.to_string();
            let app_secret = env.secret("META_APP_SECRET")?.to_string();
            let redirect_uri = format!("{}/auth/facebook/callback", base_url);

            // Exchange code for access token
            let token_url = format!(
                "{}/{}/oauth/access_token?client_id={}&redirect_uri={}&client_secret={}&code={}",
                GRAPH_API_BASE,
                crate::META_API_VERSION,
                app_id,
                urlencoding::encode(&redirect_uri),
                app_secret,
                code
            );

            let mut init = RequestInit::new();
            init.with_method(Method::Get);
            let token_req = Request::new_with_init(&token_url, &init)?;
            let mut token_resp = Fetch::Request(token_req).send().await?;

            if token_resp.status_code() != 200 {
                let err = token_resp.text().await.unwrap_or_default();
                console_log!("Facebook token exchange failed: {}", err);
                return Response::error("Authentication failed. Please try again.", 500);
            }

            let body: serde_json::Value = token_resp.json().await?;
            let access_token = body
                .get("access_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::from("Missing access_token"))?
                .to_string();

            // Get Facebook user info
            let me_url = format!(
                "{}/{}/me?fields=id,name,email&access_token={}",
                GRAPH_API_BASE,
                crate::META_API_VERSION,
                access_token
            );
            let mut init = RequestInit::new();
            init.with_method(Method::Get);
            let me_req = Request::new_with_init(&me_url, &init)?;
            let mut me_resp = Fetch::Request(me_req).send().await?;

            if me_resp.status_code() != 200 {
                return Response::error("Failed to get Facebook user info", 500);
            }

            let fb_user: serde_json::Value = me_resp.json().await?;
            let fb_id = fb_user
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::from("Missing Facebook user id"))?
                .to_string();
            let fb_name = fb_user
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let fb_email = fb_user
                .get("email")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let kv = env.kv("KV")?;

            // Check if this is a link request (user already signed in)
            if let Some(tenant_id) = resolve_tenant_id(&req, &kv).await {
                if let Some(mut tenant) = get_tenant(&kv, &tenant_id).await? {
                    tenant.facebook_id = Some(fb_id);
                    if tenant.name.is_none() {
                        tenant.name = fb_name;
                    }
                    tenant.updated_at = now_iso();
                    save_tenant(&kv, &tenant).await?;
                }
                let headers = Headers::new();
                headers.set("Location", "/admin/settings?success=facebook_linked")?;
                return Ok(Response::empty()?.with_status(302).with_headers(headers));
            }

            // Find tenant by facebook_id, then by email, then create
            let tenant = if let Some(t) = get_tenant_by_facebook_id(&kv, &fb_id).await? {
                t
            } else if !fb_email.is_empty() {
                if let Some(mut t) = get_tenant_by_email(&kv, &fb_email).await? {
                    // Link Facebook to existing Google account
                    t.facebook_id = Some(fb_id);
                    t.updated_at = now_iso();
                    save_tenant(&kv, &t).await?;
                    t
                } else {
                    let now = now_iso();
                    let tenant = Tenant {
                        id: generate_id(),
                        email: fb_email,
                        name: fb_name,
                        facebook_id: Some(fb_id),
                        plan: "free".to_string(),
                        created_at: now.clone(),
                        updated_at: now,
                    };
                    save_tenant(&kv, &tenant).await?;
                    tenant
                }
            } else {
                // No email from Facebook — cannot create account without email
                return Response::error(
                    "Facebook did not provide an email address. Please sign in with Google instead.",
                    400,
                );
            };

            create_session_and_redirect(&kv, &tenant.id, "facebook").await
        }

        // Unlink a provider
        (Method::Delete, "/auth/unlink/google") => {
            let kv = env.kv("KV")?;
            let tenant_id = match resolve_tenant_id(&req, &kv).await {
                Some(id) => id,
                None => return Response::error("Unauthorized", 401),
            };
            let mut tenant = match get_tenant(&kv, &tenant_id).await? {
                Some(t) => t,
                None => return Response::error("Not found", 404),
            };

            // Must keep at least one provider
            if tenant.facebook_id.is_none() {
                return Response::from_html(
                    "<div class=\"error\">Cannot unlink Google. It is your only sign-in method. Link Facebook first.</div>",
                );
            }

            // Remove email index and clear email
            kv.delete(&format!("tenant_email:{}", tenant.email.to_lowercase()))
                .await?;
            tenant.email = String::new();
            tenant.updated_at = now_iso();
            save_tenant(&kv, &tenant).await?;
            Response::from_html("<div class=\"success\">Google account unlinked.</div>")
        }

        (Method::Delete, "/auth/unlink/facebook") => {
            let kv = env.kv("KV")?;
            let tenant_id = match resolve_tenant_id(&req, &kv).await {
                Some(id) => id,
                None => return Response::error("Unauthorized", 401),
            };
            let mut tenant = match get_tenant(&kv, &tenant_id).await? {
                Some(t) => t,
                None => return Response::error("Not found", 404),
            };

            // Must keep at least one provider
            if tenant.email.is_empty() {
                return Response::from_html(
                    "<div class=\"error\">Cannot unlink Facebook. It is your only sign-in method. Link Google first.</div>",
                );
            }

            if let Some(ref fb_id) = tenant.facebook_id {
                delete_tenant_facebook_index(&kv, fb_id).await?;
            }
            tenant.facebook_id = None;
            tenant.updated_at = now_iso();
            save_tenant(&kv, &tenant).await?;
            Response::from_html("<div class=\"success\">Facebook account unlinked.</div>")
        }

        (Method::Get, "/auth/logout") => {
            // Clear session from KV if exists
            if let Some(session_token) = get_session_cookie(&req) {
                let kv = env.kv("KV")?;
                delete_session(&kv, &session_token).await?;
            }

            let headers = Headers::new();
            headers.set("Location", "/auth/login")?;
            headers.set(
                "Set-Cookie",
                "session=; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=0",
            )?;

            Ok(Response::empty()?.with_status(302).with_headers(headers))
        }

        _ => Response::error("Not Found", 404),
    }
}

async fn create_session_and_redirect(
    kv: &kv::KvStore,
    tenant_id: &str,
    provider: &str,
) -> Result<Response> {
    let session_token = generate_token()?;
    let csrf_token = generate_token()?;
    save_session(kv, &session_token, tenant_id, SESSION_TTL_SECONDS).await?;
    save_csrf_token(kv, tenant_id, &csrf_token, SESSION_TTL_SECONDS).await?;

    let headers = Headers::new();
    headers.set("Location", "/admin")?;
    headers.set(
        "Set-Cookie",
        &format!(
            "session={}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={}",
            session_token, SESSION_TTL_SECONDS
        ),
    )?;
    headers.append(
        "Set-Cookie",
        &format!(
            "last_provider={}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=2592000",
            provider
        ),
    )?;
    // CSRF cookie — NOT HttpOnly so HTMX JS can read it
    headers.append(
        "Set-Cookie",
        &format!(
            "csrf={}; Path=/; Secure; SameSite=Strict; Max-Age={}",
            csrf_token, SESSION_TTL_SECONDS
        ),
    )?;

    Ok(Response::empty()?.with_status(302).with_headers(headers))
}

/// Extract a named cookie from request.
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

/// Extract session cookie from request.
pub fn get_session_cookie(req: &Request) -> Option<String> {
    get_cookie(req, "session")
}

/// Resolve tenant_id from session cookie, returns None if not authenticated
pub async fn resolve_tenant_id(req: &Request, kv: &kv::KvStore) -> Option<String> {
    let token = get_session_cookie(req)?;
    get_session(kv, &token).await.ok()?
}

/// Validate CSRF token from X-CSRF-Token header or csrf form field against stored token.
pub async fn validate_csrf(
    req: &Request,
    kv: &kv::KvStore,
    tenant_id: &str,
) -> std::result::Result<(), String> {
    use subtle::ConstantTimeEq;

    // Get token from header (HTMX) or cookie (double-submit)
    let submitted = req
        .headers()
        .get("X-CSRF-Token")
        .ok()
        .flatten()
        .or_else(|| get_cookie(req, "csrf"))
        .ok_or_else(|| "Missing CSRF token".to_string())?;

    let stored = get_csrf_token(kv, tenant_id)
        .await
        .map_err(|e| format!("CSRF lookup failed: {e}"))?
        .ok_or_else(|| "No CSRF token stored for session".to_string())?;

    let valid: bool = submitted.as_bytes().ct_eq(stored.as_bytes()).into();
    if !valid {
        return Err("CSRF token mismatch".to_string());
    }
    Ok(())
}
