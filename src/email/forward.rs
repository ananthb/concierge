use super::mime::ParsedEmail;

/// Check if an address is a reverse alias (reply+ prefix).
pub fn is_reverse_alias(address: &str) -> bool {
    address.starts_with("reply+")
}

/// Pick the best "original sender" for inbound mail. Forwarded mail often
/// arrives with `From:` set to the forwarder; the human we want to reply
/// to lives in `Reply-To:`, `X-Forwarded-For:`, or `X-Original-From:`.
/// Falls back to envelope `from` when no header indicates a forward.
pub fn extract_original_sender(parsed: Option<&ParsedEmail>, envelope_from: &str) -> String {
    if let Some(p) = parsed {
        if let Some(addr) = p.reply_to.as_deref() {
            if !addr.is_empty() {
                return addr.to_string();
            }
        }
        if let Some(addr) = p.x_original_from.as_deref() {
            if !addr.is_empty() {
                return addr.to_string();
            }
        }
        if let Some(addr) = p.x_forwarded_for.as_deref() {
            if !addr.is_empty() {
                return addr.to_string();
            }
        }
    }
    envelope_from.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_reverse_alias() {
        assert!(is_reverse_alias("reply+abc123@proxy.example.com"));
        assert!(!is_reverse_alias("shop123@proxy.example.com"));
        assert!(!is_reverse_alias(""));
    }
}
