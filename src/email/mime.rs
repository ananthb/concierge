use mail_parser::{HeaderValue, MessageParser};

/// Parsed representation of an inbound email. Only the fields we need to
/// reconstruct an outbound message via Cloudflare Email Service.
pub struct ParsedEmail {
    pub subject: String,
    pub message_id: Option<String>,
    pub references: Option<String>,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub has_attachment: bool,
    /// Reply-To header (first address). Preferred for routing replies back
    /// to the real human when an inbound is forwarded mail.
    pub reply_to: Option<String>,
    /// X-Forwarded-For / X-Original-From. Set by some mail forwarders to
    /// preserve the original sender when the From address gets rewritten.
    pub x_forwarded_for: Option<String>,
    pub x_original_from: Option<String>,
}

fn first_address(value: &HeaderValue<'_>) -> Option<String> {
    match value {
        HeaderValue::Address(addr) => addr
            .first()
            .and_then(|a| a.address.as_ref())
            .map(|s| s.to_string()),
        HeaderValue::Text(s) => Some(s.to_string()),
        _ => None,
    }
}

fn header_text<'a>(message: &'a mail_parser::Message<'a>, name: &str) -> Option<String> {
    let v = message.header(name)?;
    match v {
        HeaderValue::Text(s) => Some(s.to_string()),
        HeaderValue::Address(addr) => addr
            .first()
            .and_then(|a| a.address.as_ref())
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// Parse raw MIME bytes into a structured email.
pub fn parse_email(raw: &[u8]) -> Option<ParsedEmail> {
    let message = MessageParser::default().parse(raw)?;
    let has_attachment = message.attachment_count() > 0;

    let reply_to = message.header("Reply-To").and_then(first_address);
    let x_forwarded_for = header_text(&message, "X-Forwarded-For");
    let x_original_from = header_text(&message, "X-Original-From");

    Some(ParsedEmail {
        subject: message.subject().unwrap_or("").to_string(),
        message_id: message.message_id().map(|s| s.to_string()),
        references: message.references().as_text().map(|s| s.to_string()),
        text_body: message.body_text(0).map(|s| s.to_string()),
        html_body: message.body_html(0).map(|s| s.to_string()),
        has_attachment,
        reply_to,
        x_forwarded_for,
        x_original_from,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_EMAIL: &[u8] = b"From: Alice <alice@example.org>\r\n\
        To: shop123@proxy.example.com\r\n\
        Subject: Hello from Alice\r\n\
        Message-ID: <original123@example.org>\r\n\
        Content-Type: text/plain; charset=utf-8\r\n\
        \r\n\
        Hi there, this is a test email.\r\n";

    const MULTIPART_EMAIL: &[u8] = b"From: Bob <bob@shop.com>\r\n\
        To: orders@proxy.example.com\r\n\
        Subject: Your order confirmation\r\n\
        Message-ID: <order456@shop.com>\r\n\
        In-Reply-To: <prev789@proxy.example.com>\r\n\
        References: <prev789@proxy.example.com>\r\n\
        MIME-Version: 1.0\r\n\
        Content-Type: multipart/alternative; boundary=\"boundary42\"\r\n\
        \r\n\
        --boundary42\r\n\
        Content-Type: text/plain; charset=utf-8\r\n\
        \r\n\
        Your order #1234 is confirmed.\r\n\
        --boundary42\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        \r\n\
        <h1>Your order #1234 is confirmed.</h1>\r\n\
        --boundary42--\r\n";

    #[test]
    fn parse_simple_email() {
        let parsed = parse_email(SIMPLE_EMAIL).expect("should parse");
        assert_eq!(parsed.subject, "Hello from Alice");
        assert_eq!(
            parsed.message_id.as_deref(),
            Some("original123@example.org")
        );
        assert!(parsed.text_body.as_ref().unwrap().contains("test email"));
        assert!(!parsed.has_attachment);
    }

    #[test]
    fn parse_multipart_email() {
        let parsed = parse_email(MULTIPART_EMAIL).expect("should parse");
        assert_eq!(parsed.subject, "Your order confirmation");
        assert!(parsed.text_body.as_ref().unwrap().contains("order #1234"));
        assert!(parsed.html_body.as_ref().unwrap().contains("<h1>"));
        assert_eq!(
            parsed.references.as_deref(),
            Some("prev789@proxy.example.com")
        );
    }
}
