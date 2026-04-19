//! Discord message component (button) and modal submission handlers.
//! Implements the cross-channel relay: Reply button → modal → send back.

use worker::*;

use crate::channel;
use crate::helpers::generate_id;
use crate::storage::*;
use crate::types::*;

/// Handle MESSAGE_COMPONENT interactions (button clicks).
pub async fn handle_component(interaction: &DiscordInteraction, env: &Env) -> Result<Response> {
    let custom_id = interaction
        .data
        .as_ref()
        .and_then(|d| d.custom_id.as_deref())
        .unwrap_or("");

    if let Some(ctx_id) = custom_id.strip_prefix("reply:") {
        return show_reply_modal(ctx_id);
    }

    if let Some(ctx_id) = custom_id.strip_prefix("approve:") {
        return handle_approve(ctx_id, env).await;
    }

    if let Some(ctx_id) = custom_id.strip_prefix("reject:") {
        return handle_reject(ctx_id, env).await;
    }

    if let Some(ctx_id) = custom_id.strip_prefix("drop:") {
        return handle_drop(ctx_id, env).await;
    }

    Response::from_json(&serde_json::json!({
        "type": 4,
        "data": {"content": "Unknown action", "flags": 64}
    }))
}

/// Handle MODAL_SUBMIT interactions (reply text submitted).
pub async fn handle_modal_submit(interaction: &DiscordInteraction, env: &Env) -> Result<Response> {
    let custom_id = interaction
        .data
        .as_ref()
        .and_then(|d| d.custom_id.as_deref())
        .unwrap_or("");

    if let Some(ctx_id) = custom_id.strip_prefix("reply_modal:") {
        let reply_text = extract_modal_text(interaction, "reply_text").unwrap_or_default();

        if reply_text.is_empty() {
            return Response::from_json(&serde_json::json!({
                "type": 4,
                "data": {"content": "Reply cannot be empty.", "flags": 64}
            }));
        }

        return send_relay_reply(ctx_id, &reply_text, env).await;
    }

    Response::from_json(&serde_json::json!({
        "type": 4,
        "data": {"content": "Unknown modal", "flags": 64}
    }))
}

/// Show the reply modal with a text input.
fn show_reply_modal(ctx_id: &str) -> Result<Response> {
    Response::from_json(&serde_json::json!({
        "type": 9,
        "data": {
            "custom_id": format!("reply_modal:{ctx_id}"),
            "title": "Reply",
            "components": [{
                "type": 1,
                "components": [{
                    "type": 4,
                    "custom_id": "reply_text",
                    "label": "Your reply",
                    "style": 2,
                    "required": true,
                    "placeholder": "Type your reply here..."
                }]
            }]
        }
    }))
}

/// Send a relay reply back through the originating channel.
///
/// TODO: This handler performs KV lookup + external API call + D1 write before
/// returning the Discord interaction response. If the total exceeds Discord's
/// 3-second deadline the interaction will fail. Consider switching to a deferred
/// response (type 5 / DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE) followed by a
/// followup message via the interaction webhook once Cloudflare Workers supports
/// reliable post-response execution.
async fn send_relay_reply(ctx_id: &str, reply_text: &str, env: &Env) -> Result<Response> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    let ctx = match get_conversation_context(&kv, ctx_id).await? {
        Some(c) => c,
        None => {
            return Response::from_json(&serde_json::json!({
                "type": 4,
                "data": {"content": "Conversation expired or not found.", "flags": 64}
            }));
        }
    };

    // Send reply via the originating channel
    let subject = if ctx.origin_channel == Channel::Email {
        Some("Re: your message")
    } else {
        None
    };

    match channel::send_reply(
        &ctx.origin_channel,
        env,
        &ctx.reply_metadata,
        &ctx.origin_sender,
        reply_text,
        subject,
    )
    .await
    {
        Ok(()) => {}
        Err(e) => {
            console_log!("Relay reply error: {:?}", e);
            return Response::from_json(&serde_json::json!({
                "type": 4,
                "data": {"content": format!("Failed to send reply: {e}"), "flags": 64}
            }));
        }
    }

    // Log the relay message
    let _ = save_message(
        &db,
        &generate_id(),
        &ctx.origin_channel,
        "relay",
        &ctx.origin_recipient,
        &ctx.origin_sender,
        &ctx.tenant_id,
        &ctx.channel_account_id,
        Some("relay"),
    )
    .await;

    Response::from_json(&serde_json::json!({
        "type": 4,
        "data": {"content": format!("Reply sent to {} via {}.", ctx.origin_sender, ctx.origin_channel.as_str()), "flags": 64}
    }))
}

/// Approve an AI-generated draft and send it.
///
/// TODO: Same 3-second Discord interaction timeout risk as `send_relay_reply`.
/// A deferred response pattern should be used when feasible.
async fn handle_approve(ctx_id: &str, env: &Env) -> Result<Response> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    let ctx = match get_conversation_context(&kv, ctx_id).await? {
        Some(c) => c,
        None => {
            return Response::from_json(&serde_json::json!({
                "type": 4,
                "data": {"content": "Conversation expired.", "flags": 64}
            }));
        }
    };

    let draft = match &ctx.ai_draft {
        Some(d) => d.clone(),
        None => {
            return Response::from_json(&serde_json::json!({
                "type": 4,
                "data": {"content": "No draft found.", "flags": 64}
            }));
        }
    };

    let subject = if ctx.origin_channel == Channel::Email {
        Some("Re: your message")
    } else {
        None
    };

    match channel::send_reply(
        &ctx.origin_channel,
        env,
        &ctx.reply_metadata,
        &ctx.origin_sender,
        &draft,
        subject,
    )
    .await
    {
        Ok(()) => {}
        Err(e) => {
            return Response::from_json(&serde_json::json!({
                "type": 4,
                "data": {"content": format!("Failed to send: {e}"), "flags": 64}
            }));
        }
    }

    // Log
    let _ = save_message(
        &db,
        &generate_id(),
        &ctx.origin_channel,
        "outbound",
        &ctx.origin_recipient,
        &ctx.origin_sender,
        &ctx.tenant_id,
        &ctx.channel_account_id,
        Some("ai_approved"),
    )
    .await;

    // Clean up context
    let _ = delete_conversation_context(&kv, ctx_id).await;

    Response::from_json(&serde_json::json!({
        "type": 4,
        "data": {"content": format!("Approved and sent to {} via {}.", ctx.origin_sender, ctx.origin_channel.as_str()), "flags": 64}
    }))
}

/// Reject an AI draft.
async fn handle_reject(ctx_id: &str, env: &Env) -> Result<Response> {
    let kv = env.kv("KV")?;
    let _ = delete_conversation_context(&kv, ctx_id).await;

    Response::from_json(&serde_json::json!({
        "type": 4,
        "data": {"content": "Draft rejected and discarded.", "flags": 64}
    }))
}

/// Drop/dismiss a forwarded message.
async fn handle_drop(ctx_id: &str, env: &Env) -> Result<Response> {
    let kv = env.kv("KV")?;
    let _ = delete_conversation_context(&kv, ctx_id).await;

    Response::from_json(&serde_json::json!({
        "type": 4,
        "data": {"content": "Message dropped.", "flags": 64}
    }))
}

/// Extract text from a modal submission's components.
fn extract_modal_text(interaction: &DiscordInteraction, custom_id: &str) -> Option<String> {
    interaction
        .data
        .as_ref()?
        .components
        .as_ref()?
        .iter()
        .flat_map(|row| &row.components)
        .find(|c| c.custom_id.as_deref() == Some(custom_id))
        .and_then(|c| c.value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_interaction(custom_id: &str) -> DiscordInteraction {
        DiscordInteraction {
            id: "int-1".into(),
            interaction_type: 3,
            data: Some(InteractionData {
                name: None,
                custom_id: Some(custom_id.into()),
                options: None,
                components: None,
            }),
            message: None,
            member: None,
            token: "tok".into(),
            guild_id: Some("guild-1".into()),
            channel_id: Some("ch-1".into()),
        }
    }

    fn make_modal_interaction(custom_id: &str, field_id: &str, value: &str) -> DiscordInteraction {
        DiscordInteraction {
            id: "int-1".into(),
            interaction_type: 5,
            data: Some(InteractionData {
                name: None,
                custom_id: Some(custom_id.into()),
                options: None,
                components: Some(vec![ActionRow {
                    components: vec![ModalComponent {
                        custom_id: Some(field_id.into()),
                        value: Some(value.into()),
                    }],
                }]),
            }),
            message: None,
            member: None,
            token: "tok".into(),
            guild_id: None,
            channel_id: None,
        }
    }

    #[test]
    fn extract_modal_text_found() {
        let interaction = make_modal_interaction("reply_modal:ctx-1", "reply_text", "Hello back!");
        let text = extract_modal_text(&interaction, "reply_text");
        assert_eq!(text.as_deref(), Some("Hello back!"));
    }

    #[test]
    fn extract_modal_text_wrong_id() {
        let interaction = make_modal_interaction("reply_modal:ctx-1", "reply_text", "Hello");
        let text = extract_modal_text(&interaction, "wrong_id");
        assert!(text.is_none());
    }

    #[test]
    fn extract_modal_text_no_components() {
        let interaction = make_interaction("reply:ctx-1");
        let text = extract_modal_text(&interaction, "reply_text");
        assert!(text.is_none());
    }

    #[test]
    fn custom_id_prefix_parsing() {
        // Verify the prefix stripping used in handle_component
        let id = "reply:abc-123-def";
        assert_eq!(id.strip_prefix("reply:"), Some("abc-123-def"));

        let id = "approve:xyz";
        assert_eq!(id.strip_prefix("approve:"), Some("xyz"));

        let id = "reject:xyz";
        assert_eq!(id.strip_prefix("reject:"), Some("xyz"));

        let id = "drop:xyz";
        assert_eq!(id.strip_prefix("drop:"), Some("xyz"));

        // Non-matching prefix
        let id = "unknown:xyz";
        assert!(id.strip_prefix("reply:").is_none());
    }
}
