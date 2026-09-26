//! Human-handoff notifications.
//!
//! When the AI emits the handoff sentinel on a customer turn, the
//! pipeline strips the token, switches the conversation into the
//! holding-pattern path, and calls [`notify_human_requested`] exactly
//! once. This module owns that one-shot page.
//!
//! Email is the only delivery channel: the Discord relay that used to
//! carry an embed with Reply/Approve buttons was cut, and with it the
//! approval queue. A handoff now pages the tenant and nothing else —
//! they take the conversation over on the channel it arrived on.
//!
//! Failures are logged but not propagated. A handoff alert that
//! doesn't reach the tenant is unfortunate; an alert that errors and
//! takes the whole inbound pipeline down is worse.
//!
//! Idempotency: callers MUST guard with `HandoffState::notified`. This
//! module fans out unconditionally.

use worker::*;

use crate::email::send::{send_outbound, OutboundEmail};
use crate::storage::{get_onboarding, get_tenant};
use crate::types::Channel;

const PREVIEW_LEN: usize = 200;

/// One-shot notification: a customer message just tripped a handoff.
///
/// Honours the tenant's `NotificationConfig.approval_email` opt-out.
/// Customer excerpt is clamped to [`PREVIEW_LEN`] so the email body
/// doesn't balloon.
pub async fn notify_human_requested(
    env: &Env,
    db: &D1Database,
    tenant_id: &str,
    inbound_channel: &Channel,
    customer_sender: &str,
    customer_excerpt: &str,
    risk: Option<crate::risk::RiskReason>,
) -> Result<()> {
    let kv = env.kv("KV")?;
    let onboarding = get_onboarding(&kv, tenant_id).await?;
    if !onboarding.notifications.handoff_email {
        return Ok(());
    }

    let preview = clamp_preview(customer_excerpt);
    if let Err(e) = dispatch_email(
        env,
        db,
        tenant_id,
        inbound_channel,
        customer_sender,
        &preview,
        risk,
    )
    .await
    {
        console_log!("Handoff email notify failed for {tenant_id}: {e:?}");
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_email(
    env: &Env,
    db: &D1Database,
    tenant_id: &str,
    inbound_channel: &Channel,
    customer_sender: &str,
    preview: &str,
    risk: Option<crate::risk::RiskReason>,
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
    let dashboard_url = format!("{base_url}/dashboard");
    // Two ways to get here: the model asked for a human, or the risk gate
    // withheld a draft. The tenant needs to know which, because in the
    // second case a reply was written and deliberately not sent.
    let why = match risk {
        Some(reason) => format!(
            "Concierge wrote a reply and did not send it, because {}.\n\n",
            reason.label()
        ),
        None => String::new(),
    };
    let text = format!(
        "Concierge has paused replying on a customer message that needs you.\n\n\
         {why}\
         Channel: {channel_label}\n\
         From: {customer_sender}\n\n\
         Last customer message:\n{preview}\n\n\
         Take it from here directly on {channel_label}. Concierge will hold the conversation \
         briefly and then go silent.\n\n\
         More: {dashboard_url}\n",
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
/// trailing whitespace so the email body doesn't balloon on a single
/// message.
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
