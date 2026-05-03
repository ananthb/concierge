//! Risk-gate decision logic for AI-generated drafts.
//!
//! Given a rule's `ApprovalPolicy` and a freshly generated draft, decide
//! whether to send immediately or queue for human approval. The cheap
//! risk-gate signals (length, money words, commitment words, persona drift)
//! run synchronously on the inbound request: no model calls, no I/O.
//!
//! The operator-level `ALLOW_NO_GATE` env var passes through as a boolean.
//! When false, `NoGate` rules fall back to `Auto` semantics (the gate runs)
//! so the operator's policy choice always wins over the tenant's.

use crate::types::{ApprovalPolicy, PersonaConfig, PersonaSource, QueueReason, ReplyRule};
use worker::Env;

/// True if the operator's deploy has opted into the unsafe `NoGate` policy.
/// Defaults to false: tenants can't pick `NoGate` unless the operator
/// explicitly sets `ALLOW_NO_GATE=true`.
pub fn allow_no_gate(env: &Env) -> bool {
    env.var("ALLOW_NO_GATE")
        .map(|v| v.to_string() == "true")
        .unwrap_or(false)
}

#[derive(Debug, PartialEq, Eq)]
pub enum ApprovalDecision {
    SendNow,
    Queue { reason: QueueReason },
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

pub fn decide(
    rule: &ReplyRule,
    draft: &str,
    persona: &PersonaConfig,
    allow_no_gate: bool,
) -> ApprovalDecision {
    match &rule.approval {
        ApprovalPolicy::Always => ApprovalDecision::Queue {
            reason: QueueReason::RuleAlways,
        },
        ApprovalPolicy::NoGate { .. } if allow_no_gate => ApprovalDecision::SendNow,
        // Operator override: NoGate without env permission falls through to Auto.
        ApprovalPolicy::Auto | ApprovalPolicy::NoGate { .. } => match risk_signal(draft, persona) {
            Some(reason) => ApprovalDecision::Queue { reason },
            None => ApprovalDecision::SendNow,
        },
    }
}

fn risk_signal(draft: &str, persona: &PersonaConfig) -> Option<QueueReason> {
    if risk_length(draft) {
        return Some(QueueReason::RiskLength);
    }
    let lower = draft.to_lowercase();
    if MONEY_WORDS.iter().any(|w| lower.contains(*w)) {
        return Some(QueueReason::RiskMoneyWord);
    }
    if COMMITMENT_WORDS.iter().any(|w| lower.contains(*w)) {
        return Some(QueueReason::RiskCommitment);
    }
    if risk_persona_drift(&lower, persona) {
        return Some(QueueReason::RiskPersonaDrift);
    }
    None
}

fn risk_length(draft: &str) -> bool {
    let n = draft.chars().count();
    n < MIN_LEN || n > MAX_LEN
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
    use crate::types::{
        NoGateAcceptance, PersonaBuilder, PersonaSafety, ReplyMatcher, ReplyResponse,
    };

    fn rule(policy: ApprovalPolicy) -> ReplyRule {
        ReplyRule {
            id: "r".into(),
            label: "r".into(),
            matcher: ReplyMatcher::Default,
            response: ReplyResponse::Prompt { text: "x".into() },
            approval: policy,
        }
    }

    fn persona_preset() -> PersonaConfig {
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

    fn safe_draft() -> &'static str {
        "Sure thing. We can take care of that for you."
    }

    #[test]
    fn always_policy_queues_with_rule_always() {
        let r = rule(ApprovalPolicy::Always);
        assert_eq!(
            decide(&r, safe_draft(), &persona_preset(), true),
            ApprovalDecision::Queue {
                reason: QueueReason::RuleAlways
            }
        );
    }

    #[test]
    fn auto_policy_with_safe_draft_sends() {
        let r = rule(ApprovalPolicy::Auto);
        assert_eq!(
            decide(&r, safe_draft(), &persona_preset(), true),
            ApprovalDecision::SendNow
        );
    }

    #[test]
    fn auto_policy_short_draft_queues_length() {
        let r = rule(ApprovalPolicy::Auto);
        assert_eq!(
            decide(&r, "ok", &persona_preset(), true),
            ApprovalDecision::Queue {
                reason: QueueReason::RiskLength
            }
        );
    }

    #[test]
    fn auto_policy_long_draft_queues_length() {
        let r = rule(ApprovalPolicy::Auto);
        let long: String = "a".repeat(MAX_LEN + 1);
        assert_eq!(
            decide(&r, &long, &persona_preset(), true),
            ApprovalDecision::Queue {
                reason: QueueReason::RiskLength
            }
        );
    }

    #[test]
    fn auto_policy_money_word_queues_money() {
        let r = rule(ApprovalPolicy::Auto);
        assert_eq!(
            decide(
                &r,
                "Our price for that arrangement is reasonable.",
                &persona_preset(),
                true
            ),
            ApprovalDecision::Queue {
                reason: QueueReason::RiskMoneyWord
            }
        );
    }

    #[test]
    fn auto_policy_commitment_queues_commitment() {
        let r = rule(ApprovalPolicy::Auto);
        assert_eq!(
            decide(
                &r,
                "Yes, we will deliver by Friday for sure.",
                &persona_preset(),
                true
            ),
            ApprovalDecision::Queue {
                reason: QueueReason::RiskCommitment
            }
        );
    }

    #[test]
    fn auto_policy_persona_drift_queues_drift() {
        let r = rule(ApprovalPolicy::Auto);
        let p = persona_builder(vec!["politics"], "");
        assert_eq!(
            decide(&r, "Let me share my views on politics with you.", &p, true),
            ApprovalDecision::Queue {
                reason: QueueReason::RiskPersonaDrift
            }
        );
    }

    #[test]
    fn auto_policy_persona_never_queues_drift() {
        let r = rule(ApprovalPolicy::Auto);
        let p = persona_builder(vec![], "diagnose conditions");
        assert_eq!(
            decide(&r, "I will diagnose conditions for you remotely.", &p, true),
            ApprovalDecision::Queue {
                reason: QueueReason::RiskPersonaDrift
            }
        );
    }

    #[test]
    fn no_gate_with_operator_allow_sends_regardless() {
        let r = rule(ApprovalPolicy::NoGate {
            acceptance: NoGateAcceptance {
                accepted_at: "2026-04-27T00:00:00Z".into(),
                accepted_by: "owner@example.com".into(),
                version: "v1".into(),
            },
        });
        assert_eq!(
            decide(
                &r,
                "Our price for that is $500, guaranteed by Friday.",
                &persona_preset(),
                true
            ),
            ApprovalDecision::SendNow
        );
    }

    #[test]
    fn no_gate_without_operator_allow_falls_through_to_auto() {
        let r = rule(ApprovalPolicy::NoGate {
            acceptance: NoGateAcceptance {
                accepted_at: "2026-04-27T00:00:00Z".into(),
                accepted_by: "owner@example.com".into(),
                version: "v1".into(),
            },
        });
        assert_eq!(
            decide(
                &r,
                "Our price for that is reasonable.",
                &persona_preset(),
                false
            ),
            ApprovalDecision::Queue {
                reason: QueueReason::RiskMoneyWord
            }
        );
    }

    #[test]
    fn money_signal_uses_lowercase_match() {
        let r = rule(ApprovalPolicy::Auto);
        assert_eq!(
            decide(&r, "What about the PRICE?", &persona_preset(), true),
            ApprovalDecision::Queue {
                reason: QueueReason::RiskMoneyWord
            }
        );
    }

    #[test]
    fn empty_off_topic_does_not_match() {
        let r = rule(ApprovalPolicy::Auto);
        let p = persona_builder(vec!["", "   "], "");
        assert_eq!(
            decide(&r, safe_draft(), &p, true),
            ApprovalDecision::SendNow
        );
    }
}
