pub mod digest;
pub mod forward;
pub mod handler;
pub mod mime;
pub mod send;

const RESERVED_LOCAL_PARTS: &[&str] = &[
    "admin",
    "root",
    "postmaster",
    "abuse",
    "noreply",
    "no-reply",
    "mailer-daemon",
    "www",
    "mail",
    "smtp",
    "hostmaster",
    "security",
    "support",
    "help",
    "info",
];

/// Validate a customer-chosen email local-part.
///
/// Rules: 1–32 chars, lowercase a–z, digits, single dot/dash/underscore
/// separators (no leading/trailing/consecutive separators), reserved labels
/// blocked. Returns a static error message suitable for showing the user.
pub fn validate_local_part(label: &str) -> Result<(), &'static str> {
    if label.is_empty() {
        return Err("Address can't be empty");
    }
    if label.len() > 32 {
        return Err("Address must be 32 characters or fewer");
    }
    let bytes = label.as_bytes();
    let first = bytes[0];
    let last = bytes[bytes.len() - 1];
    if matches!(first, b'.' | b'-' | b'_') || matches!(last, b'.' | b'-' | b'_') {
        return Err("Address can't start or end with a separator");
    }
    let mut prev_was_sep = false;
    for &b in bytes {
        let is_alnum = b.is_ascii_lowercase() || b.is_ascii_digit();
        let is_sep = matches!(b, b'.' | b'-' | b'_');
        if !is_alnum && !is_sep {
            return Err("Address can only contain a-z, 0-9, dot, dash, underscore");
        }
        if is_sep && prev_was_sep {
            return Err("Address can't have two separators in a row");
        }
        prev_was_sep = is_sep;
    }
    if RESERVED_LOCAL_PARTS.contains(&label) {
        return Err("That address is reserved — please pick another");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_local_part;

    #[test]
    fn accepts_simple_labels() {
        assert!(validate_local_part("acme").is_ok());
        assert!(validate_local_part("acme-co").is_ok());
        assert!(validate_local_part("a.b").is_ok());
        assert!(validate_local_part("a_b_c").is_ok());
        assert!(validate_local_part("user1").is_ok());
    }

    #[test]
    fn rejects_uppercase() {
        assert!(validate_local_part("Acme").is_err());
    }

    #[test]
    fn rejects_consecutive_separators() {
        assert!(validate_local_part("a..b").is_err());
        assert!(validate_local_part("a--b").is_err());
    }

    #[test]
    fn rejects_leading_or_trailing_separator() {
        assert!(validate_local_part("-foo").is_err());
        assert!(validate_local_part(".foo").is_err());
        assert!(validate_local_part("foo-").is_err());
    }

    #[test]
    fn rejects_spaces_or_specials() {
        assert!(validate_local_part("with space").is_err());
        assert!(validate_local_part("hi+plus").is_err());
    }

    #[test]
    fn rejects_reserved() {
        assert!(validate_local_part("admin").is_err());
        assert!(validate_local_part("postmaster").is_err());
    }

    #[test]
    fn rejects_too_long() {
        let long = "a".repeat(33);
        assert!(validate_local_part(&long).is_err());
    }
}
