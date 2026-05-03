//! Human-handoff notifications.
//!
//! When the AI emits the handoff sentinel on a customer turn, the
//! pipeline strips the token, switches the conversation into the
//! holding-pattern path, and calls [`notify_human_requested`] exactly
//! once. This module owns that one-shot fan-out: it reads the tenant's
//! existing [`NotificationConfig`] (the same config that powers the
//! approval queue) and dispatches a Discord embed and/or an immediate
//! email, whichever the tenant has enabled.
//!
//! Failures are logged but not propagated. A handoff alert that
//! doesn't reach the tenant is unfortunate; an alert that errors and
//! takes the whole inbound pipeline down is worse.
//!
//! Idempotency: callers MUST guard with `HandoffState::notified`. This
//! module fans out unconditionally.

use botrelay::discord::{CreateMessage, DiscordBot, Embed, EmbedField, EmbedFooter};
use worker::*;

use crate::email::send::{send_outbound, OutboundEmail};
use crate::storage::{get_discord_config_by_tenant, get_onboarding, get_tenant};
use crate::types::Channel;

const PREVIEW_LEN: usize = 200;
const HANDOFF_COLOR: u32 = 0xF19E1C; // Concierge accent, warmer than Discord red.

/// One-shot notification: a customer message just tripped a handoff.
///
/// Reads the tenant's onboarding state for its `NotificationConfig`
/// (Discord on/off, email on/off) and dispatches the matching
/// channels. Customer excerpt is clamped to [`PREVIEW_LEN`] so the
/// embed/email don't balloon.
pub async fn notify_human_requested(
    env: &Env,
    db: &D1Database,
    tenant_id: &str,
    inbound_channel: &Channel,
    customer_sender: &str,
    customer_excerpt: &str,
) -> Result<()> {
    let kv = env.kv("KV")?;
    let onboarding = get_onboarding(&kv, tenant_id).await?;
    let notif = onboarding.notifications;

    let preview = clamp_preview(customer_excerpt);

    if notif.approval_discord {
        if let Err(e) = dispatch_discord(
            env,
            &kv,
            tenant_id,
            inbound_channel,
            customer_sender,
            &preview,
        )
        .await
        {
            // Non-fatal. Keep going so email still gets a chance.
            console_log!("Handoff Discord notify failed for {tenant_id}: {e:?}");
        }
    }

    if notif.approval_email {
        if let Err(e) = dispatch_email(
            env,
            db,
            tenant_id,
            inbound_channel,
            customer_sender,
            &preview,
        )
        .await
        {
            console_log!("Handoff email notify failed for {tenant_id}: {e:?}");
        }
    }

    Ok(())
}

async fn dispatch_discord(
    env: &Env,
    kv: &kv::KvStore,
    tenant_id: &str,
    inbound_channel: &Channel,
    customer_sender: &str,
    preview: &str,
) -> Result<()> {
    let cfg = match get_discord_config_by_tenant(kv, tenant_id).await? {
        Some(c) => c,
        None => return Ok(()),
    };
    let channel_id = match cfg.approval_channel_id.as_deref() {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => return Ok(()),
    };
    let bot = match bot_from_env(env) {
        Some(b) => b,
        None => return Ok(()),
    };

    let params = CreateMessage {
        content: "**Concierge needs you.** A customer message just tripped a handoff condition. Concierge has paused replying and is holding the conversation until a human takes over.".into(),
        embeds: vec![Embed {
            title: Some("Handoff requested".into()),
            description: Some(format!("**Last customer message:**\n{preview}")),
            color: Some(HANDOFF_COLOR),
            fields: vec![
                EmbedField {
                    name: "From".into(),
                    value: customer_sender.to_string(),
                    inline: true,
                },
                EmbedField {
                    name: "Channel".into(),
                    value: inbound_channel.as_str().into(),
                    inline: true,
                },
            ],
            footer: Some(EmbedFooter {
                text: "Reply directly on the original channel — Concierge won't keep responding."
                    .into(),
            }),
        }],
        components: vec![],
        ..Default::default()
    };
    bot.create_message(&channel_id, params).await?;
    Ok(())
}

async fn dispatch_email(
    env: &Env,
    db: &D1Database,
    tenant_id: &str,
    inbound_channel: &Channel,
    customer_sender: &str,
    preview: &str,
) -> Result<()> {
    let recipient = match get_tenant(db, tenant_id).await? {
        Some(t) => t.email,
        None => return Ok(()),
    };

    let email_domain = env
        .var("EMAIL_DOMAIN")
        .ok()
        .map(|v| v.to_string())
        .filter(|s| !s.is_empty());
    let base_url = env
        .var("PUBLIC_BASE_URL")
        .ok()
        .map(|v| v.to_string())
        .filter(|s| !s.is_empty());
    let (Some(email_domain), Some(base_url)) = (email_domain, base_url) else {
        return Ok(());
    };
    let from_addr = format!("noreply@{email_domain}");

    let channel_label = inbound_channel.as_str();
    let messages_url = format!("{base_url}/admin/messages");
    let text = format!(
        "Concierge has paused replying on a customer message that needs you.\n\n\
         Channel: {channel_label}\n\
         From: {customer_sender}\n\n\
         Last customer message:\n{preview}\n\n\
         Take it from here directly on {channel_label}. Concierge will hold the conversation \
         briefly and then go silent.\n\n\
         More: {messages_url}\n",
    );

    let outbound = OutboundEmail {
        from: from_addr,
        to: recipient,
        subject: "Concierge needs you on a customer message".into(),
        text: Some(text),
        html: None,
        reply_to: None,
        cc: vec![],
        bcc: vec![],
        headers: vec![],
    };
    send_outbound(env, &outbound).await
}

/// Clamp the customer excerpt to [`PREVIEW_LEN`] characters and strip
/// trailing whitespace so neither the Discord embed nor the email body
/// balloons on a single message.
fn clamp_preview(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= PREVIEW_LEN {
        return trimmed.to_string();
    }
    let truncated: String = trimmed
        .chars()
        .take(PREVIEW_LEN.saturating_sub(3))
        .collect();
    format!("{truncated}...")
}

fn bot_from_env(env: &Env) -> Option<DiscordBot> {
    let token = env.secret("DISCORD_BOT_TOKEN").ok()?.to_string();
    let app_id = env.secret("DISCORD_APPLICATION_ID").ok()?.to_string();
    let public_key = env.secret("DISCORD_PUBLIC_KEY").ok()?.to_string();
    if token.is_empty() || app_id.is_empty() || public_key.is_empty() {
        return None;
    }
    Some(DiscordBot::new(token, app_id, public_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_preview_passes_short_text_through() {
        assert_eq!(clamp_preview("hi"), "hi");
        assert_eq!(clamp_preview("  hi  "), "hi");
    }

    #[test]
    fn clamp_preview_truncates_long_text() {
        let long = "x".repeat(500);
        let out = clamp_preview(&long);
        assert!(out.ends_with("..."));
        assert!(out.chars().count() <= PREVIEW_LEN);
    }
}
