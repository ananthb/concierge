//! Webhook handlers for incoming WhatsApp and Instagram messages

use worker::*;

use crate::channel;
use crate::helpers::generate_id;
use crate::pipeline;
use crate::storage::*;
use crate::types::*;

/// Handle /webhook/* routes
pub async fn handle_webhook(
    mut req: Request,
    env: Env,
    ctx: &Context,
    path: &str,
    method: Method,
) -> Result<Response> {
    let stripped = path
        .strip_prefix("/api/webhook")
        .or_else(|| path.strip_prefix("/webhooks"))
        .or_else(|| path.strip_prefix("/webhook"))
        .unwrap_or("");
    let path_parts: Vec<&str> = stripped.split('/').filter(|s| !s.is_empty()).collect();

    match (method, path_parts.as_slice()) {
        // Meta webhook verification (GET /webhook/whatsapp or /webhook)
        (Method::Get, []) | (Method::Get, ["whatsapp"]) => {
            let url = req.url()?;
            let query_pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();

            let verify_token = env
                .secret("WHATSAPP_VERIFY_TOKEN")
                .map(|s| s.to_string().trim().to_string())
                .unwrap_or_default();

            let mode = query_pairs.get("hub.mode").map(|s| s.trim().to_string());
            let token = query_pairs
                .get("hub.verify_token")
                .map(|s| s.trim().to_string());
            let challenge = query_pairs
                .get("hub.challenge")
                .map(|s| s.to_string())
                .unwrap_or_default();

            if mode.as_deref() == Some("subscribe")
                && !verify_token.is_empty()
                && token.as_deref() == Some(verify_token.as_str())
            {
                return Response::ok(challenge);
            }

            Response::error("Forbidden", 403)
        }

        // Incoming WhatsApp messages (POST /webhook/whatsapp or /webhook)
        (Method::Post, []) | (Method::Post, ["whatsapp"]) => {
            let sig_header = req
                .headers()
                .get("X-Hub-Signature-256")
                .ok()
                .flatten()
                .unwrap_or_default();
            let body_text = req.text().await?;

            let app_secret = env
                .secret("META_APP_SECRET")
                .map(|s| s.to_string().trim().to_string())
                .unwrap_or_default();

            let db = env.d1("DB")?;

            if app_secret.is_empty()
                || !crate::crypto::verify_meta_signature(
                    &app_secret,
                    body_text.as_bytes(),
                    sig_header.trim(),
                )
            {
                console_log!("WhatsApp webhook signature verification failed");
                let _ = save_message_raw_action(
                    &db,
                    &generate_id(),
                    &Channel::WhatsApp,
                    MessageDirection::Inbound,
                    "webhook",
                    "signature_check",
                    "system",
                    "system",
                    Some("sig_failed"),
                    None,
                )
                .await;
                return Response::error("Unauthorized", 401);
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

            for entry in &body.entry {
                for change in &entry.changes {
                    if change.field != "messages" {
                        continue;
                    }

                    if change.value.messages.is_empty() {
                        continue;
                    }

                    let phone_number_id = &change.value.metadata.phone_number_id;

                    let account_id =
                        match get_whatsapp_account_by_phone(&db, phone_number_id).await? {
                            Some(id) => id,
                            None => match get_whatsapp_account_by_waba(&db, &entry.id).await? {
                                Some(id) => id,
                                None => {
                                    let _ = save_message_raw_action(
                                        &db,
                                        &generate_id(),
                                        &Channel::WhatsApp,
                                        MessageDirection::Inbound,
                                        "webhook",
                                        phone_number_id,
                                        "system",
                                        "unmatched_phone",
                                        Some("unmatched_phone"),
                                        None,
                                    )
                                    .await;
                                    continue;
                                }
                            },
                        };

                    let mut account = match get_whatsapp_account(&db, &account_id).await? {
                        Some(a) => a,
                        None => continue,
                    };

                    if account.phone_number_id != *phone_number_id && !phone_number_id.is_empty() {
                        account.phone_number_id = phone_number_id.clone();
                        let display = change.value.metadata.display_phone_number.trim();
                        if !display.is_empty() {
                            account.phone_number = display.to_string();
                        }
                        let _ = save_whatsapp_account(&db, &account).await;
                    }

                    // Parse into unified messages via channel adapter
                    let messages = channel::whatsapp::parse_inbound(change, &account);

                    // Dispatch each through the unified pipeline asynchronously via wait_until
                    for msg in messages {
                        let env_clone = env.clone();
                        ctx.wait_until(async move {
                            if let Err(e) = pipeline::process_inbound(&msg, &env_clone).await {
                                console_log!("Pipeline error (WhatsApp): {:?}", e);
                            }
                        });
                    }
                }
            }

            Response::ok("OK")
        }

        _ => Response::error("Not Found", 404),
    }
}
