//! Webhook handlers for incoming WhatsApp messages

use worker::*;

use crate::ai;
use crate::helpers::*;
use crate::storage::*;
use crate::types::*;
use crate::whatsapp::send_whatsapp_message;

/// Handle /webhook/whatsapp routes
pub async fn handle_webhook(
    mut req: Request,
    env: Env,
    path: &str,
    method: Method,
) -> Result<Response> {
    let path_parts: Vec<&str> = path
        .strip_prefix("/webhook/")
        .unwrap_or("")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    match (method, path_parts.as_slice()) {
        // Meta webhook verification (GET /webhook/whatsapp)
        (Method::Get, ["whatsapp"]) => {
            let url = req.url()?;
            let query_pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();

            let verify_token = env
                .secret("WHATSAPP_VERIFY_TOKEN")
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

        // Instagram webhook verification (GET /webhook/instagram)
        (Method::Get, ["instagram"]) => {
            super::instagram_webhook::handle_instagram_verify(&req, &env)
        }

        // Incoming WhatsApp messages (POST /webhook/whatsapp)
        (Method::Post, ["whatsapp"]) => {
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
                && !crate::crypto::verify_meta_signature(
                    &app_secret,
                    body_text.as_bytes(),
                    &sig_header,
                )
            {
                console_log!("Invalid WhatsApp webhook signature");
                return Response::ok("OK");
            }

            let body: WhatsAppWebhook = match serde_json::from_str(&body_text) {
                Ok(b) => b,
                Err(e) => {
                    console_log!("Failed to parse webhook body: {:?}", e);
                    return Response::ok("OK");
                }
            };

            if body.object != "whatsapp_business_account" {
                return Response::ok("OK");
            }

            let kv = env.kv("CALENDARS_KV")?;
            let db = env.d1("DB")?;
            let platform_token = env
                .secret("WHATSAPP_ACCESS_TOKEN")
                .map(|s| s.to_string())
                .unwrap_or_default();

            for entry in &body.entry {
                for change in &entry.changes {
                    if change.field != "messages" {
                        continue;
                    }

                    let phone_number_id = &change.value.metadata.phone_number_id;

                    // Look up WhatsApp account by phone_number_id
                    let account_id =
                        match get_whatsapp_account_by_phone(&kv, phone_number_id).await? {
                            Some(id) => id,
                            None => {
                                console_log!(
                                    "No WhatsApp account found for phone_number_id: {}",
                                    phone_number_id
                                );
                                continue;
                            }
                        };

                    let account = match get_whatsapp_account(&kv, &account_id).await? {
                        Some(a) => a,
                        None => continue,
                    };

                    let contacts = &change.value.contacts;
                    for msg in &change.value.messages {
                        let sender_name = contacts
                            .iter()
                            .find(|c| c.wa_id == msg.from)
                            .map(|c| c.profile.name.clone())
                            .unwrap_or_default();

                        let text = match &msg.text {
                            Some(t) => t.body.clone(),
                            None => continue,
                        };

                        let incoming = IncomingMessage {
                            from: msg.from.clone(),
                            sender_name,
                            text: text.clone(),
                            message_id: msg.id.clone(),
                            timestamp: msg.timestamp.clone(),
                        };

                        // Log inbound message
                        if let Err(e) = save_whatsapp_message(
                            &db,
                            &generate_id(),
                            &account_id,
                            "inbound",
                            &incoming.from,
                            phone_number_id,
                            &incoming.text,
                            &account.tenant_id,
                        )
                        .await
                        {
                            console_log!("Failed to save message: {:?}", e);
                        }

                        // Handle auto-reply
                        if account.auto_reply.enabled {
                            let reply = match account.auto_reply.mode {
                                AutoReplyMode::Static => account.auto_reply.prompt.clone(),
                                AutoReplyMode::Ai => {
                                    let mut context = serde_json::Map::new();
                                    context.insert(
                                        "sender_name".to_string(),
                                        serde_json::Value::String(incoming.sender_name.clone()),
                                    );
                                    context.insert(
                                        "message".to_string(),
                                        serde_json::Value::String(incoming.text.clone()),
                                    );
                                    match ai::generate_response(
                                        &env,
                                        &account.auto_reply.prompt,
                                        &context,
                                    )
                                    .await
                                    {
                                        Ok(r) => r,
                                        Err(e) => {
                                            console_log!("AI auto-reply error: {:?}", e);
                                            continue;
                                        }
                                    }
                                }
                            };

                            if !reply.is_empty() {
                                if let Err(e) = send_whatsapp_message(
                                    &platform_token,
                                    &account.phone_number_id,
                                    &incoming.from,
                                    &reply,
                                )
                                .await
                                {
                                    console_log!("Auto-reply send error: {:?}", e);
                                }

                                // Log outbound message
                                if let Err(e) = save_whatsapp_message(
                                    &db,
                                    &generate_id(),
                                    &account_id,
                                    "outbound",
                                    phone_number_id,
                                    &incoming.from,
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
                }
            }

            Response::ok("OK")
        }

        // Incoming Instagram DMs (POST /webhook/instagram)
        (Method::Post, ["instagram"]) => {
            super::instagram_webhook::handle_instagram_dm(req, env).await
        }

        _ => Response::error("Not Found", 404),
    }
}
