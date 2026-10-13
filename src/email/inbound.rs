//! Inbound mail: Email Routing → Worker → shared pipeline.
//!
//! Cloudflare Email Routing hands every message for a connected domain to
//! this Worker's `email` event. We resolve the recipient to a tenant's
//! [`EmailAccount`], build the same [`InboundMessage`] the WhatsApp webhook
//! builds, and hand it to the shared pipeline. The reply context (subject,
//! `Message-ID`, `References`, the account's From) rides in `raw_metadata`
//! so the outbound adapter can thread the answer onto the same thread.
//!
//! [`EmailAccount`]: crate::types::EmailAccount
//! [`InboundMessage`]: crate::types::InboundMessage

use worker::*;

use crate::email::mime;
use crate::helpers::generate_id;
use crate::pipeline;
use crate::storage::{get_email_account, resolve_email_account_id};
use crate::types::{Channel, InboundMessage};

pub async fn handle(message: ForwardableEmailMessage, env: Env) -> Result<()> {
    let to = message.to();
    let from = message.from();
    let kv = env.kv("KV")?;

    // Envelope to a domain we don't host: reject rather than swallow. Only
    // reachable when a catch-all routes the whole zone to the Worker.
    let Some(account_id) = resolve_email_account_id(&kv, &to).await? else {
        message.set_reject("Recipient address is not configured for this service.");
        return Ok(());
    };
    let Some(account) = get_email_account(&kv, &account_id).await? else {
        message.set_reject("Recipient address is not configured for this service.");
        return Ok(());
    };

    let headers = message.headers();
    let header = |name: &str| headers.get(name).ok().flatten().unwrap_or_default();
    let subject = header("subject");
    let message_id = header("message-id");
    let references = header("references");

    // The body is only available as raw RFC 5322 bytes.
    let raw = message.raw_bytes().await?;
    let body = mime::extract_text(&mime::parse_body(&raw));
    if body.is_empty() {
        // Accept (don't bounce) an attachment-only or empty message: there
        // is simply nothing to answer.
        console_log!("Inbound email from {from} has no text body, ignoring");
        return Ok(());
    }

    let msg = InboundMessage {
        id: generate_id(),
        channel: Channel::Email,
        sender: from.clone(),
        sender_name: None,
        recipient: to.clone(),
        body,
        // Attachment presence isn't surfaced to the pipeline yet; like
        // WhatsApp, a note-only inbound is treated as text.
        has_attachment: false,
        tenant_id: account.tenant_id.clone(),
        channel_account_id: account.id.clone(),
        raw_metadata: serde_json::json!({
            "subject": subject,
            "message_id": message_id,
            "references": references,
            "to": to,
            "from_address": account.from_address(),
            "from_header": account.from_header(),
        }),
    };

    pipeline::process_inbound(&msg, &env).await
}
