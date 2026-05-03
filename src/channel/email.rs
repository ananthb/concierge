use worker::*;

use crate::email::send::{self, OutboundEmail};
use crate::storage::*;
use crate::types::*;

/// Send a reply via email through the EMAIL binding.
///
/// `metadata` carries inbound context (from `InboundMessage.raw_metadata`):
/// - `local_part`: the concierge address that received the inbound mail.
/// - `base_domain`: the platform email domain.
/// - `original_subject`, `message_id`, `references`: for thread headers.
///
/// CC and BCC recipients come from the address's verified
/// `notification_recipients` list. Owner is always present.
pub async fn send_reply(
    env: &Env,
    metadata: &serde_json::Value,
    to: &str,
    body: &str,
    subject: Option<&str>,
) -> Result<()> {
    let kv = env.kv("KV")?;

    let local_part = metadata
        .get("local_part")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let base_domain = metadata
        .get("base_domain")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let tenant_id = metadata
        .get("tenant_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let from_addr = if !local_part.is_empty() && !base_domain.is_empty() {
        format!("{local_part}@{base_domain}")
    } else {
        // Fallback for callers that don't set these (won't happen in the
        // unified pipeline, but keeps reverse-alias replies working).
        metadata
            .get("original_to")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    // Load verified recipients for CC/BCC. Owner is always Verified.
    let mut cc_list: Vec<String> = Vec::new();
    let mut bcc_list: Vec<String> = Vec::new();
    if !tenant_id.is_empty() && !local_part.is_empty() {
        if let Some(addr) = get_email_address(&kv, &tenant_id, &local_part).await? {
            for r in &addr.notification_recipients {
                if r.status != RecipientStatus::Verified {
                    continue;
                }
                match r.kind {
                    RecipientKind::Cc => cc_list.push(r.address.clone()),
                    RecipientKind::Bcc => bcc_list.push(r.address.clone()),
                }
            }
        }
    }

    let original_subject = metadata
        .get("original_subject")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let subject = match subject {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => reply_subject(original_subject),
    };

    let mut headers: Vec<(String, String)> =
        vec![("X-EmailProxy-Forwarded".to_string(), "1".to_string())];
    if let Some(msg_id) = metadata.get("message_id").and_then(|v| v.as_str()) {
        if !msg_id.is_empty() {
            headers.push(("In-Reply-To".to_string(), msg_id.to_string()));
        }
    }
    if let Some(refs) = metadata.get("references").and_then(|v| v.as_str()) {
        if !refs.is_empty() {
            headers.push(("References".to_string(), refs.to_string()));
        }
    }

    let outbound = OutboundEmail {
        from: from_addr.clone(),
        to: to.to_string(),
        subject,
        text: Some(body.to_string()),
        html: None,
        reply_to: Some(from_addr),
        cc: cc_list,
        bcc: bcc_list,
        headers,
    };

    send::send_outbound(env, &outbound).await
}

fn reply_subject(original: &str) -> String {
    let trimmed = original.trim();
    if trimmed.is_empty() {
        return "Re: your message".to_string();
    }
    if trimmed.len() >= 3 && trimmed[..3].eq_ignore_ascii_case("re:") {
        trimmed.to_string()
    } else {
        format!("Re: {trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::reply_subject;

    #[test]
    fn empty_subject_falls_back() {
        assert_eq!(reply_subject(""), "Re: your message");
    }

    #[test]
    fn prefixes_re_when_missing() {
        assert_eq!(reply_subject("Order #1234"), "Re: Order #1234");
    }

    #[test]
    fn does_not_double_prefix() {
        assert_eq!(reply_subject("Re: hello"), "Re: hello");
        assert_eq!(reply_subject("RE: hello"), "RE: hello");
    }
}
