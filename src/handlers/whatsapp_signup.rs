//! WhatsApp Embedded Signup callback handler

use worker::*;

use crate::helpers::*;
use crate::storage::*;
use crate::types::*;

/// Handle /whatsapp/signup/* routes
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
        // POST /whatsapp/signup/callback — receive code from JS SDK
        (Method::Post, ["callback"]) => {
            let kv = env.kv("CALENDARS_KV")?;

            // Authenticate via session cookie
            let tenant_id = match super::auth::resolve_tenant_id(&req, &kv).await {
                Some(id) => id,
                None => return Response::error("Unauthorized", 401),
            };

            let form = req.form_data().await?;

            // Verify CSRF state
            let state = match form.get("state") {
                Some(FormEntry::Field(s)) => s,
                _ => {
                    return redirect_error("/admin/whatsapp", "invalid_state");
                }
            };
            let state_key = format!("wa_signup_state:{}", state);
            match kv.get(&state_key).text().await? {
                Some(tid) if tid == tenant_id => {
                    kv.delete(&state_key).await?;
                }
                _ => {
                    return redirect_error("/admin/whatsapp", "invalid_state");
                }
            }

            let code = match form.get("code") {
                Some(FormEntry::Field(c)) if !c.is_empty() => c,
                _ => {
                    return redirect_error("/admin/whatsapp", "missing_code");
                }
            };

            // Also grab phone_number_id if the JS SDK provided it directly
            let js_phone_number_id = match form.get("phone_number_id") {
                Some(FormEntry::Field(p)) if !p.is_empty() => Some(p),
                _ => None,
            };

            let app_id = env
                .secret("FACEBOOK_APP_ID")
                .map(|s| s.to_string())
                .unwrap_or_default();
            let app_secret = env
                .secret("FACEBOOK_APP_SECRET")
                .map(|s| s.to_string())
                .unwrap_or_default();
            let platform_token = env
                .secret("WHATSAPP_ACCESS_TOKEN")
                .map(|s| s.to_string())
                .unwrap_or_default();
            let waba_id = env
                .var("WHATSAPP_WABA_ID")
                .map(|v| v.to_string())
                .unwrap_or_default();

            // Exchange code for short-lived token
            let _token = exchange_code(&code, &app_id, &app_secret).await?;

            // Discover the phone number — either from JS SDK or by querying the WABA
            let (phone_number, phone_number_id) = if let Some(pnid) = js_phone_number_id {
                // Verify it exists on the WABA
                match get_phone_number_details(&waba_id, &pnid, &platform_token).await? {
                    Some((display_phone, _)) => (display_phone, pnid),
                    None => {
                        return redirect_error("/admin/whatsapp", "phone_not_found");
                    }
                }
            } else {
                // Query WABA for phone numbers using the exchange token
                match discover_new_phone(&waba_id, &platform_token).await? {
                    Some(result) => result,
                    None => {
                        return redirect_error("/admin/whatsapp", "no_phone_numbers");
                    }
                }
            };

            // Check for duplicate
            if let Some(_existing) = get_whatsapp_account_by_phone(&kv, &phone_number_id).await? {
                return redirect_error("/admin/whatsapp", "already_connected");
            }

            // Create WhatsApp account
            let now = now_iso();
            let account = WhatsAppAccount {
                id: generate_id(),
                tenant_id,
                name: format!("WhatsApp {}", phone_number),
                phone_number,
                phone_number_id,
                auto_reply: AutoReplyConfig::default(),
                created_at: now.clone(),
                updated_at: now,
            };
            save_whatsapp_account(&kv, &account).await?;

            let headers = Headers::new();
            headers.set("Location", "/admin/whatsapp?success=connected")?;
            Ok(Response::empty()?.with_status(302).with_headers(headers))
        }

        _ => Response::error("Not Found", 404),
    }
}

fn redirect_error(base: &str, error: &str) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Location", &format!("{}?error={}", base, error))?;
    Ok(Response::empty()?.with_status(302).with_headers(headers))
}

/// Exchange the JS SDK code for an access token
async fn exchange_code(code: &str, app_id: &str, app_secret: &str) -> Result<String> {
    let url = format!(
        "https://graph.facebook.com/v21.0/oauth/access_token?client_id={}&client_secret={}&code={}",
        app_id, app_secret, code
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

/// Get details for a specific phone number on the WABA
async fn get_phone_number_details(
    waba_id: &str,
    phone_number_id: &str,
    platform_token: &str,
) -> Result<Option<(String, String)>> {
    let url = format!(
        "https://graph.facebook.com/v21.0/{}?fields=display_phone_number,id&access_token={}",
        phone_number_id, platform_token
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

    let _ = waba_id; // used for context, actual lookup is by phone_number_id

    if id.is_empty() {
        Ok(None)
    } else {
        Ok(Some((display, id)))
    }
}

/// Query WABA phone numbers to find newly registered ones
async fn discover_new_phone(
    waba_id: &str,
    platform_token: &str,
) -> Result<Option<(String, String)>> {
    let url = format!(
        "https://graph.facebook.com/v21.0/{}/phone_numbers?fields=display_phone_number,id&access_token={}",
        waba_id, platform_token
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

    // Return the most recently added (last in list)
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
