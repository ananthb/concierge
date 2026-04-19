//! Instagram DM webhook handler

use worker::*;

use crate::channel;
use crate::pipeline;
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
        .secret("META_APP_SECRET")
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

    let kv = env.kv("KV")?;

    for entry in &payload.entry {
        let page_id = &entry.id;

        let account_id = match get_instagram_account_by_page(&kv, page_id).await? {
            Some(id) => id,
            None => continue,
        };

        let account = match get_instagram_account(&kv, &account_id).await? {
            Some(a) => a,
            None => continue,
        };

        if !account.enabled {
            continue;
        }

        for messaging in &entry.messaging {
            // Parse into unified message via channel adapter
            let msg = match channel::instagram::parse_inbound(messaging, &account, page_id) {
                Some(m) => m,
                None => continue,
            };

            // Process through the unified pipeline
            if let Err(e) = pipeline::process_inbound(&msg, &env).await {
                console_log!("Pipeline error (Instagram): {:?}", e);
            }
        }
    }

    Response::ok("OK")
}
