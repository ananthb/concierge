//! WhatsApp Embedded Signup callback handler (`/whatsapp/signup/callback`).

use serde::Serialize;
use worker::*;

use crate::helpers::*;
use crate::storage::*;
use crate::types::*;

struct SignupParams {
    code: String,
    state: String,
    phone_number_id: Option<String>,
    waba_id: Option<String>,
}

#[derive(Serialize)]
struct SignupSuccessView {
    ok: bool,
    destination: &'static str,
    account: Option<SignupAccountView>,
}

#[derive(Serialize)]
struct SignupAccountView {
    id: String,
    name: String,
    phone_number: String,
    phone_number_id: String,
    waba_id: Option<String>,
}

/// Handle `/whatsapp/signup/*` routes (`POST` and `GET /whatsapp/signup/callback`).
pub async fn handle_whatsapp_signup(
    mut req: Request,
    env: Env,
    path: &str,
    method: Method,
) -> Result<Response> {
    let path_parts: Vec<&str> = path
        .strip_prefix("/whatsapp/signup/")
        .unwrap_or("")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    match (method, path_parts.as_slice()) {
        (Method::Post, ["callback"]) | (Method::Get, ["callback"]) => {
            let is_post = req.method() == Method::Post;
            let content_type = req
                .headers()
                .get("Content-Type")
                .ok()
                .flatten()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let accept = req
                .headers()
                .get("Accept")
                .ok()
                .flatten()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let wants_json = is_post
                && (content_type.contains("application/json")
                    || accept.contains("application/json"));

            let db = env.d1("DB")?;

            let logged_in_tenant = super::auth::resolve_tenant_id(&req, &db).await;
            let is_logged_in = logged_in_tenant.is_some();
            let err_destination = match &logged_in_tenant {
                Some(tid) => {
                    let ob = get_onboarding(&db, tid).await.unwrap_or_default();
                    if ob.completed {
                        "dashboard"
                    } else {
                        "wizard"
                    }
                }
                None => "login",
            };

            let params = if !is_post {
                parse_get_params(&req)?
            } else if content_type.contains("application/json") {
                parse_json_params(&mut req).await?
            } else {
                parse_form_params(&mut req).await?
            };

            if params.state.is_empty() {
                return fail(
                    wants_json,
                    err_destination,
                    "invalid_state",
                    "Invalid or expired signup state.",
                    400,
                );
            }
            let state_valid = match (
                &logged_in_tenant,
                take_signup_state(&db, &params.state).await?,
            ) {
                (Some(tid), Some(state_tid)) => state_tid == *tid,
                (None, Some(state_tid)) => state_tid.is_empty(),
                _ => false,
            };
            if !state_valid {
                return fail(
                    wants_json,
                    err_destination,
                    "invalid_state",
                    "Invalid or expired signup state.",
                    400,
                );
            }

            if params.code.is_empty() {
                return fail(
                    wants_json,
                    err_destination,
                    "missing_code",
                    "Missing Meta OAuth code.",
                    400,
                );
            }

            let app_id = env
                .var("META_APP_ID")
                .map(|v| v.to_string().trim().to_string())
                .unwrap_or_default();
            let app_secret = env
                .secret("META_APP_SECRET")
                .map(|s| s.to_string().trim().to_string())
                .unwrap_or_default();
            let platform_token = env
                .secret("WHATSAPP_ACCESS_TOKEN")
                .map(|s| s.to_string().trim().to_string())
                .unwrap_or_default();
            let fallback_waba_id = env
                .var("WHATSAPP_WABA_ID")
                .map(|v| v.to_string().trim().to_string())
                .unwrap_or_default();

            let public_base = env
                .var("PUBLIC_BASE_URL")
                .map(|v| v.to_string())
                .unwrap_or_default();
            let base = if !public_base.trim().is_empty() {
                public_base.trim().trim_end_matches('/').to_string()
            } else {
                let u = req.url()?;
                format!("{}://{}", u.scheme(), u.host_str().unwrap_or(""))
            };
            let callback_redirect_uri = format!("{base}/whatsapp/signup/callback");

            let token_res = if is_post {
                match exchange_code(&params.code, &app_id, &app_secret, None).await {
                    Ok(t) => Ok(t),
                    Err(_) => {
                        exchange_code(
                            &params.code,
                            &app_id,
                            &app_secret,
                            Some(&callback_redirect_uri),
                        )
                        .await
                    }
                }
            } else {
                match exchange_code(
                    &params.code,
                    &app_id,
                    &app_secret,
                    Some(&callback_redirect_uri),
                )
                .await
                {
                    Ok(t) => Ok(t),
                    Err(_) => exchange_code(&params.code, &app_id, &app_secret, None).await,
                }
            };

            let short_token = match token_res {
                Ok(t) => t,
                Err(e) => {
                    console_log!("WhatsApp signup token exchange failed: {:?}", e);
                    return fail(
                        wants_json,
                        err_destination,
                        "token_exchange_failed",
                        "Could not exchange Meta authorization code.",
                        502,
                    );
                }
            };
            let user_token = exchange_for_long_lived_token(&short_token, &app_id, &app_secret)
                .await
                .unwrap_or(short_token);

            let meta_user = fetch_meta_user(&user_token).await;
            let tenant_id = match logged_in_tenant.clone() {
                Some(tid) => {
                    // Best-effort: persist `tenants.facebook_id` so `POST /data-deletion`
                    // can locate this tenant for Meta App Review.
                    if let Some(ref user) = meta_user {
                        if let Err(e) = set_tenant_facebook_id(&db, &tid, &user.id).await {
                            console_log!("Failed to save facebook_id on tenant {}: {:?}", tid, e);
                        }
                    }
                    tid
                }
                None => {
                    let Some(user) = meta_user else {
                        return fail(
                            wants_json,
                            err_destination,
                            "meta_profile_failed",
                            "Could not verify your Meta account.",
                            502,
                        );
                    };
                    let existing = match get_tenant_by_facebook_id(&db, &user.id).await? {
                        Some(t) => Some(t),
                        None => match user.email.as_deref() {
                            Some(email) => {
                                let found = get_tenant_by_email(&db, email).await?;
                                if let Some(ref t) = found {
                                    if t.facebook_id.is_none() {
                                        let _ = set_tenant_facebook_id(&db, &t.id, &user.id).await;
                                    }
                                }
                                found
                            }
                            None => None,
                        },
                    };
                    match existing {
                        Some(t) => t.id,
                        None => {
                            let signup_locale = crate::locale::Locale::from_request(&req);
                            let now = now_iso();
                            let tenant = Tenant {
                                id: generate_id(),
                                email: user.email.unwrap_or_default(),
                                name: user.name,
                                facebook_id: Some(user.id),
                                plan: Plan::Paid,
                                locale: signup_locale.langid.to_string(),
                                currency: signup_locale.currency,
                                created_at: now.clone(),
                                updated_at: now,
                            };
                            save_tenant(&db, &tenant).await?;
                            tenant.id
                        }
                    }
                }
            };

            let app_token = if !app_id.trim().is_empty() && !app_secret.trim().is_empty() {
                format!("{}|{}", app_id.trim(), app_secret.trim())
            } else {
                String::new()
            };

            let mut tenant_supplied_waba = params.waba_id.clone().filter(|s| !s.is_empty());
            let mut pre_discovered_phone: Option<(String, String)> = None;
            if tenant_supplied_waba.is_none() {
                let discovered_wabas = discover_all_wabas_from_token(&user_token, &app_token).await;
                if discovered_wabas.len() > 1 {
                    let (chosen_waba, chosen_phone) = select_best_waba(
                        &discovered_wabas,
                        params.phone_number_id.as_deref(),
                        &user_token,
                        &platform_token,
                    )
                    .await;
                    tenant_supplied_waba = Some(chosen_waba);
                    pre_discovered_phone = chosen_phone;
                } else {
                    tenant_supplied_waba = discovered_wabas.into_iter().next();
                }
            }
            let effective_waba_id = tenant_supplied_waba.clone().unwrap_or(fallback_waba_id);

            if !effective_waba_id.is_empty() {
                let mut subscribed = false;
                if !user_token.trim().is_empty() {
                    subscribed = subscribe_waba_app(&effective_waba_id, &user_token).await;
                }
                if !subscribed && !platform_token.trim().is_empty() && platform_token != user_token
                {
                    let _ = subscribe_waba_app(&effective_waba_id, &platform_token).await;
                }
            }

            let resolved_phone = if let Some(pnid) = params.phone_number_id {
                let mut found = if !user_token.trim().is_empty() {
                    get_phone_number_details(&effective_waba_id, &pnid, &user_token)
                        .await
                        .ok()
                        .flatten()
                } else {
                    None
                };
                if found.is_none() && !platform_token.trim().is_empty() {
                    found = get_phone_number_details(&effective_waba_id, &pnid, &platform_token)
                        .await
                        .ok()
                        .flatten();
                }
                match found {
                    Some((display_phone, _)) => Ok((display_phone, pnid)),
                    None => Err((
                        "phone_not_found",
                        "Could not verify that WhatsApp phone number.",
                    )),
                }
            } else if effective_waba_id.is_empty() {
                Err(("missing_waba", "Missing WhatsApp Business Account ID."))
            } else {
                let mut found = pre_discovered_phone;
                if found.is_none() && !user_token.trim().is_empty() {
                    found = discover_new_phone(&effective_waba_id, &user_token)
                        .await
                        .ok()
                        .flatten();
                }
                if found.is_none() && !platform_token.trim().is_empty() {
                    found = discover_new_phone(&effective_waba_id, &platform_token)
                        .await
                        .ok()
                        .flatten();
                }
                match found {
                    Some(result) => Ok(result),
                    None => Err((
                        "no_phone_numbers",
                        "No WhatsApp phone numbers found on that account.",
                    )),
                }
            };

            let account_view = match resolved_phone {
                Ok((phone_number, phone_number_id)) => {
                    if !phone_number_id.is_empty() {
                        let mut registered = false;
                        if !user_token.trim().is_empty() {
                            registered = register_phone_number(&phone_number_id, &user_token).await;
                        }
                        if !registered
                            && !platform_token.trim().is_empty()
                            && platform_token != user_token
                        {
                            let _ = register_phone_number(&phone_number_id, &platform_token).await;
                        }
                    }
                    let waba_opt = if effective_waba_id.is_empty() {
                        None
                    } else {
                        Some(effective_waba_id)
                    };
                    let token_opt = if user_token.trim().is_empty() {
                        None
                    } else {
                        Some(user_token.trim().to_string())
                    };
                    let existing_account =
                        match get_whatsapp_account_by_phone(&db, &phone_number_id).await? {
                            Some(id) => get_whatsapp_account(&db, &id).await?,
                            None => None,
                        };
                    let account = match existing_account {
                        Some(mut existing) if existing.tenant_id == tenant_id => {
                            existing.phone_number = phone_number;
                            if let Some(waba) = waba_opt {
                                existing.waba_id = Some(waba);
                            }
                            if let Some(tok) = token_opt {
                                existing.auto_reply.access_token = Some(tok);
                            }
                            existing.updated_at = now_iso();
                            save_whatsapp_account(&db, &existing).await?;
                            existing
                        }
                        Some(_) => {
                            return fail(
                                wants_json,
                                err_destination,
                                "phone_in_use",
                                "That WhatsApp phone number is already connected to another account.",
                                409,
                            );
                        }
                        None => {
                            let now = now_iso();
                            let created = WhatsAppAccount {
                                id: generate_id(),
                                tenant_id: tenant_id.clone(),
                                name: format!("WhatsApp {}", phone_number),
                                phone_number,
                                phone_number_id,
                                waba_id: waba_opt,
                                auto_reply: ReplyConfig {
                                    access_token: token_opt,
                                    ..ReplyConfig::default()
                                },
                                canned_safety: SafetyState::default(),
                                created_at: now.clone(),
                                updated_at: now,
                            };
                            save_whatsapp_account(&db, &created).await?;
                            created
                        }
                    };
                    Some(SignupAccountView {
                        id: account.id,
                        name: account.name,
                        phone_number: account.phone_number,
                        phone_number_id: account.phone_number_id,
                        waba_id: account.waba_id,
                    })
                }
                Err((code, msg)) => {
                    if is_logged_in {
                        return fail(wants_json, err_destination, code, msg, 400);
                    }
                    None
                }
            };

            let onboarding = get_onboarding(&db, &tenant_id).await?;
            let destination = if onboarding.completed {
                "dashboard"
            } else {
                "wizard"
            };
            let session_cookie = if logged_in_tenant.is_none() {
                Some(super::auth::mint_session_cookie(&req, &db, &tenant_id).await?)
            } else {
                None
            };

            if wants_json {
                let mut resp = crate::api::json(&SignupSuccessView {
                    ok: true,
                    destination,
                    account: account_view,
                })?;
                if let Some(cookie) = session_cookie {
                    resp.headers_mut().set("Set-Cookie", &cookie)?;
                }
                Ok(resp)
            } else {
                let headers = Headers::new();
                headers.set("Location", &format!("/{destination}"))?;
                headers.set("Cache-Control", "no-store")?;
                if let Some(cookie) = session_cookie {
                    headers.set("Set-Cookie", &cookie)?;
                }
                Ok(Response::empty()?.with_status(302).with_headers(headers))
            }
        }

        _ => Response::error("Not Found", 404),
    }
}

fn parse_get_params(req: &Request) -> Result<SignupParams> {
    let url = req.url()?;
    let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
    Ok(SignupParams {
        code: query
            .get("code")
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        state: query
            .get("state")
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        phone_number_id: query
            .get("phone_number_id")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        waba_id: query
            .get("waba_id")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    })
}

async fn parse_json_params(req: &mut Request) -> Result<SignupParams> {
    let body = req
        .json::<serde_json::Value>()
        .await
        .unwrap_or(serde_json::Value::Null);
    let get = |k: &str| {
        body.get(k)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let phone_number_id = get("phone_number_id");
    let waba_id = get("waba_id");
    Ok(SignupParams {
        code: get("code"),
        state: get("state"),
        phone_number_id: if phone_number_id.is_empty() {
            None
        } else {
            Some(phone_number_id)
        },
        waba_id: if waba_id.is_empty() {
            None
        } else {
            Some(waba_id)
        },
    })
}

async fn parse_form_params(req: &mut Request) -> Result<SignupParams> {
    let form = req.form_data().await?;
    let get = |k: &str| match form.get(k) {
        Some(FormEntry::Field(v)) => v.trim().to_string(),
        _ => String::new(),
    };
    let phone_number_id = get("phone_number_id");
    let waba_id = get("waba_id");
    Ok(SignupParams {
        code: get("code"),
        state: get("state"),
        phone_number_id: if phone_number_id.is_empty() {
            None
        } else {
            Some(phone_number_id)
        },
        waba_id: if waba_id.is_empty() {
            None
        } else {
            Some(waba_id)
        },
    })
}

fn fail(
    wants_json: bool,
    destination: &str,
    code: &str,
    message: &str,
    status: u16,
) -> Result<Response> {
    if wants_json {
        crate::api::error(code, message, status)
    } else {
        let headers = Headers::new();
        headers.set("Location", &format!("/{destination}?error={code}"))?;
        Ok(Response::empty()?.with_status(302).with_headers(headers))
    }
}

fn push_unique_id(out: &mut Vec<String>, id: &str) {
    let trimmed = id.trim();
    if !trimmed.is_empty() && !out.iter().any(|existing| existing == trimmed) {
        out.push(trimmed.to_string());
    }
}

async fn discover_all_wabas_from_token(user_token: &str, app_token: &str) -> Vec<String> {
    let mut wabas = Vec::new();
    if !app_token.trim().is_empty() {
        let url = format!(
            "https://graph.facebook.com/{}/debug_token?input_token={}&access_token={}",
            crate::META_API_VERSION,
            urlencoding::encode(user_token),
            urlencoding::encode(app_token.trim()),
        );
        let mut init = RequestInit::new();
        init.with_method(Method::Get);
        if let Ok(request) = Request::new_with_init(&url, &init) {
            if let Ok(mut response) = Fetch::Request(request).send().await {
                if response.status_code() == 200 {
                    if let Ok(body) = response.json::<serde_json::Value>().await {
                        for id in extract_all_wabas_from_debug_token(&body) {
                            push_unique_id(&mut wabas, &id);
                        }
                        if !wabas.is_empty() {
                            return wabas;
                        }
                    }
                }
            }
        }
    }

    for path in [
        "me/businesses?fields=id,owned_whatsapp_business_accounts{id},client_whatsapp_business_accounts{id}",
        "me?fields=id,whatsapp_business_accounts{id},accounts{id,whatsapp_business_accounts{id}}",
        "me/assigned_whatsapp_business_accounts?fields=id",
    ] {
        let url = format!(
            "https://graph.facebook.com/{}/{}&access_token={}",
            crate::META_API_VERSION,
            path,
            urlencoding::encode(user_token),
        );
        let mut init = RequestInit::new();
        init.with_method(Method::Get);
        let Ok(request) = Request::new_with_init(&url, &init) else {
            continue;
        };
        let Ok(mut response) = Fetch::Request(request).send().await else {
            continue;
        };
        if response.status_code() != 200 {
            continue;
        }
        let Ok(body) = response.json::<serde_json::Value>().await else {
            continue;
        };
        for id in extract_all_wabas_from_businesses(&body) {
            push_unique_id(&mut wabas, &id);
        }
    }

    wabas
}

fn collect_ids(arr: &[serde_json::Value], out: &mut Vec<String>) {
    for item in arr {
        if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
            push_unique_id(out, id);
        }
    }
}

fn extract_all_wabas_from_businesses(body: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();

    if let Some(wabas) = body
        .get("whatsapp_business_accounts")
        .and_then(|w| w.get("data"))
        .and_then(|d| d.as_array())
    {
        collect_ids(wabas, &mut out);
    }

    if let Some(businesses) = body.get("data").and_then(|d| d.as_array()) {
        // When querying /me/assigned_whatsapp_business_accounts, `data` is the WABA list itself
        let has_nested = businesses.iter().any(|b| {
            b.get("owned_whatsapp_business_accounts").is_some()
                || b.get("client_whatsapp_business_accounts").is_some()
        });
        if !has_nested {
            collect_ids(businesses, &mut out);
        }
        for biz in businesses {
            for key in [
                "owned_whatsapp_business_accounts",
                "client_whatsapp_business_accounts",
            ] {
                if let Some(wabas) = biz
                    .get(key)
                    .and_then(|w| w.get("data"))
                    .and_then(|d| d.as_array())
                {
                    collect_ids(wabas, &mut out);
                }
            }
        }
    }

    if let Some(accounts) = body
        .get("accounts")
        .and_then(|a| a.get("data"))
        .and_then(|d| d.as_array())
    {
        for acct in accounts {
            if let Some(wabas) = acct
                .get("whatsapp_business_accounts")
                .and_then(|w| w.get("data"))
                .and_then(|d| d.as_array())
            {
                collect_ids(wabas, &mut out);
            }
        }
    }

    out
}

#[cfg(test)]
fn extract_waba_from_businesses(body: &serde_json::Value) -> Option<String> {
    extract_all_wabas_from_businesses(body).into_iter().next()
}

fn extract_all_wabas_from_debug_token(body: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(scopes) = body
        .get("data")
        .and_then(|d| d.get("granular_scopes"))
        .and_then(|g| g.as_array())
    else {
        return out;
    };
    for entry in scopes {
        let scope = entry.get("scope").and_then(|s| s.as_str()).unwrap_or("");
        if scope == "whatsapp_business_management" || scope == "whatsapp_business_messaging" {
            if let Some(ids) = entry.get("target_ids").and_then(|t| t.as_array()) {
                for id in ids {
                    if let Some(s) = id.as_str() {
                        push_unique_id(&mut out, s);
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
fn extract_waba_from_debug_token(body: &serde_json::Value) -> Option<String> {
    extract_all_wabas_from_debug_token(body).into_iter().next()
}

struct MetaUserInfo {
    id: String,
    name: Option<String>,
    email: Option<String>,
}

async fn fetch_meta_user(user_token: &str) -> Option<MetaUserInfo> {
    for fields in ["id,name,email", "id,name"] {
        let url = format!(
            "https://graph.facebook.com/{}/me?fields={}&access_token={}",
            crate::META_API_VERSION,
            fields,
            urlencoding::encode(user_token),
        );
        let mut init = RequestInit::new();
        init.with_method(Method::Get);
        let Ok(request) = Request::new_with_init(&url, &init) else {
            continue;
        };
        let Ok(mut response) = Fetch::Request(request).send().await else {
            continue;
        };
        if response.status_code() != 200 {
            continue;
        }
        let Ok(body) = response.json::<serde_json::Value>().await else {
            continue;
        };
        let Some(id) = body
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
        else {
            continue;
        };
        let name = body
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from);
        let email = body
            .get("email")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase());
        return Some(MetaUserInfo { id, name, email });
    }
    None
}

async fn subscribe_waba_app(waba_id: &str, token: &str) -> bool {
    let url = format!(
        "https://graph.facebook.com/{}/{}/subscribed_apps",
        crate::META_API_VERSION,
        waba_id
    );
    let headers = Headers::new();
    if headers
        .set("Authorization", &format!("Bearer {token}"))
        .is_err()
    {
        return false;
    }
    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_headers(headers);
    let Ok(req) = Request::new_with_init(&url, &init) else {
        return false;
    };
    match Fetch::Request(req).send().await {
        Ok(resp) if resp.status_code() < 300 => true,
        Ok(mut resp) => {
            let text = resp.text().await.unwrap_or_default();
            console_log!(
                "Warning: subscribed_apps on WABA {} returned HTTP {}: {}",
                waba_id,
                resp.status_code(),
                text
            );
            false
        }
        Err(e) => {
            console_log!(
                "Warning: subscribed_apps on WABA {} failed: {:?}",
                waba_id,
                e
            );
            false
        }
    }
}

async fn register_phone_number(phone_number_id: &str, token: &str) -> bool {
    let url = format!(
        "https://graph.facebook.com/{}/{}/register",
        crate::META_API_VERSION,
        urlencoding::encode(phone_number_id)
    );
    let headers = Headers::new();
    if headers
        .set("Authorization", &format!("Bearer {}", token.trim()))
        .is_err()
        || headers.set("Content-Type", "application/json").is_err()
    {
        return false;
    }
    let payload = serde_json::json!({
        "messaging_product": "whatsapp",
        "pin": "000000"
    });
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&payload.to_string())));
    let Ok(req) = Request::new_with_init(&url, &init) else {
        return false;
    };
    let registered = match Fetch::Request(req).send().await {
        Ok(resp) if resp.status_code() < 300 => true,
        Ok(mut resp) => {
            let text = resp.text().await.unwrap_or_default();
            console_log!(
                "Warning: register on phone_number_id {} returned HTTP {}: {}",
                phone_number_id,
                resp.status_code(),
                text
            );
            false
        }
        Err(e) => {
            console_log!(
                "Warning: register on phone_number_id {} failed: {:?}",
                phone_number_id,
                e
            );
            false
        }
    };
    if !registered {
        let smb_ok = sync_smb_app_data(phone_number_id, token).await;
        if smb_ok {
            return true;
        }
    }
    registered
}

async fn sync_smb_app_data(phone_number_id: &str, token: &str) -> bool {
    let url = format!(
        "https://graph.facebook.com/{}/{}/smb_app_data",
        crate::META_API_VERSION,
        urlencoding::encode(phone_number_id)
    );
    let mut any_ok = false;
    for sync_type in ["smb_app_state_sync", "history"] {
        let headers = Headers::new();
        if headers
            .set("Authorization", &format!("Bearer {}", token.trim()))
            .is_err()
            || headers.set("Content-Type", "application/json").is_err()
        {
            continue;
        }
        let payload = serde_json::json!({
            "messaging_product": "whatsapp",
            "sync_type": sync_type
        });
        let mut init = RequestInit::new();
        init.with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(wasm_bindgen::JsValue::from_str(&payload.to_string())));
        if let Ok(req) = Request::new_with_init(&url, &init) {
            if let Ok(resp) = Fetch::Request(req).send().await {
                if resp.status_code() < 300 {
                    any_ok = true;
                }
            }
        }
    }
    any_ok
}

async fn exchange_for_long_lived_token(
    short_token: &str,
    app_id: &str,
    app_secret: &str,
) -> Option<String> {
    if short_token.trim().is_empty() || app_id.trim().is_empty() || app_secret.trim().is_empty() {
        return None;
    }
    let url = format!(
        "https://graph.facebook.com/{}/oauth/access_token?grant_type=fb_exchange_token&client_id={}&client_secret={}&fb_exchange_token={}",
        crate::META_API_VERSION,
        urlencoding::encode(app_id.trim()),
        urlencoding::encode(app_secret.trim()),
        urlencoding::encode(short_token.trim())
    );
    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    let request = Request::new_with_init(&url, &init).ok()?;
    let mut response = Fetch::Request(request).send().await.ok()?;
    if response.status_code() != 200 {
        return None;
    }
    let body: serde_json::Value = response.json().await.ok()?;
    body.get("access_token")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

async fn exchange_code(
    code: &str,
    app_id: &str,
    app_secret: &str,
    redirect_uri: Option<&str>,
) -> Result<String> {
    let mut url = format!(
        "https://graph.facebook.com/{}/oauth/access_token?client_id={}&client_secret={}&code={}",
        crate::META_API_VERSION,
        urlencoding::encode(app_id),
        urlencoding::encode(app_secret),
        urlencoding::encode(code)
    );
    if let Some(uri) = redirect_uri {
        url.push_str(&format!("&redirect_uri={}", urlencoding::encode(uri)));
    }

    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    let request = Request::new_with_init(&url, &init)?;
    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        let err = response.text().await.unwrap_or_default();
        return Err(Error::from(format!("Token exchange failed: {}", err)));
    }

    let body: serde_json::Value = response.json().await?;
    body.get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::from("Missing access_token in response"))
}

async fn get_phone_number_details(
    waba_id: &str,
    phone_number_id: &str,
    platform_token: &str,
) -> Result<Option<(String, String)>> {
    let url = format!(
        "https://graph.facebook.com/{}/{}?fields=display_phone_number,id&access_token={}",
        crate::META_API_VERSION,
        urlencoding::encode(phone_number_id),
        urlencoding::encode(platform_token)
    );

    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    let request = Request::new_with_init(&url, &init)?;
    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        return Ok(None);
    }

    let body: serde_json::Value = response.json().await?;
    let display = body
        .get("display_phone_number")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let id = body
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let _ = waba_id;

    if id.is_empty() {
        Ok(None)
    } else {
        Ok(Some((display, id)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PhoneCandidate {
    display_phone_number: String,
    id: String,
    is_non_test: bool,
    is_verified_or_connected: bool,
}

fn is_non_test_phone(display_phone_number: &str) -> bool {
    let disp = display_phone_number.trim();
    !disp.starts_with("+1 555") && !disp.starts_with("1555") && !disp.starts_with("+1555")
}

fn pick_best_phone(phones: &[serde_json::Value]) -> Option<PhoneCandidate> {
    let candidates: Vec<PhoneCandidate> = phones
        .iter()
        .rev()
        .filter_map(|p| {
            let id = p
                .get("id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())?
                .to_string();
            let display = p
                .get("display_phone_number")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let status = p
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_ascii_uppercase();
            let code_ver = p
                .get("code_verification_status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_ascii_uppercase();
            let is_non_test = is_non_test_phone(&display);
            let is_verified_or_connected = status == "CONNECTED" || code_ver == "VERIFIED";
            Some(PhoneCandidate {
                display_phone_number: display,
                id,
                is_non_test,
                is_verified_or_connected,
            })
        })
        .collect();

    candidates
        .iter()
        .find(|c| c.is_non_test && c.is_verified_or_connected)
        .or_else(|| candidates.iter().find(|c| c.is_non_test))
        .or_else(|| candidates.first())
        .cloned()
}

async fn fetch_waba_phones(waba_id: &str, token: &str) -> Result<Vec<serde_json::Value>> {
    let url = format!(
        "https://graph.facebook.com/{}/{}/phone_numbers?fields=display_phone_number,id,status,code_verification_status,platform_type&access_token={}",
        crate::META_API_VERSION,
        urlencoding::encode(waba_id),
        urlencoding::encode(token)
    );

    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    let request = Request::new_with_init(&url, &init)?;
    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        let err = response.text().await.unwrap_or_default();
        return Err(Error::from(format!("WABA phone query failed: {}", err)));
    }

    let body: serde_json::Value = response.json().await?;
    Ok(body
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default())
}

async fn fetch_waba_phones_with_fallback(
    waba_id: &str,
    user_token: &str,
    platform_token: &str,
) -> Option<Vec<serde_json::Value>> {
    if !user_token.trim().is_empty() {
        if let Ok(phones) = fetch_waba_phones(waba_id, user_token).await {
            return Some(phones);
        }
    }
    if !platform_token.trim().is_empty() && platform_token.trim() != user_token.trim() {
        if let Ok(phones) = fetch_waba_phones(waba_id, platform_token).await {
            return Some(phones);
        }
    }
    None
}

async fn select_best_waba(
    wabas: &[String],
    target_phone_number_id: Option<&str>,
    user_token: &str,
    platform_token: &str,
) -> (String, Option<(String, String)>) {
    let mut fallback_non_test: Option<(String, Option<(String, String)>)> = None;
    let mut fallback_any: Option<(String, Option<(String, String)>)> = None;

    for waba_id in wabas {
        let Some(phones) =
            fetch_waba_phones_with_fallback(waba_id, user_token, platform_token).await
        else {
            continue;
        };

        if let Some(target_pnid) = target_phone_number_id {
            let matches = phones.iter().any(|p| {
                p.get("id").and_then(|v| v.as_str()).map(str::trim) == Some(target_pnid.trim())
            });
            if matches {
                return (waba_id.clone(), None);
            }
            continue;
        }

        if let Some(best) = pick_best_phone(&phones) {
            let pair = Some((best.display_phone_number.clone(), best.id.clone()));
            if best.is_non_test && best.is_verified_or_connected {
                return (waba_id.clone(), pair);
            }
            if best.is_non_test && fallback_non_test.is_none() {
                fallback_non_test = Some((waba_id.clone(), pair));
            } else if fallback_any.is_none() {
                fallback_any = Some((waba_id.clone(), pair));
            }
        }
    }

    fallback_non_test
        .or(fallback_any)
        .unwrap_or_else(|| (wabas[0].clone(), None))
}

async fn discover_new_phone(
    waba_id: &str,
    platform_token: &str,
) -> Result<Option<(String, String)>> {
    let phones = fetch_waba_phones(waba_id, platform_token).await?;
    Ok(pick_best_phone(&phones).map(|c| (c.display_phone_number, c.id)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_waba_from_debug_token_finds_first_shared_waba() {
        let body = serde_json::json!({
            "data": {
                "app_id": "1734605544890765",
                "is_valid": true,
                "granular_scopes": [
                    {
                        "scope": "public_profile"
                    },
                    {
                        "scope": "whatsapp_business_management",
                        "target_ids": ["1122334455667788", "9988776655443322"]
                    },
                    {
                        "scope": "whatsapp_business_messaging",
                        "target_ids": ["9999999999999999", "1122334455667788"]
                    }
                ]
            }
        });
        assert_eq!(
            extract_waba_from_debug_token(&body),
            Some("1122334455667788".to_string())
        );
        assert_eq!(
            extract_all_wabas_from_debug_token(&body),
            vec![
                "1122334455667788".to_string(),
                "9988776655443322".to_string(),
                "9999999999999999".to_string(),
            ]
        );

        let messaging_only = serde_json::json!({
            "data": {
                "granular_scopes": [
                    {
                        "scope": "whatsapp_business_messaging",
                        "target_ids": ["  556677889900  "]
                    }
                ]
            }
        });
        assert_eq!(
            extract_waba_from_debug_token(&messaging_only),
            Some("556677889900".to_string())
        );

        let empty = serde_json::json!({ "data": { "granular_scopes": [] } });
        assert_eq!(extract_waba_from_debug_token(&empty), None);
    }

    #[test]
    fn extract_waba_from_businesses_finds_owned_client_or_direct_waba() {
        let biz = serde_json::json!({
            "data": [
                {
                    "id": "biz-1",
                    "owned_whatsapp_business_accounts": {
                        "data": [{ "id": "waba-owned-1" }, { "id": "waba-owned-2" }]
                    },
                    "client_whatsapp_business_accounts": {
                        "data": [{ "id": "waba-client-1" }, { "id": "waba-owned-1" }]
                    }
                }
            ]
        });
        assert_eq!(
            extract_waba_from_businesses(&biz),
            Some("waba-owned-1".to_string())
        );
        assert_eq!(
            extract_all_wabas_from_businesses(&biz),
            vec![
                "waba-owned-1".to_string(),
                "waba-owned-2".to_string(),
                "waba-client-1".to_string(),
            ]
        );

        let direct = serde_json::json!({
            "id": "me-1",
            "whatsapp_business_accounts": {
                "data": [{ "id": "waba-direct-1" }]
            }
        });
        assert_eq!(
            extract_waba_from_businesses(&direct),
            Some("waba-direct-1".to_string())
        );
    }

    #[test]
    fn pick_best_phone_prefers_verified_or_connected_non_test_number() {
        let phones = vec![
            serde_json::json!({
                "id": "pn-verified",
                "display_phone_number": "+91 98200 11223",
                "status": "PENDING",
                "code_verification_status": "VERIFIED"
            }),
            serde_json::json!({
                "id": "pn-unverified-newer",
                "display_phone_number": "+91 98200 99999",
                "status": "PENDING",
                "code_verification_status": "NOT_VERIFIED"
            }),
            serde_json::json!({
                "id": "pn-test-connected",
                "display_phone_number": "+1 555 123 4567",
                "status": "CONNECTED",
                "code_verification_status": "VERIFIED"
            }),
        ];
        let best = pick_best_phone(&phones).unwrap();
        assert_eq!(best.id, "pn-verified");
        assert!(best.is_non_test);
        assert!(best.is_verified_or_connected);

        let unverified_only = vec![
            serde_json::json!({
                "id": "pn-test",
                "display_phone_number": "+1 555 000 0000",
                "status": "CONNECTED"
            }),
            serde_json::json!({
                "id": "pn-real",
                "display_phone_number": "+91 98200 99999",
                "status": "PENDING",
                "code_verification_status": "NOT_VERIFIED"
            }),
        ];
        let best_unverified = pick_best_phone(&unverified_only).unwrap();
        assert_eq!(best_unverified.id, "pn-real");
        assert!(best_unverified.is_non_test);
        assert!(!best_unverified.is_verified_or_connected);
    }
}
