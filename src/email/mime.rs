//! MIME parsing for inbound mail.
//!
//! Cloudflare's `ForwardableEmailMessage` exposes the envelope (`from`,
//! `to`) and the top-level headers directly, but the body only as raw
//! RFC 5322 bytes. `mail-parser` turns those into the text/plain (and
//! text/html) parts we hand the pipeline.
//!
//! We keep only what the reply path needs. Attachments and headers beyond
//! the ones the message object already gives us are ignored.

use mail_parser::MessageParser;

/// The bodies of interest from an inbound message.
pub struct ParsedBody {
    pub text: Option<String>,
    pub html: Option<String>,
}

/// Parse raw MIME bytes into the text and HTML bodies. A message that
/// doesn't parse yields two `None`s rather than an error: an unreadable
/// body should drop the message, not fail the email event.
pub fn parse_body(raw: &[u8]) -> ParsedBody {
    match MessageParser::default().parse(raw) {
        Some(message) => ParsedBody {
            text: message.body_text(0).map(|s| s.to_string()),
            html: message.body_html(0).map(|s| s.to_string()),
        },
        None => ParsedBody {
            text: None,
            html: None,
        },
    }
}

/// The best available plain-text rendering of an inbound message.
///
/// `body_text` already renders an HTML-only message to text (tags dropped,
/// entities decoded), so it is preferred. We normalize it either way —
/// mail-parser leaves a non-breaking space where `&nbsp;` was — and fall
/// back to stripping the HTML body only when there is no usable text part.
/// No truncation happens here; the pipeline caps body length before it
/// reaches the model.
pub fn extract_text(parsed: &ParsedBody) -> String {
    if let Some(text) = parsed.text.as_ref() {
        let normalized = collapse_ws(text);
        if !normalized.is_empty() {
            return normalized;
        }
    }
    match parsed.html.as_ref() {
        Some(html) => collapse_ws(&strip_html(html)),
        None => String::new(),
    }
}

/// Drop HTML tags. Deliberately not a parser: the output is a prompt
/// context, not a rendered document.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

/// Decode the handful of entities that survive in enquiry mail, normalize
/// non-breaking spaces, and collapse runs of whitespace to single spaces.
fn collapse_ws(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace('\u{a0}', " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: &[u8] = b"From: Alice <alice@example.org>\r\n\
        To: hello@acme.com\r\n\
        Subject: Enquiry\r\n\
        Content-Type: text/plain; charset=utf-8\r\n\
        \r\n\
        Do you deliver on Saturdays?\r\n";

    const MULTIPART: &[u8] = b"From: Bob <bob@example.org>\r\n\
        To: hello@acme.com\r\n\
        Subject: Quote please\r\n\
        MIME-Version: 1.0\r\n\
        Content-Type: multipart/alternative; boundary=\"b42\"\r\n\
        \r\n\
        --b42\r\n\
        Content-Type: text/plain; charset=utf-8\r\n\
        \r\n\
        Can I get a quote for 20 chairs?\r\n\
        --b42\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        \r\n\
        <p>Can I get a quote for <b>20 chairs</b>?</p>\r\n\
        --b42--\r\n";

    const HTML_ONLY: &[u8] = b"From: Carol <carol@example.org>\r\n\
        To: hello@acme.com\r\n\
        Subject: Hours\r\n\
        Content-Type: text/html; charset=utf-8\r\n\
        \r\n\
        <p>What are your <strong>opening hours</strong>&nbsp;today?</p>\r\n";

    #[test]
    fn extracts_plain_text_body() {
        let parsed = parse_body(PLAIN);
        assert_eq!(extract_text(&parsed), "Do you deliver on Saturdays?");
    }

    #[test]
    fn prefers_plain_over_html_in_multipart() {
        let parsed = parse_body(MULTIPART);
        assert_eq!(extract_text(&parsed), "Can I get a quote for 20 chairs?");
    }

    #[test]
    fn falls_back_to_stripped_html() {
        let parsed = parse_body(HTML_ONLY);
        assert_eq!(extract_text(&parsed), "What are your opening hours today?");
    }

    #[test]
    fn unparseable_body_is_empty() {
        let parsed = parse_body(b"\x00\x01\x02 not an email");
        assert_eq!(extract_text(&parsed), "");
    }

    #[test]
    fn strip_html_drops_tags_and_collapse_decodes_entities() {
        assert_eq!(
            collapse_ws(&strip_html("<p>a &amp; b</p>\n<p>c</p>")),
            "a & b c".to_string()
        );
    }
}
