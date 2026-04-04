//! Instagram DM webhook handler

use worker::*;

use crate::ai;
use crate::crypto;
use crate::helpers::*;
use crate::instagram;
use crate::storage::*;
use crate::types::*;

/// Handle GET /webhook/instagram — Meta verification challenge
pub fn handle_instagram_verify(req: &Request, env: &Env) -> Result<Response> {
    let url = req.url()?;
    let query_pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();

    let verify_token = env
        .secret("INSTAGRAM_VERIFY_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_default();

    let mode = query_pairs.get("hub.mode").map(|s| s.to_string());
    let token = query_pairs.get("hub.verify_token").map(|s| s.to_string());
    let challenge = query_pairs
        .get("hub.challenge")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if mode.as_deref() == Some("subscribe") && token.as_deref() == Some(&verify_token) {
        return Response::ok(challenge);
    }

    Response::error("Forbidden", 403)
}

/// Handle POST /webhook/instagram — incoming DM
pub async fn handle_instagram_dm(mut req: Request, env: Env) -> Result<Response> {
    let sig_header = req
        .headers()
        .get("X-Hub-Signature-256")
        .ok()
        .flatten()
        .unwrap_or_default();
    let body_text = req.text().await?;

    let app_secret = env
        .secret("FACEBOOK_APP_SECRET")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if !app_secret.is_empty()
        && !crate::crypto::verify_meta_signature(&app_secret, body_text.as_bytes(), &sig_header)
    {
        console_log!("Invalid Instagram webhook signature");
        return Response::ok("OK");
    }

    let payload: InstagramWebhookPayload = match serde_json::from_str(&body_text) {
        Ok(p) => p,
        Err(e) => {
            console_log!("Failed to parse Instagram webhook: {:?}", e);
            return Response::ok("OK");
        }
    };

    if payload.object != "instagram" {
        return Response::ok("OK");
    }

    let kv = env.kv("CALENDARS_KV")?;
    let db = env.d1("DB")?;
    let encryption_key = env
        .secret("ENCRYPTION_KEY")
        .map(|s| s.to_string())
        .unwrap_or_default();

    for entry in &payload.entry {
        let page_id = &entry.id;

        // Look up Instagram account by page_id
        let account_id = match get_instagram_account_by_page(&kv, page_id).await? {
            Some(id) => id,
            None => {
                console_log!("No Instagram account found for page_id: {}", page_id);
                continue;
            }
        };

        let account = match get_instagram_account(&kv, &account_id).await? {
            Some(a) => a,
            None => continue,
        };

        if !account.enabled || !account.auto_reply.enabled {
            continue;
        }

        // Load page token
        let token_key = format!("instagram_token:{}", account.id);
        let encrypted_token = match kv.get(&token_key).text().await? {
            Some(t) => t,
            None => continue,
        };

        let token = match crypto::decrypt_token(&encrypted_token, &encryption_key).await {
            Ok(t) => t,
            Err(e) => {
                console_log!(
                    "Failed to decrypt Instagram token for {}: {:?}",
                    account.id,
                    e
                );
                continue;
            }
        };

        if instagram::token_is_expired(&token) {
            continue;
        }

        for messaging in &entry.messaging {
            let sender_id = &messaging.sender.id;

            // Skip messages from ourselves
            if sender_id == page_id {
                continue;
            }

            let dm = match &messaging.message {
                Some(dm) => dm,
                None => continue,
            };

            let text = match &dm.text {
                Some(t) => t.clone(),
                None => continue,
            };

            // Log inbound message
            if let Err(e) = save_instagram_message(
                &db,
                &generate_id(),
                &account_id,
                "inbound",
                sender_id,
                page_id,
                &text,
                &account.tenant_id,
            )
            .await
            {
                console_log!("Failed to save message: {:?}", e);
            }

            // Generate reply
            let reply = match account.auto_reply.mode {
                AutoReplyMode::Static => account.auto_reply.prompt.clone(),
                AutoReplyMode::Ai => {
                    let mut context = serde_json::Map::new();
                    context.insert(
                        "message".to_string(),
                        serde_json::Value::String(text.clone()),
                    );
                    match ai::generate_response(&env, &account.auto_reply.prompt, &context).await {
                        Ok(r) => r,
                        Err(e) => {
                            console_log!("AI Instagram reply error: {:?}", e);
                            continue;
                        }
                    }
                }
            };

            if !reply.is_empty() {
                if let Err(e) =
                    instagram::send_instagram_dm(&token.access_token, page_id, sender_id, &reply)
                        .await
                {
                    console_log!("Instagram DM send error: {:?}", e);
                }

                // Log outbound
                if let Err(e) = save_instagram_message(
                    &db,
                    &generate_id(),
                    &account_id,
                    "outbound",
                    page_id,
                    sender_id,
                    &reply,
                    &account.tenant_id,
                )
                .await
                {
                    console_log!("Failed to save message: {:?}", e);
                }
            }
        }
    }

    Response::ok("OK")
}
