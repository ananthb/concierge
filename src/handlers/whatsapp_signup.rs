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
    account: SignupAccountView,
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

            let Some(tenant_id) = super::auth::resolve_tenant_id(&req, &db).await else {
                if wants_json {
                    return crate::api::error("unauthenticated", "Sign in to continue.", 401);
                }
                return Response::error("Unauthorized", 401);
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
                    "invalid_state",
                    "Invalid or expired signup state.",
                    400,
                );
            }
            match take_signup_state(&db, &params.state).await? {
                Some(tid) if tid == tenant_id => {}
                _ => {
                    return fail(
                        wants_json,
                        "invalid_state",
                        "Invalid or expired signup state.",
                        400,
                    );
                }
            }

            if params.code.is_empty() {
                return fail(wants_json, "missing_code", "Missing Meta OAuth code.", 400);
            }

            let app_id = env
                .secret("META_APP_ID")
                .map(|s| s.to_string())
                .unwrap_or_default();
            let app_secret = env
                .secret("META_APP_SECRET")
                .map(|s| s.to_string())
                .unwrap_or_default();
            let platform_token = env
                .secret("WHATSAPP_ACCESS_TOKEN")
                .map(|s| s.to_string())
                .unwrap_or_default();
            let fallback_waba_id = env
                .var("WHATSAPP_WABA_ID")
                .map(|v| v.to_string())
                .unwrap_or_default();

            let user_token = match exchange_code(&params.code, &app_id, &app_secret).await {
                Ok(t) => t,
                Err(e) => {
                    console_log!("WhatsApp signup token exchange failed: {:?}", e);
                    return fail(
                        wants_json,
                        "token_exchange_failed",
                        "Could not exchange Meta authorization code.",
                        502,
                    );
                }
            };

            // Best-effort: fetch Meta user id and persist `tenants.facebook_id`
            // so `POST /data-deletion` can locate this tenant for Meta App Review.
            if let Some(fb_id) = fetch_fb_user_id(&user_token).await {
                if let Err(e) = set_tenant_facebook_id(&db, &tenant_id, &fb_id).await {
                    console_log!(
                        "Failed to save facebook_id on tenant {}: {:?}",
                        tenant_id,
                        e
                    );
                }
            }

            let tenant_supplied_waba = params.waba_id.clone().filter(|s| !s.is_empty());
            let effective_waba_id = tenant_supplied_waba.clone().unwrap_or(fallback_waba_id);

            if let Some(ref waba) = tenant_supplied_waba {
                subscribe_waba_app(waba, &platform_token).await;
            }

            let (phone_number, phone_number_id) = if let Some(pnid) = params.phone_number_id {
                match get_phone_number_details(&effective_waba_id, &pnid, &platform_token).await? {
                    Some((display_phone, _)) => (display_phone, pnid),
                    None => {
                        return fail(
                            wants_json,
                            "phone_not_found",
                            "Could not verify that WhatsApp phone number.",
                            400,
                        );
                    }
                }
            } else {
                if effective_waba_id.is_empty() {
                    return fail(
                        wants_json,
                        "missing_waba",
                        "Missing WhatsApp Business Account ID.",
                        400,
                    );
                }
                match discover_new_phone(&effective_waba_id, &platform_token).await? {
                    Some(result) => result,
                    None => {
                        return fail(
                            wants_json,
                            "no_phone_numbers",
                            "No WhatsApp phone numbers found on that account.",
                            400,
                        );
                    }
                }
            };

            if get_whatsapp_account_by_phone(&db, &phone_number_id)
                .await?
                .is_some()
            {
                return fail(
                    wants_json,
                    "already_connected",
                    "That WhatsApp phone number is already connected.",
                    409,
                );
            }

            let now = now_iso();
            let account = WhatsAppAccount {
                id: generate_id(),
                tenant_id,
                name: format!("WhatsApp {}", phone_number),
                phone_number,
                phone_number_id,
                waba_id: if effective_waba_id.is_empty() {
                    None
                } else {
                    Some(effective_waba_id)
                },
                auto_reply: ReplyConfig {
                    enabled: true,
                    response: ReplyResponse::Ai,
                    wait_seconds: 3,
                },
                canned_safety: SafetyState::default(),
                created_at: now.clone(),
                updated_at: now,
            };
            save_whatsapp_account(&db, &account).await?;

            if wants_json {
                crate::api::json(&SignupSuccessView {
                    ok: true,
                    account: SignupAccountView {
                        id: account.id,
                        name: account.name,
                        phone_number: account.phone_number,
                        phone_number_id: account.phone_number_id,
                        waba_id: account.waba_id,
                    },
                })
            } else {
                let headers = Headers::new();
                headers.set("Location", "/wizard")?;
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

fn fail(wants_json: bool, code: &str, message: &str, status: u16) -> Result<Response> {
    if wants_json {
        crate::api::error(code, message, status)
    } else {
        let headers = Headers::new();
        headers.set("Location", &format!("/wizard?error={code}"))?;
        Ok(Response::empty()?.with_status(302).with_headers(headers))
    }
}

async fn fetch_fb_user_id(user_token: &str) -> Option<String> {
    let url = format!(
        "https://graph.facebook.com/{}/me?fields=id&access_token={}",
        crate::META_API_VERSION,
        urlencoding::encode(user_token),
    );
    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    let request = Request::new_with_init(&url, &init).ok()?;
    let mut response = Fetch::Request(request).send().await.ok()?;
    if response.status_code() != 200 {
        return None;
    }
    let body: serde_json::Value = response.json().await.ok()?;
    body.get("id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

async fn subscribe_waba_app(waba_id: &str, platform_token: &str) {
    let url = format!(
        "https://graph.facebook.com/{}/{}/subscribed_apps",
        crate::META_API_VERSION,
        waba_id
    );
    let headers = Headers::new();
    if headers
        .set("Authorization", &format!("Bearer {platform_token}"))
        .is_err()
    {
        return;
    }
    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_headers(headers);
    let Ok(req) = Request::new_with_init(&url, &init) else {
        return;
    };
    match Fetch::Request(req).send().await {
        Ok(resp) if resp.status_code() < 300 => {}
        Ok(mut resp) => {
            let text = resp.text().await.unwrap_or_default();
            console_log!(
                "Warning: subscribed_apps on WABA {} returned HTTP {}: {}",
                waba_id,
                resp.status_code(),
                text
            );
        }
        Err(e) => {
            console_log!(
                "Warning: subscribed_apps on WABA {} failed: {:?}",
                waba_id,
                e
            );
        }
    }
}

async fn exchange_code(code: &str, app_id: &str, app_secret: &str) -> Result<String> {
    let url = format!(
        "https://graph.facebook.com/{}/oauth/access_token?client_id={}&client_secret={}&code={}",
        crate::META_API_VERSION,
        urlencoding::encode(app_id),
        urlencoding::encode(app_secret),
        urlencoding::encode(code)
    );

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

async fn discover_new_phone(
    waba_id: &str,
    platform_token: &str,
) -> Result<Option<(String, String)>> {
    let url = format!(
        "https://graph.facebook.com/{}/{}/phone_numbers?fields=display_phone_number,id&access_token={}",
        crate::META_API_VERSION,
        urlencoding::encode(waba_id),
        urlencoding::encode(platform_token)
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
    let phones = body
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    if let Some(phone) = phones.last() {
        let display = phone
            .get("display_phone_number")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let id = phone
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !id.is_empty() {
            return Ok(Some((display, id)));
        }
    }

    Ok(None)
}
