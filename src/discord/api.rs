//! Discord REST API helpers — post messages, embeds, buttons.

use serde::Serialize;
use worker::*;

use crate::helpers::generate_id;
use crate::storage;
use crate::types::*;

const DISCORD_API: &str = "https://discord.com/api/v10";

/// Post a forwarded message to Discord with Reply/Drop buttons.
/// Creates a ConversationContext for relay and returns the Discord message ID.
pub async fn post_forwarded_message(
    env: &Env,
    msg: &InboundMessage,
    discord_channel_id: &str,
    rule_name: Option<&str>,
) -> Result<String> {
    let bot_token = env
        .secret("DISCORD_BOT_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_default();

    let kv = env.kv("KV")?;

    // Create conversation context for relay
    let ctx = ConversationContext {
        id: generate_id(),
        discord_message_id: String::new(), // Updated after post
        discord_channel_id: discord_channel_id.to_string(),
        origin_channel: msg.channel.clone(),
        origin_sender: msg.sender.clone(),
        origin_recipient: msg.recipient.clone(),
        tenant_id: msg.tenant_id.clone(),
        channel_account_id: msg.channel_account_id.clone(),
        reply_metadata: msg.raw_metadata.clone(),
        ai_draft: None,
        created_at: crate::helpers::now_iso(),
    };

    let (color, channel_label) = match msg.channel {
        Channel::WhatsApp => (0x25D366, "WhatsApp"),
        Channel::Instagram => (0xE4405F, "Instagram"),
        Channel::Email => (0xF38020, "Email"),
        Channel::Discord => (0x5865F2, "Discord"),
    };

    let body_preview = if msg.body.len() > 1000 {
        format!("{}...", &msg.body[..997])
    } else {
        msg.body.clone()
    };

    let mut fields = vec![
        EmbedField {
            name: "From".into(),
            value: format!(
                "{}{}",
                msg.sender,
                msg.sender_name
                    .as_ref()
                    .map(|n| format!(" ({n})"))
                    .unwrap_or_default()
            ),
            inline: true,
        },
        EmbedField {
            name: "Channel".into(),
            value: channel_label.to_string(),
            inline: true,
        },
    ];

    if let Some(ref subj) = msg.subject {
        fields.push(EmbedField {
            name: "Subject".into(),
            value: subj.clone(),
            inline: false,
        });
    }

    let footer = rule_name.map(|n| EmbedFooter {
        text: format!("Rule: {n}"),
    });

    let title = msg
        .subject
        .clone()
        .unwrap_or_else(|| format!("New message from {}", msg.sender));

    let payload = serde_json::json!({
        "embeds": [{
            "title": title,
            "description": body_preview,
            "color": color,
            "fields": fields,
            "footer": footer,
        }],
        "components": [{
            "type": 1,
            "components": [
                {
                    "type": 2,
                    "style": 1,
                    "label": "Reply",
                    "custom_id": format!("reply:{}", ctx.id)
                },
                {
                    "type": 2,
                    "style": 4,
                    "label": "Drop",
                    "custom_id": format!("drop:{}", ctx.id)
                }
            ]
        }]
    });

    let discord_msg_id = post_to_discord(&bot_token, discord_channel_id, &payload).await?;

    // Save context with the discord message ID
    let ctx = ConversationContext {
        discord_message_id: discord_msg_id.clone(),
        ..ctx
    };
    storage::save_conversation_context(&kv, &ctx).await?;

    Ok(discord_msg_id)
}

/// Post an AI draft to Discord for approval.
pub async fn post_ai_draft(
    env: &Env,
    msg: &InboundMessage,
    discord_channel_id: &str,
    draft: &str,
    rule_name: Option<&str>,
) -> Result<()> {
    let bot_token = env
        .secret("DISCORD_BOT_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_default();

    let kv = env.kv("KV")?;

    let ctx = ConversationContext {
        id: generate_id(),
        discord_message_id: String::new(),
        discord_channel_id: discord_channel_id.to_string(),
        origin_channel: msg.channel.clone(),
        origin_sender: msg.sender.clone(),
        origin_recipient: msg.recipient.clone(),
        tenant_id: msg.tenant_id.clone(),
        channel_account_id: msg.channel_account_id.clone(),
        reply_metadata: msg.raw_metadata.clone(),
        ai_draft: Some(draft.to_string()),
        created_at: crate::helpers::now_iso(),
    };

    let original_preview = if msg.body.len() > 500 {
        format!("{}...", &msg.body[..497])
    } else {
        msg.body.clone()
    };

    let footer = rule_name.map(|n| EmbedFooter {
        text: format!("Rule: {n}"),
    });

    let payload = serde_json::json!({
        "content": "**AI Draft Reply**: Approve or reject.",
        "embeds": [
            {
                "title": format!("Re: {}", msg.subject.as_deref().unwrap_or("(message)")),
                "description": format!("**Draft:**\n{draft}"),
                "color": 0x5865F2,
                "fields": [
                    {"name": "To", "value": &msg.sender, "inline": true},
                    {"name": "Channel", "value": msg.channel.as_str(), "inline": true},
                ],
                "footer": footer,
            },
            {
                "title": "Original message",
                "description": original_preview,
                "color": 0x99AAB5,
            }
        ],
        "components": [{
            "type": 1,
            "components": [
                {
                    "type": 2,
                    "style": 3,
                    "label": "Approve",
                    "custom_id": format!("approve:{}", ctx.id)
                },
                {
                    "type": 2,
                    "style": 4,
                    "label": "Reject",
                    "custom_id": format!("reject:{}", ctx.id)
                }
            ]
        }]
    });

    let discord_msg_id = post_to_discord(&bot_token, discord_channel_id, &payload).await?;

    let ctx = ConversationContext {
        discord_message_id: discord_msg_id,
        ..ctx
    };
    storage::save_conversation_context(&kv, &ctx).await?;

    Ok(())
}

/// Low-level: POST a message to a Discord channel. Returns the message ID.
async fn post_to_discord(
    bot_token: &str,
    channel_id: &str,
    payload: &serde_json::Value,
) -> Result<String> {
    let url = format!("{DISCORD_API}/channels/{channel_id}/messages");
    let body =
        serde_json::to_string(payload).map_err(|e| Error::from(format!("JSON error: {e}")))?;

    let headers = Headers::new();
    headers.set("Authorization", &format!("Bot {bot_token}"))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&body)));

    let request = Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(request).send().await?;

    if resp.status_code() >= 400 {
        let err = resp.text().await.unwrap_or_default();
        console_log!("Discord API error: {err}");
        return Err(Error::from(format!(
            "Discord API error: {}",
            resp.status_code()
        )));
    }

    let response: serde_json::Value = resp.json().await?;
    let msg_id = response
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(msg_id)
}

// Serialization helpers for embeds

#[derive(Serialize)]
struct EmbedField {
    name: String,
    value: String,
    inline: bool,
}

#[derive(Serialize)]
struct EmbedFooter {
    text: String,
}
