//! Risk gate for AI-generated drafts.
//!
//! A cheap, synchronous heuristic over a freshly generated draft: no model
//! calls, no I/O. Drafts that look like they're quoting a price, making a
//! commitment, drifting off the persona's stated boundaries, or coming out
//! a suspicious length do not go to the customer.
//!
//! This used to feed an approval queue: a risky draft was held, a human
//! approved or rejected it from Discord or a web page, and only then did it
//! send. The queue is gone. A risky draft now trips the same human-handoff
//! path the model can trigger itself — the customer gets a holding sentence,
//! the tenant gets paged, and the conversation is theirs.
//!
//! The per-rule `ApprovalPolicy` (`Auto` / `Always` / `NoGate`) is gone with
//! it, along with the `ALLOW_NO_GATE` operator override. The gate is always
//! on; there is nothing to configure and no way to turn it off.

use crate::types::{PersonaConfig, PersonaSource};

/// Why a draft was withheld. Recorded in logs and surfaced on the
/// handoff page so the tenant knows what tripped.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RiskReason {
    Length,
    MoneyWord,
    Commitment,
    PersonaDrift,
}

impl RiskReason {
    pub fn as_str(self) -> &'static str {
        match self {
            RiskReason::Length => "length",
            RiskReason::MoneyWord => "money_word",
            RiskReason::Commitment => "commitment",
            RiskReason::PersonaDrift => "persona_drift",
        }
    }

    /// One-line explanation for the tenant-facing handoff page.
    pub fn label(self) -> &'static str {
        match self {
            RiskReason::Length => "the draft was unusually short or long",
            RiskReason::MoneyWord => "the draft mentioned money",
            RiskReason::Commitment => "the draft made a commitment",
            RiskReason::PersonaDrift => "the draft strayed outside your stated boundaries",
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
pub fn signal(draft: &str, persona: &PersonaConfig) -> Option<RiskReason> {
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
    if risk_persona_drift(&lower, persona) {
        return Some(RiskReason::PersonaDrift);
    }
    None
}

fn risk_length(draft: &str) -> bool {
    let n = draft.chars().count();
    !(MIN_LEN..=MAX_LEN).contains(&n)
}

fn risk_persona_drift(draft_lower: &str, persona: &PersonaConfig) -> bool {
    let PersonaSource::Builder(b) = &persona.source else {
        return false;
    };
    let never = b.never.trim();
    if !never.is_empty() && draft_lower.contains(&never.to_lowercase()) {
        return true;
    }
    b.off_topics
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .any(|t| draft_lower.contains(&t.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PersonaBuilder, PersonaSafety};

    fn persona_default() -> PersonaConfig {
        PersonaConfig::default()
    }

    fn persona_builder(off_topics: Vec<&str>, never: &str) -> PersonaConfig {
        PersonaConfig {
            source: PersonaSource::Builder(PersonaBuilder {
                catch_phrases: vec![],
                off_topics: off_topics.into_iter().map(String::from).collect(),
                never: never.into(),
                ..Default::default()
            }),
            safety: PersonaSafety::default(),
        }
    }

    #[test]
    fn plain_draft_passes() {
        assert_eq!(
            signal("Sure, we're open until six today.", &persona_default()),
            None
        );
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
                signal(draft, &persona_default()),
                Some(RiskReason::MoneyWord),
                "expected money signal for {draft:?}"
            );
        }
    }

    #[test]
    fn commitment_words_trip_the_gate() {
        assert_eq!(
            signal("I guarantee it ships today.", &persona_default()),
            Some(RiskReason::Commitment)
        );
        assert_eq!(
            signal("We'll have it ready by tomorrow.", &persona_default()),
            Some(RiskReason::Commitment)
        );
    }

    #[test]
    fn length_is_checked_before_content() {
        assert_eq!(signal("ok", &persona_default()), Some(RiskReason::Length));
        let long = "a".repeat(601);
        assert_eq!(signal(&long, &persona_default()), Some(RiskReason::Length));
    }

    #[test]
    fn persona_drift_matches_off_topics_and_never() {
        let p = persona_builder(vec!["legal advice"], "medical diagnosis");
        assert_eq!(
            signal("Here is some Legal Advice for you.", &p),
            Some(RiskReason::PersonaDrift)
        );
        assert_eq!(
            signal("That sounds like a Medical Diagnosis.", &p),
            Some(RiskReason::PersonaDrift)
        );
        assert_eq!(signal("We open at nine tomorrow-ish.", &p), None);
    }

    #[test]
    fn custom_persona_has_no_drift_signal() {
        let p = PersonaConfig {
            source: PersonaSource::Custom("raw prompt".into()),
            safety: PersonaSafety::default(),
        };
        assert_eq!(signal("Anything at all goes here.", &p), None);
    }
}
