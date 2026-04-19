use worker::*;

use crate::email::{forward, mime};
use crate::helpers::generate_id;
use crate::types::*;

/// Parse an inbound email into an InboundMessage.
pub fn parse_inbound(
    from: &str,
    to: &str,
    parsed: &mime::ParsedEmail,
    tenant_id: &str,
    domain: &str,
) -> InboundMessage {
    InboundMessage {
        id: generate_id(),
        channel: Channel::Email,
        sender: from.to_string(),
        sender_name: None,
        recipient: to.to_string(),
        body: parsed.text_body.clone().unwrap_or_default(),
        subject: Some(parsed.subject.clone()),
        has_attachment: parsed.has_attachment,
        tenant_id: tenant_id.to_string(),
        channel_account_id: domain.to_string(),
        raw_metadata: serde_json::json!({
            "domain": domain,
            "original_from": from,
            "original_to": to,
        }),
    }
}

/// The result of sending an email reply.
pub enum EmailSendResult {
    Sent {
        from: String,
        to: String,
        raw: Vec<u8>,
    },
}

/// Send a reply via email. Builds a MIME message and returns the raw bytes
/// for the caller to dispatch via the send_email binding.
pub async fn send_reply(
    env: &Env,
    metadata: &serde_json::Value,
    to: &str,
    body: &str,
    subject: Option<&str>,
) -> Result<EmailSendResult> {
    let domain = metadata
        .get("domain")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let original_to = metadata
        .get("original_to")
        .and_then(|v| v.as_str())
        .unwrap_or(domain);

    let subject = subject.unwrap_or("Re: your message");

    // Build a simple reply email
    let parsed = mime::ParsedEmail {
        subject: subject.to_string(),
        message_id: None,
        references: None,
        text_body: Some(body.to_string()),
        html_body: None,
        has_attachment: false,
    };

    let raw = mime::build_reply_email(&parsed, original_to, to, domain);

    Ok(EmailSendResult::Sent {
        from: original_to.to_string(),
        to: to.to_string(),
        raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_inbound_sets_fields() {
        let parsed = mime::ParsedEmail {
            subject: "Test subject".into(),
            message_id: Some("msg-id-1".into()),
            references: None,
            text_body: Some("Hello world".into()),
            html_body: None,
            has_attachment: true,
        };

        let msg = parse_inbound(
            "alice@example.com",
            "support@proxy.com",
            &parsed,
            "t1",
            "proxy.com",
        );
        assert_eq!(msg.channel, Channel::Email);
        assert_eq!(msg.sender, "alice@example.com");
        assert_eq!(msg.recipient, "support@proxy.com");
        assert_eq!(msg.body, "Hello world");
        assert_eq!(msg.subject.as_deref(), Some("Test subject"));
        assert!(msg.has_attachment);
        assert_eq!(msg.tenant_id, "t1");
        assert_eq!(msg.channel_account_id, "proxy.com");
    }

    #[test]
    fn parse_inbound_empty_body() {
        let parsed = mime::ParsedEmail {
            subject: String::new(),
            message_id: None,
            references: None,
            text_body: None,
            html_body: None,
            has_attachment: false,
        };

        let msg = parse_inbound("a@b.com", "c@d.com", &parsed, "t1", "d.com");
        assert_eq!(msg.body, "");
    }
}
