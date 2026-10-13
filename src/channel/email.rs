//! Email channel adapter.
//!
//! Outbound replies go through Cloudflare Email Service from the tenant's
//! own domain, threaded onto the original message so the customer's client
//! groups it. The SMTP envelope differs from WhatsApp — there is no
//! recipient id, only the sender's address — so the reply context (the
//! original subject, `Message-ID` and `References`) travels in the
//! inbound message's `raw_metadata`, stamped by `email::inbound`.

use worker::*;

use crate::email::send::{send_outbound, OutboundEmail};

/// Send an auto-reply to an inbound email.
///
/// `metadata` is the originating `InboundMessage::raw_metadata`: it holds
/// the account's From header and the threading headers. `to` is the
/// customer's address (the inbound `sender`); `body` is the reply text.
pub async fn send_reply(
    env: &Env,
    metadata: &serde_json::Value,
    to: &str,
    body: &str,
) -> Result<()> {
    let str_field = |key: &str| metadata.get(key).and_then(|v| v.as_str()).unwrap_or("");

    // The From header carries the display name; `from_address` is the bare
    // fallback if the account had no name when the metadata was stamped.
    let from = match (str_field("from_header"), str_field("from_address")) {
        ("", "") => return Err(Error::from("email reply metadata has no From address")),
        (header, _) if !header.is_empty() => header.to_string(),
        (_, address) => address.to_string(),
    };

    let subject = reply_subject(str_field("subject"));

    let mut headers = Vec::new();
    let message_id = str_field("message_id");
    if !message_id.is_empty() {
        headers.push(("In-Reply-To".to_string(), message_id.to_string()));
        let references = str_field("references");
        let value = if references.is_empty() {
            message_id.to_string()
        } else {
            format!("{references} {message_id}")
        };
        headers.push(("References".to_string(), value));
    }

    let outbound = OutboundEmail {
        from,
        to: to.to_string(),
        subject,
        text: Some(body.to_string()),
        html: None,
        reply_to: None,
        cc: Vec::new(),
        bcc: Vec::new(),
        headers,
    };
    send_outbound(env, &outbound).await
}

/// Build the reply subject: `Re: <original>`, unless the original already
/// carries a `Re:` prefix. Empty subjects become a definite placeholder so
/// the customer doesn't get a blank subject line.
fn reply_subject(subject: &str) -> String {
    let subject = subject.trim();
    if subject.is_empty() {
        return "Re: your enquiry".to_string();
    }
    let lower = subject.to_ascii_lowercase();
    if lower.starts_with("re:") {
        subject.to_string()
    } else {
        format!("Re: {subject}")
    }
}

#[cfg(test)]
mod tests {
    use super::reply_subject;

    #[test]
    fn prefixes_re_for_a_plain_subject() {
        assert_eq!(reply_subject("Do you deliver?"), "Re: Do you deliver?");
    }

    #[test]
    fn does_not_double_prefix() {
        assert_eq!(reply_subject("Re: Do you deliver?"), "Re: Do you deliver?");
        assert_eq!(reply_subject("RE: earlier"), "RE: earlier");
    }

    #[test]
    fn empty_subject_gets_a_placeholder() {
        assert_eq!(reply_subject("   "), "Re: your enquiry");
    }
}
