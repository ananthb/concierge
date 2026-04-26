//! Discord integration — top-level helpers built on the `botrelay` crate,
//! plus the interaction-dispatch entry point and slash-command / component
//! handlers.

pub mod commands;
pub mod components;
pub mod events;

use botrelay::discord::{
    parse_interaction, ActionRow, Component, CreateMessage, DiscordBot, Embed, EmbedField,
    EmbedFooter, InteractionResponse,
};
use worker::*;

use crate::helpers::generate_id;
use crate::storage;
use crate::types::*;

/// Inbound preview line cap shown on a Discord draft embed.
const PREVIEW_LEN: usize = 500;

/// Construct a `DiscordBot` from worker secrets. Returns `None` if any of the
/// three required secrets are missing or empty.
fn bot_from_env(env: &Env) -> Option<DiscordBot> {
    let token = env.secret("DISCORD_BOT_TOKEN").ok()?.to_string();
    let app_id = env.secret("DISCORD_APPLICATION_ID").ok()?.to_string();
    let public_key = env.secret("DISCORD_PUBLIC_KEY").ok()?.to_string();
    if token.is_empty() || app_id.is_empty() || public_key.is_empty() {
        return None;
    }
    Some(DiscordBot::new(token, app_id, public_key))
}

/// Post a forwarded inbound message to Discord with Reply/Drop buttons.
/// Stores a `ConversationContext` in KV keyed by the generated id so the
/// subsequent button click can look up the origin channel. Returns the posted
/// Discord message id.
pub async fn post_forwarded_message(
    env: &Env,
    msg: &InboundMessage,
    discord_channel_id: &str,
    rule_name: Option<&str>,
) -> Result<String> {
    let bot = bot_from_env(env).ok_or_else(|| Error::from("Discord not configured"))?;
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
        ai_draft: None,
        created_at: crate::helpers::now_iso(),
    };

    let (color, channel_label) = match msg.channel {
        Channel::WhatsApp => (0x25D366u32, "WhatsApp"),
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

    let params = CreateMessage {
        embeds: vec![Embed {
            title: Some(title),
            description: Some(body_preview),
            color: Some(color),
            fields,
            footer,
        }],
        components: vec![ActionRow::new(vec![
            Component::primary_button(format!("reply:{}", ctx.id), "Reply"),
            Component::danger_button(format!("drop:{}", ctx.id), "Drop"),
        ])],
        ..Default::default()
    };

    let message = bot.create_message(discord_channel_id, params).await?;

    let ctx = ConversationContext {
        discord_message_id: message.id.clone(),
        ..ctx
    };
    storage::save_conversation_context(&kv, &ctx).await?;

    Ok(message.id)
}

/// Post an AI-generated draft reply with Approve/Reject buttons. Caller owns
/// the `ConversationContext`: this function only creates the Discord message
/// and returns its id so the caller can stamp it back into the context.
pub async fn post_ai_draft(
    env: &Env,
    ctx: &ConversationContext,
    inbound_subject: Option<&str>,
    inbound_preview: &str,
    queue_reason_label: Option<&str>,
    rule_name: Option<&str>,
) -> Result<String> {
    let bot = bot_from_env(env).ok_or_else(|| Error::from("Discord not configured"))?;

    let draft = ctx
        .ai_draft
        .as_deref()
        .ok_or_else(|| Error::from("post_ai_draft called with no draft"))?;

    let footer_text = match (rule_name, queue_reason_label) {
        (Some(rule), Some(reason)) => Some(format!("Rule: {rule} · Reason: {reason}")),
        (Some(rule), None) => Some(format!("Rule: {rule}")),
        (None, Some(reason)) => Some(format!("Reason: {reason}")),
        (None, None) => None,
    };
    let footer = footer_text.map(|text| EmbedFooter { text });

    let params = CreateMessage {
        content: "**AI Draft Reply**: Approve or reject.".into(),
        embeds: vec![
            Embed {
                title: Some(format!("Re: {}", inbound_subject.unwrap_or("(message)"))),
                description: Some(format!("**Draft:**\n{draft}")),
                color: Some(0x5865F2),
                fields: vec![
                    EmbedField {
                        name: "To".into(),
                        value: ctx.origin_sender.clone(),
                        inline: true,
                    },
                    EmbedField {
                        name: "Channel".into(),
                        value: ctx.origin_channel.as_str().into(),
                        inline: true,
                    },
                ],
                footer,
            },
            Embed {
                title: Some("Original message".into()),
                description: Some(inbound_preview.to_string()),
                color: Some(0x99AAB5),
                ..Default::default()
            },
        ],
        components: vec![ActionRow::new(vec![
            Component::success_button(format!("approve:{}", ctx.id), "Approve"),
            Component::danger_button(format!("reject:{}", ctx.id), "Reject"),
        ])],
        ..Default::default()
    };

    let message = bot.create_message(&ctx.discord_channel_id, params).await?;
    Ok(message.id)
}

/// Build a clamped preview of an inbound message body for embedding into a
/// Discord draft post.
pub fn truncate_inbound_preview(body: &str) -> String {
    if body.len() > PREVIEW_LEN {
        format!("{}...", &body[..PREVIEW_LEN.saturating_sub(3)])
    } else {
        body.to_string()
    }
}

/// Entry point for `POST /discord/interactions`. Verifies the request
/// signature, parses the body, and dispatches to the right handler.
pub async fn handle_interaction(mut req: Request, env: Env) -> Result<Response> {
    let bot = match bot_from_env(&env) {
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

    let interaction = parse_interaction(body.as_bytes())?;

    if interaction.is_ping() {
        return Response::from_json(&InteractionResponse::pong());
    }
    if interaction.is_application_command() {
        return commands::handle_command(&interaction, &env).await;
    }
    if interaction.is_component_click() {
        return components::handle_component(&interaction, &env).await;
    }
    if interaction.is_modal_submit() {
        return components::handle_modal_submit(&interaction, &env).await;
    }

    Response::from_json(&InteractionResponse::ephemeral_message(
        "Unsupported interaction",
    ))
}
