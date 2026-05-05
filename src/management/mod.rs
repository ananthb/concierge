//! Management panel: super-admin routes gated by Cloudflare Access.
//! Verifies the Cf-Access-Jwt-Assertion JWT against the team's JWKS.

pub mod archetypes;
pub mod audit;
pub mod billing;
pub mod demo;
pub mod tenants;

use wasm_bindgen::JsCast;
use worker::*;

use crate::templates::management as tmpl;

/// Handle /manage/* routes. Requires Cloudflare Access.
pub async fn handle_management(
    req: Request,
    env: Env,
    path: &str,
    method: Method,
) -> Result<Response> {
    let email = match verify_access(&req, &env).await {
        Some(e) => e,
        None => return Response::error("Forbidden: Cloudflare Access required", 403),
    };

    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;
    let base_url = crate::handlers::get_base_url(&req);
    let locale = crate::locale::Locale::from_request(&req);

    let sub = path
        .strip_prefix("/manage")
        .unwrap_or("")
        .trim_start_matches('/');

    // Route subroutes first (before consuming method in match)
    if sub.starts_with("tenants") {
        return tenants::handle_tenants(req, &env, &kv, &db, sub, method, &email, &base_url).await;
    }

    if sub.starts_with("billing") {
        return billing::handle_billing(req, &kv, &db, sub, method, &email, &base_url).await;
    }

    if sub.starts_with("archetypes") {
        return archetypes::handle_archetypes(req, &env, &db, sub, method, &email, &base_url).await;
    }

    if sub.starts_with("demo") {
        return demo::handle_demo(req, &env, &db, sub, method, &email, &base_url).await;
    }

    match (method, sub) {
        (Method::Get, "" | "/") => {
            let tenant_count = crate::storage::count_tenants(&db).await.unwrap_or(0);
            let report = crate::handlers::health::run_checks(&env, true).await;
            Response::from_html(tmpl::dashboard_html(
                &email,
                tenant_count,
                &report,
                &base_url,
                &locale,
            ))
        }

        (Method::Get, "audit") => {
            // Parse filter + cursor query params once. The HX-Request
            // header distinguishes a "Load older" page swap (`before`
            // is set) and a filter-input swap (no `before`) from the
            // initial full-page render. Page size = 50 so a few
            // pages cover most operator scrutiny without hammering D1.
            let url = req.url()?;
            let mut actor = String::new();
            let mut action = String::new();
            let mut resource_type = String::new();
            let mut before = String::new();
            for (k, v) in url.query_pairs() {
                match k.as_ref() {
                    "actor" => actor = v.into_owned(),
                    "action" => action = v.into_owned(),
                    "resource_type" => resource_type = v.into_owned(),
                    "before" => before = v.into_owned(),
                    _ => {}
                }
            }
            const PAGE_SIZE: u32 = 50;
            let log =
                audit::search_audit_log(&db, &actor, &action, &resource_type, &before, PAGE_SIZE)
                    .await?;
            let has_more = log.len() as u32 == PAGE_SIZE;
            let is_htmx = req.headers().get("HX-Request").ok().flatten().is_some();
            if is_htmx && !before.is_empty() {
                // "Load older" click: append-only fragment.
                Response::from_html(tmpl::audit_page_fragment_html(
                    &log,
                    &actor,
                    &action,
                    &resource_type,
                    has_more,
                    &base_url,
                ))
            } else if is_htmx {
                Response::from_html(tmpl::audit_table_html(
                    &log,
                    &actor,
                    &action,
                    &resource_type,
                    has_more,
                    &base_url,
                ))
            } else {
                Response::from_html(tmpl::audit_html(
                    &log,
                    &actor,
                    &action,
                    &resource_type,
                    has_more,
                    &email,
                    &base_url,
                    &locale,
                ))
            }
        }

        _ => Response::error("Not Found", 404),
    }
}

/// Verify the Cloudflare Access JWT and return the authenticated email.
///
/// Token lookup tries the `Cf-Access-Jwt-Assertion` header first, then
/// falls back to the `CF_Authorization` cookie. Access sets *both* on
/// requests it forwards to the worker, but if the header is somehow
/// stripped (proxy, custom domain config, browser caching the original
/// pre-Access response), the cookie still carries a valid JWT.
///
/// Misconfiguration (missing CF_ACCESS_AUD / CF_ACCESS_TEAM, neither
/// header nor cookie present, signature mismatch) all log a specific
/// diagnostic to `console_log!` so failed-Access debugging from
/// `wrangler tail` shows *why* the 403 happened.
async fn verify_access(req: &Request, env: &Env) -> Option<String> {
    // Local-dev bypass for the management panel and the AI stubs that
    // back it (see `crate::dev_bypass`). Active iff BOTH conditions
    // hold simultaneously:
    //   1. `CF_ACCESS_AUD` is empty (production sets this via the
    //      Cloudflare Workers build env, so prod can never bypass).
    //   2. `MANAGE_BYPASS_EMAIL` is set to a non-empty value (the dev
    //      explicitly opts in by setting this in their .env file).
    // The double-gate means even if MANAGE_BYPASS_EMAIL leaks into a
    // prod deploy, the presence of CF_ACCESS_AUD still blocks it.
    if let Some(email) = crate::dev_bypass::manage_bypass_email(env) {
        worker::console_log!("Access: dev bypass active, treating actor as {email}");
        return Some(email);
    }

    let token = match req.headers().get("Cf-Access-Jwt-Assertion").ok().flatten() {
        Some(t) => t,
        None => match crate::handlers::auth::get_cookie(req, "CF_Authorization") {
            Some(t) => t,
            None => {
                worker::console_log!(
                    "Access: no Cf-Access-Jwt-Assertion header and no CF_Authorization cookie"
                );
                return None;
            }
        },
    };

    let aud = match env.var("CF_ACCESS_AUD").map(|v| v.to_string()).ok() {
        Some(a) if !a.is_empty() => a,
        _ => {
            worker::console_log!("Access: CF_ACCESS_AUD not set or empty");
            return None;
        }
    };
    let team_domain = match env.var("CF_ACCESS_TEAM").map(|v| v.to_string()).ok() {
        Some(t) if !t.is_empty() => t,
        _ => {
            worker::console_log!("Access: CF_ACCESS_TEAM not set or empty");
            return None;
        }
    };

    match verify_cf_jwt(&token, &aud, &team_domain).await {
        Ok(email) => Some(email),
        Err(e) => {
            worker::console_log!("Access JWT verification failed: {e}");
            None
        }
    }
}

/// Verify a Cloudflare Access RS256 JWT.
/// Returns the email claim on success.
async fn verify_cf_jwt(token: &str, expected_aud: &str, team_domain: &str) -> Result<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(Error::from("Invalid JWT format"));
    }

    let header: serde_json::Value = decode_jwt_part(parts[0])?;
    let payload: serde_json::Value = decode_jwt_part(parts[1])?;

    // Check algorithm
    if header.get("alg").and_then(|v| v.as_str()) != Some("RS256") {
        return Err(Error::from("Unsupported JWT algorithm"));
    }

    let kid = header
        .get("kid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::from("Missing kid in JWT header"))?;

    // Verify audience
    let aud_valid = match payload.get("aud") {
        Some(serde_json::Value::Array(arr)) => arr.iter().any(|v| v.as_str() == Some(expected_aud)),
        Some(serde_json::Value::String(s)) => s == expected_aud,
        _ => false,
    };
    if !aud_valid {
        return Err(Error::from("JWT audience mismatch"));
    }

    // Check expiry
    let now = (js_sys::Date::now() / 1000.0) as u64;
    let exp = payload
        .get("exp")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Error::from("Missing exp claim"))?;
    if now > exp {
        return Err(Error::from("JWT expired"));
    }

    // Fetch JWKS and find the matching key
    let certs_url = format!("https://{team_domain}.cloudflareaccess.com/cdn-cgi/access/certs");
    let mut resp = Fetch::Url(Url::parse(&certs_url).map_err(|_| Error::from("Bad certs URL"))?)
        .send()
        .await?;
    let jwks: serde_json::Value = resp.json().await?;

    let keys = jwks
        .get("keys")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::from("Invalid JWKS response"))?;

    let jwk = keys
        .iter()
        .find(|k| k.get("kid").and_then(|v| v.as_str()) == Some(kid))
        .ok_or_else(|| Error::from("No matching key in JWKS"))?;

    // Import the RSA public key and verify signature
    let crypto = get_subtle()?;

    let algorithm = js_sys::Object::new();
    js_sys::Reflect::set(&algorithm, &"name".into(), &"RSASSA-PKCS1-v1_5".into())
        .map_err(|_| Error::from("reflect error"))?;
    js_sys::Reflect::set(&algorithm, &"hash".into(), &"SHA-256".into())
        .map_err(|_| Error::from("reflect error"))?;

    let jwk_js = serde_json::to_string(jwk).map_err(|e| Error::from(format!("JSON: {e}")))?;
    let jwk_obj: js_sys::Object = js_sys::JSON::parse(&jwk_js)
        .map_err(|_| Error::from("JWK parse error"))?
        .dyn_into()
        .map_err(|_| Error::from("JWK not an object"))?;

    let usages = js_sys::Array::new();
    usages.push(&"verify".into());

    let key_promise = crypto
        .import_key_with_object("jwk", &jwk_obj, &algorithm, false, &usages)
        .map_err(|_| Error::from("importKey failed"))?;
    let crypto_key: web_sys::CryptoKey = wasm_bindgen_futures::JsFuture::from(key_promise)
        .await
        .map_err(|_| Error::from("importKey await failed"))?
        .into();

    let signed_input = format!("{}.{}", parts[0], parts[1]);
    let signature = base64_url_decode(parts[2])?;

    let verify_promise = crypto
        .verify_with_object_and_buffer_source_and_buffer_source(
            &algorithm,
            &crypto_key,
            &js_sys::Uint8Array::from(signature.as_slice()),
            &js_sys::Uint8Array::from(signed_input.as_bytes()),
        )
        .map_err(|_| Error::from("verify call failed"))?;

    let valid = wasm_bindgen_futures::JsFuture::from(verify_promise)
        .await
        .map_err(|_| Error::from("verify await failed"))?
        .as_bool()
        .unwrap_or(false);

    if !valid {
        return Err(Error::from("JWT signature invalid"));
    }

    payload
        .get("email")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::from("Missing email in JWT"))
}

fn decode_jwt_part(part: &str) -> Result<serde_json::Value> {
    let bytes = base64_url_decode(part)?;
    serde_json::from_slice(&bytes).map_err(|e| Error::from(format!("JWT decode: {e}")))
}

fn base64_url_decode(input: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(input)
        .map_err(|e| Error::from(format!("base64: {e}")))
}

fn get_subtle() -> Result<web_sys::SubtleCrypto> {
    let global = js_sys::global();
    let crypto = js_sys::Reflect::get(&global, &"crypto".into())
        .map_err(|_| Error::from("Failed to get crypto"))?;
    let crypto: web_sys::Crypto = crypto
        .dyn_into()
        .map_err(|_| Error::from("Not a Crypto object"))?;
    Ok(crypto.subtle())
}
