//! Webhook handlers for incoming WhatsApp and Instagram messages

use worker::*;

use crate::channel;
use crate::pipeline;
use crate::storage::*;
use crate::types::*;

/// Handle /webhook/* routes
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
                .secret("META_APP_SECRET")
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

            let kv = env.kv("KV")?;

            for entry in &body.entry {
                for change in &entry.changes {
                    if change.field != "messages" {
                        continue;
                    }

                    let phone_number_id = &change.value.metadata.phone_number_id;

                    let account_id =
                        match get_whatsapp_account_by_phone(&kv, phone_number_id).await? {
                            Some(id) => id,
                            None => continue,
                        };

                    let account = match get_whatsapp_account(&kv, &account_id).await? {
                        Some(a) => a,
                        None => continue,
                    };

                    // Parse into unified messages via channel adapter
                    let messages = channel::whatsapp::parse_inbound(change, &account);

                    // Process each through the unified pipeline
                    for msg in &messages {
                        if let Err(e) = pipeline::process_inbound(msg, &env).await {
                            console_log!("Pipeline error (WhatsApp): {:?}", e);
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
