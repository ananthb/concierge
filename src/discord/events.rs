//! Application Webhook Events handler. Receives `MESSAGE_CREATE` events
//! from Discord and routes them through the standard inbound pipeline.
//!
//! The same Ed25519 signature scheme used for interactions applies here.

use serde::Deserialize;
use worker::*;

use crate::pipeline;
use crate::storage::*;
use crate::types::*;

#[derive(Deserialize)]
struct EventPayload {
    #[serde(rename = "type")]
    payload_type: u8,
    #[serde(default)]
    event: Option<EventBody>,
}

#[derive(Deserialize)]
struct EventBody {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct DiscordMessage {
    id: String,
    channel_id: String,
    #[serde(default)]
    guild_id: Option<String>,
    content: String,
    author: DiscordUser,
    #[serde(default)]
    mentions: Vec<DiscordUser>,
    #[serde(default)]
    attachments: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    bot: bool,
}

pub async fn handle_event(mut req: Request, env: Env) -> Result<Response> {
    let bot = match super::bot_from_env(&env) {
        Some(b) => b,
        None => return Response::error("Discord not configured", 503),
    };

    let signature = req
        .headers()
        .get("X-Signature-Ed25519")?
        .unwrap_or_default();
    let timestamp = req
        .headers()
        .get("X-Signature-Timestamp")?
        .unwrap_or_default();
    let body = req.text().await?;

    if !bot
        .verify_interaction(&signature, &timestamp, &body)
        .await?
    {
        return Response::error("Invalid signature", 401);
    }

    let payload: EventPayload = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(_) => return Response::ok(""),
    };

    // Type 0 = PING (Discord verification), respond with same shape.
    if payload.payload_type == 0 {
        return Response::ok("");
    }

    let event = match payload.event {
        Some(e) => e,
        None => return Response::ok(""),
    };

    if event.event_type != "MESSAGE_CREATE" {
        // Other event types (APPLICATION_AUTHORIZED, ENTITLEMENT_CREATE…). Ack and move on.
        return Response::ok("");
    }

    let msg: DiscordMessage = match event.data.and_then(|d| serde_json::from_value(d).ok()) {
        Some(m) => m,
        None => return Response::ok(""),
    };

    // Don't respond to bots (including ourselves).
    if msg.author.bot {
        return Response::ok("");
    }

    // DMs unsupported. Would need per-tenant attribution.
    let guild_id = match msg.guild_id {
        Some(g) => g,
        None => return Response::ok(""),
    };

    let kv = env.kv("KV")?;
    let cfg = match get_discord_config_by_guild(&kv, &guild_id).await? {
        Some(c) => c,
        None => return Response::ok(""),
    };

    // Tenant decides which kinds of messages we react to.
    let app_id = env
        .secret("DISCORD_APPLICATION_ID")
        .map(|s| s.to_string())
        .unwrap_or_default();
    let mentioned_us = msg.mentions.iter().any(|u| u.id == app_id);
    let in_inbound_channel = cfg.inbound_channel_ids.contains(&msg.channel_id);
    let trigger = (mentioned_us && cfg.inbound_mentions) || in_inbound_channel;
    if !trigger || !cfg.auto_reply.enabled {
        return Response::ok("");
    }

    let inbound = InboundMessage {
        id: msg.id.clone(),
        channel: Channel::Discord,
        sender: msg.author.id.clone(),
        sender_name: msg.author.username,
        recipient: app_id,
        body: msg.content,
        subject: None,
        has_attachment: !msg.attachments.is_empty(),
        tenant_id: cfg.tenant_id.clone(),
        channel_account_id: guild_id,
        raw_metadata: serde_json::json!({
            "channel_id": msg.channel_id,
            "message_id": msg.id,
        }),
    };

    if let Err(e) = pipeline::process_inbound(&inbound, &env).await {
        console_log!("Discord inbound pipeline error: {:?}", e);
    }
    Response::ok("")
}
