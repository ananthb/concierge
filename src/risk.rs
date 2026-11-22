//! Risk gate for AI-generated drafts.
//!
//! Synchronous heuristic over a freshly generated draft: no model calls, no I/O.
//! Drafts that quote a price, make a commitment, or come out a suspicious length
//! trigger the human-handoff path instead of reaching the customer directly.

/// Why a draft was withheld. Recorded in logs and surfaced on the handoff alert.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RiskReason {
    Length,
    MoneyWord,
    Commitment,
}

impl RiskReason {
    pub fn as_str(self) -> &'static str {
        match self {
            RiskReason::Length => "length",
            RiskReason::MoneyWord => "money_word",
            RiskReason::Commitment => "commitment",
        }
    }

    /// One-line explanation for the tenant-facing handoff notification.
    pub fn label(self) -> &'static str {
        match self {
            RiskReason::Length => "the draft was unusually short or long",
            RiskReason::MoneyWord => "the draft mentioned money",
            RiskReason::Commitment => "the draft made a commitment",
        }
    }
}

const MIN_LEN: usize = 8;
const MAX_LEN: usize = 600;

const MONEY_WORDS: &[&str] = &[
    "₹", "$", "price", "quote", "refund", "discount", "free", "cost",
];

const COMMITMENT_WORDS: &[&str] = &[
    "guarantee",
    "promise",
    "confirmed",
    "booked",
    "by mon",
    "by tue",
    "by wed",
    "by thu",
    "by fri",
    "by sat",
    "by sun",
    "by tomorrow",
];

/// `Some(reason)` if the draft must not be sent as-is.
pub fn signal(draft: &str) -> Option<RiskReason> {
    if risk_length(draft) {
        return Some(RiskReason::Length);
    }
    let lower = draft.to_lowercase();
    if MONEY_WORDS.iter().any(|w| lower.contains(*w)) {
        return Some(RiskReason::MoneyWord);
    }
    if COMMITMENT_WORDS.iter().any(|w| lower.contains(*w)) {
        return Some(RiskReason::Commitment);
    }
    None
}

fn risk_length(draft: &str) -> bool {
    let n = draft.chars().count();
    !(MIN_LEN..=MAX_LEN).contains(&n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_draft_passes() {
        assert_eq!(signal("Sure, we're open until six today."), None);
    }

    #[test]
    fn money_words_trip_the_gate() {
        for draft in [
            "That'll be ₹500.",
            "The price is listed online.",
            "We can offer a discount.",
            "Happy to issue a refund.",
        ] {
            assert_eq!(
                signal(draft),
                Some(RiskReason::MoneyWord),
                "expected money signal for {draft:?}"
            );
        }
    }

    #[test]
    fn commitment_words_trip_the_gate() {
        assert_eq!(
            signal("I guarantee it ships today."),
            Some(RiskReason::Commitment)
        );
        assert_eq!(
            signal("We'll have it ready by tomorrow."),
            Some(RiskReason::Commitment)
        );
    }

    #[test]
    fn length_is_checked_before_content() {
        assert_eq!(signal("ok"), Some(RiskReason::Length));
        let long = "a".repeat(601);
        assert_eq!(signal(&long), Some(RiskReason::Length));
    }
}
