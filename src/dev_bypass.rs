//! Local-dev bypass switch (`DEV_MODE`).
//!
//! Active iff the `DEV_MODE` env var is `"1"` or `"true"` (case-insensitive).
//! Production deploys must never set `DEV_MODE`.
//!
//! When active:
//!   * `crate::ai` short-circuits chat generation to `STUB_CHAT_REPLY` and
//!     prompt-injection classification to `false`.
//!   * `crate::safety` short-circuits content classification to `Approved`.
//!   * `PUT /api/wizard/basics` and `PUT /api/whatsapp/:id` synchronously mark
//!     saved content `Approved` without needing a live Queue consumer.
//!   * `POST /auth/dev-login` mints a local session without Meta OAuth.

use worker::*;

/// True iff `DEV_MODE` is `"1"` or `"true"` (case-insensitive).
pub fn active(env: &Env) -> bool {
    env.var("DEV_MODE")
        .map(|v| is_dev_mode_value(&v.to_string()))
        .unwrap_or(false)
}

fn is_dev_mode_value(val: &str) -> bool {
    let trimmed = val.trim();
    trimmed == "1" || trimmed.eq_ignore_ascii_case("true")
}

/// Canned chat reply for `crate::ai::generate_chat_reply` when `DEV_MODE` is active.
pub const STUB_CHAT_REPLY: &str =
    "Thanks for reaching out — happy to help. (dev mode: stub reply.)";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_mode_value_matches_1_and_true() {
        assert!(is_dev_mode_value("1"));
        assert!(is_dev_mode_value(" 1 "));
        assert!(is_dev_mode_value("true"));
        assert!(is_dev_mode_value("TRUE"));
        assert!(is_dev_mode_value("True"));
        assert!(!is_dev_mode_value("0"));
        assert!(!is_dev_mode_value("false"));
        assert!(!is_dev_mode_value(""));
        assert!(!is_dev_mode_value("yes"));
    }
}
