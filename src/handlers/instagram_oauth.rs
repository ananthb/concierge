//! Instagram OAuth handlers (Facebook Login flow)

use worker::*;

use super::get_base_url;
use crate::crypto;
use crate::helpers::*;
use crate::instagram;
use crate::storage::*;
use crate::types::*;

/// Handle Instagram OAuth routes (/instagram/*)
pub async fn handle_instagram(
    req: Request,
    env: Env,
    path: &str,
    method: Method,
) -> Result<Response> {
    let base_url = get_base_url(&req);
    let kv = env.kv("KV")?;

    let app_id = env
        .secret("META_APP_ID")
        .map(|s| s.to_string())
        .unwrap_or_default();
    let app_secret = env
        .secret("META_APP_SECRET")
        .map(|s| s.to_string())
        .unwrap_or_default();
    let encryption_key = env
        .secret("ENCRYPTION_KEY")
        .map(|s| s.to_string())
        .unwrap_or_default();
    if encryption_key.is_empty() {
        return Response::error("Server misconfiguration: encryption key not set", 500);
    }

    let path_parts: Vec<&str> = path
        .strip_prefix("/instagram/")
        .unwrap_or("")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    match (method, path_parts.as_slice()) {
        // Start OAuth flow: state carries tenant_id
        (Method::Get, ["auth", tenant_id]) => {
            if app_id.is_empty() || app_secret.is_empty() {
                return Response::error("Instagram integration not configured", 500);
            }

            let state = generate_token()?;
            let redirect_uri = format!("{}/instagram/callback", base_url);

            kv.put(&format!("instagram_oauth_state:{}", state), *tenant_id)?
                .expiration_ttl(600)
                .execute()
                .await?;

            let auth_url = instagram::get_auth_url(&app_id, &redirect_uri, &state);

            let headers = Headers::new();
            headers.set("Location", &auth_url)?;
            Ok(Response::empty()?.with_status(302).with_headers(headers))
        }

        (Method::Get, ["callback"]) => {
            let url = req.url()?;
            let query_pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();

            if let Some(error) = query_pairs.get("error") {
                return Response::error(format!("Instagram error: {}", error), 400);
            }

            let code = query_pairs
                .get("code")
                .ok_or_else(|| Error::from("Missing authorization code"))?;
            let state = query_pairs
                .get("state")
                .ok_or_else(|| Error::from("Missing state parameter"))?;

            let state_key = format!("instagram_oauth_state:{}", state);
            let tenant_id = kv
                .get(&state_key)
                .text()
                .await?
                .ok_or_else(|| Error::from("Invalid or expired state"))?;

            kv.delete(&state_key).await?;

            let redirect_uri = format!("{}/instagram/callback", base_url);
            let short_token =
                instagram::exchange_code_for_token(code, &app_id, &app_secret, &redirect_uri)
                    .await?;

            // Exchange for long-lived token
            let long_lived =
                instagram::exchange_for_long_lived_token(&short_token, &app_id, &app_secret)
                    .await?;

            // Get user's Pages
            let pages = instagram::get_user_pages(&long_lived.access_token).await?;

            if pages.is_empty() {
                let headers = Headers::new();
                headers.set(
                    "Location",
                    &format!("{}/admin/instagram?error=no_pages", base_url),
                )?;
                return Ok(Response::empty()?.with_status(302).with_headers(headers));
            }

            // Find first page with an Instagram business account
            let mut found_ig = None;
            let mut page_token = String::new();
            let mut page_id = String::new();

            for (pid, _name, ptoken) in &pages {
                if let Ok(Some((ig_id, ig_username))) =
                    instagram::get_instagram_business_account(pid, ptoken).await
                {
                    found_ig = Some((ig_id, ig_username));
                    page_token = ptoken.clone();
                    page_id = pid.clone();
                    break;
                }
            }

            let (ig_user_id, ig_username) = match found_ig {
                Some(ig) => ig,
                None => {
                    let headers = Headers::new();
                    headers.set(
                        "Location",
                        &format!("{}/admin/instagram?error=no_ig_account", base_url),
                    )?;
                    return Ok(Response::empty()?.with_status(302).with_headers(headers));
                }
            };

            // Check for duplicate
            let existing_accounts = list_instagram_accounts(&kv, &tenant_id).await?;
            if existing_accounts
                .iter()
                .any(|a| a.instagram_user_id == ig_user_id)
            {
                let headers = Headers::new();
                headers.set(
                    "Location",
                    &format!("{}/admin/instagram?error=already_connected", base_url),
                )?;
                return Ok(Response::empty()?.with_status(302).with_headers(headers));
            }

            // Create InstagramAccount resource
            let account_id = generate_id();
            let now = now_iso();
            let account = InstagramAccount {
                id: account_id.clone(),
                tenant_id: tenant_id.clone(),
                instagram_user_id: ig_user_id,
                instagram_username: ig_username,
                page_id,
                auto_reply: AutoReplyConfig::default(),
                enabled: true,
                created_at: now.clone(),
                updated_at: now,
            };
            save_instagram_account(&kv, &account).await?;

            // Store encrypted page token
            let token = InstagramToken {
                access_token: page_token,
                expires_at: long_lived.expires_at,
                user_id: account.instagram_user_id.clone(),
            };
            let encrypted = crypto::encrypt_token(&token, &encryption_key).await?;
            kv.put(&format!("instagram_token:{}", account_id), encrypted)?
                .execute()
                .await?;

            // If the user is mid-wizard, send them back to the channels step.
            let dest = match get_onboarding(&kv, &account.tenant_id).await {
                Ok(s) if !s.completed => format!("{}/admin/wizard/channels", base_url),
                _ => format!("{}/admin/instagram?success=connected", base_url),
            };
            let headers = Headers::new();
            headers.set("Location", &dest)?;
            Ok(Response::empty()?.with_status(302).with_headers(headers))
        }

        _ => Response::error("Not Found", 404),
    }
}
