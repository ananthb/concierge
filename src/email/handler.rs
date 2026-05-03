//! Inbound email handler. Normalises mail into an `InboundMessage` and hands
//! it off to the unified pipeline (the same one WhatsApp/Instagram/Discord
//! use). The pipeline buffers, generates a reply (AI or static), and sends
//! it back via the email channel adapter.

use worker::*;

use super::send::OutboundEmail;
use super::{forward, mime};
use crate::helpers::generate_id;
use crate::pipeline;
use crate::storage::*;
use crate::types::*;

/// Result of email processing.
pub enum EmailResult {
    /// Silently consume the message (the reply, if any, is dispatched
    /// asynchronously through the pipeline).
    Drop,
    /// Reject the email at SMTP time with a message.
    Reject(String),
    /// Send a single outbound email immediately (used for reverse-alias
    /// reply routing; synchronous, no buffering).
    Send(OutboundEmail),
}

/// Handle an incoming email. Called from the wasm_bindgen `email()` export.
pub async fn handle_email(
    from: &str,
    to: &str,
    raw_bytes: &[u8],
    env: &Env,
) -> Result<EmailResult> {
    let kv = env.kv("KV")?;

    let to_lower = to.to_lowercase();
    let domain = to_lower.rsplit('@').next().unwrap_or("").to_string();
    let local_part = to_lower.split('@').next().unwrap_or("").to_string();

    if domain.is_empty() || local_part.is_empty() {
        return Ok(EmailResult::Reject("Invalid recipient".into()));
    }

    let base_domain = env
        .var("EMAIL_DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_default();
    if domain != base_domain {
        console_log!("Mail to non-platform domain {domain}, rejecting");
        return Ok(EmailResult::Reject("Unknown domain".into()));
    }

    // Loop detection: search the header section (up to first blank line) for
    // our forwarding marker.
    let header_end = raw_bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(raw_bytes.len().min(8192));
    let header_lower: Vec<u8> = raw_bytes[..header_end]
        .iter()
        .map(|b| b.to_ascii_lowercase())
        .collect();
    if header_lower
        .windows(b"x-emailproxy-forwarded:".len())
        .any(|w| w == b"x-emailproxy-forwarded:")
    {
        console_log!("Loop detected: {from} -> {to}");
        return Ok(EmailResult::Reject("Forwarding loop detected".into()));
    }

    let parsed = mime::parse_email(raw_bytes);

    // Reply that arrived at a reverse alias. Route it back to the original
    // human via the saved alias mapping.
    if forward::is_reverse_alias(&to_lower) {
        return handle_reverse_alias(&to_lower, &parsed, &kv).await;
    }

    let tenant_id = match get_tenant_by_address(&kv, &local_part).await? {
        Some(id) => id,
        None => {
            console_log!("No tenant for address: {local_part}@{base_domain}");
            return Ok(EmailResult::Reject("Unknown address".into()));
        }
    };

    // Pick the best "real" sender. For forwarded mail this is in
    // Reply-To / X-Forwarded-For / X-Original-From; otherwise envelope From.
    let sender = forward::extract_original_sender(parsed.as_ref(), from);

    let subject = parsed
        .as_ref()
        .map(|p| p.subject.clone())
        .unwrap_or_default();
    let body = parsed
        .as_ref()
        .and_then(|p| p.text_body.clone())
        .unwrap_or_default();
    let has_attachment = parsed.as_ref().map(|p| p.has_attachment).unwrap_or(false);

    let message_id = parsed.as_ref().and_then(|p| p.message_id.clone());
    let references = parsed.as_ref().and_then(|p| p.references.clone());

    let msg = InboundMessage {
        id: generate_id(),
        channel: Channel::Email,
        sender,
        sender_name: None,
        recipient: format!("{local_part}@{base_domain}"),
        body,
        subject: Some(subject.clone()),
        has_attachment,
        tenant_id: tenant_id.clone(),
        // For email the "channel account" is the local-part. Uniquely
        // identifies the address that received the message.
        channel_account_id: local_part.clone(),
        raw_metadata: serde_json::json!({
            "local_part": local_part,
            "base_domain": base_domain,
            "tenant_id": tenant_id,
            "envelope_from": from,
            "original_subject": subject,
            "message_id": message_id,
            "references": references,
        }),
    };

    if let Err(e) = pipeline::process_inbound(&msg, env).await {
        console_log!("Email pipeline error: {:?}", e);
    }

    // Reply (if any) was dispatched async through the pipeline.
    Ok(EmailResult::Drop)
}

/// Route a reply that arrived at a reverse alias back to the original
/// recipient. This is synchronous because there's no rule/AI step. The
/// alias mapping tells us exactly where to send.
async fn handle_reverse_alias(
    to_lower: &str,
    parsed: &Option<mime::ParsedEmail>,
    kv: &kv::KvStore,
) -> Result<EmailResult> {
    let reverse = match get_email_reverse_alias(kv, to_lower).await? {
        Some(r) => r,
        None => {
            console_log!("Unknown reverse alias: {to_lower}");
            return Ok(EmailResult::Reject("Unknown reverse alias".into()));
        }
    };

    let parsed = match parsed {
        Some(p) => p,
        None => return Ok(EmailResult::Reject("Failed to parse reply".into())),
    };

    let mut headers: Vec<(String, String)> =
        vec![("X-EmailProxy-Forwarded".to_string(), "1".to_string())];
    if let Some(msg_id) = &parsed.message_id {
        headers.push(("In-Reply-To".to_string(), msg_id.clone()));
    }
    if let Some(refs) = &parsed.references {
        headers.push(("References".to_string(), refs.clone()));
    }

    Ok(EmailResult::Send(OutboundEmail {
        from: reverse.alias.clone(),
        to: reverse.original_sender,
        subject: parsed.subject.clone(),
        text: parsed.text_body.clone(),
        html: parsed.html_body.clone(),
        reply_to: Some(reverse.alias),
        cc: vec![],
        bcc: vec![],
        headers,
    }))
}
