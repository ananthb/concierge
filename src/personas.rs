//! Persona archetypes (voice only) + the builder → prompt generator.
//!
//! `PersonaPreset` is now a *voice archetype* (Friendly / Professional /
//! Playful / Formal) — decoupled from any business type. Curated
//! personas with sample business fields live in the D1 `personas`
//! catalog (see `migrations/0001_create_schema.sql`). Adding a new
//! archetype is a code change here + a `crate::prompt::VOICE_*`
//! constant; adding a new *persona* (an archetype + business sample)
//! is a row in the catalog with no code change.

use crate::types::{
    ApprovalPolicy, PersonaBuilder, PersonaPreset, ReplyMatcher, ReplyResponse, ReplyRule,
};

impl PersonaPreset {
    pub const ALL: &'static [PersonaPreset] = &[
        PersonaPreset::Friendly,
        PersonaPreset::Professional,
        PersonaPreset::Playful,
        PersonaPreset::Formal,
    ];

    pub fn slug(&self) -> &'static str {
        match self {
            PersonaPreset::Friendly => "friendly",
            PersonaPreset::Professional => "professional",
            PersonaPreset::Playful => "playful",
            PersonaPreset::Formal => "formal",
        }
    }

    pub fn from_slug(s: &str) -> Option<PersonaPreset> {
        PersonaPreset::ALL.iter().copied().find(|p| p.slug() == s)
    }

    pub fn label(&self) -> &'static str {
        match self {
            PersonaPreset::Friendly => "Friendly",
            PersonaPreset::Professional => "Professional",
            PersonaPreset::Playful => "Playful",
            PersonaPreset::Formal => "Formal",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PersonaPreset::Friendly => "Warm, kind, and conversational.",
            PersonaPreset::Professional => "Concise and businesslike.",
            PersonaPreset::Playful => "Playful and upbeat with a light touch of emoji.",
            PersonaPreset::Formal => "Polite and formal — a measured, considered tone.",
        }
    }

    /// The voice description plugged into the middle by `generate`.
    pub fn voice(&self) -> &'static str {
        match self {
            PersonaPreset::Friendly => crate::prompt::VOICE_FRIENDLY,
            PersonaPreset::Professional => crate::prompt::VOICE_PROFESSIONAL,
            PersonaPreset::Playful => crate::prompt::VOICE_PLAYFUL,
            PersonaPreset::Formal => crate::prompt::VOICE_FORMAL,
        }
    }

    /// Default reply rules seeded into a new channel's `ReplyConfig`.
    /// Thin and archetype-flavoured — a Pricing rule and an After-hours
    /// rule, voiced per archetype. Tenants curate the rest themselves
    /// after onboarding (delivery for florists, booking for salons,
    /// emergencies for clinics, etc.).
    pub fn default_rules(&self) -> Vec<ReplyRule> {
        let pricing_response = match self {
            PersonaPreset::Friendly => "Confirm we'd love to help, ask what they have in mind, and let them know the owner will follow up with a quote.",
            PersonaPreset::Professional => "Acknowledge the question, ask for the missing detail (what they need, by when), and confirm a human will respond with a price.",
            PersonaPreset::Playful => "Stay upbeat, ask what they're after, and say someone will come back with the number soon.",
            PersonaPreset::Formal => "Acknowledge the inquiry politely, ask for the relevant detail, and indicate that a member of the team will respond with the price.",
        };
        let after_hours_response = match self {
            PersonaPreset::Friendly => "Thanks for reaching out — we're closed right now but we'll get back to you first thing.",
            PersonaPreset::Professional => "We're outside business hours; we'll respond when we're back.",
            PersonaPreset::Playful => "Catching some Zzz right now 💤 — we'll write back when we're up!",
            PersonaPreset::Formal => "Thank you for your message. We are currently outside business hours and will respond at our earliest opportunity.",
        };
        vec![
            ReplyRule {
                id: "pricing".to_string(),
                label: "Pricing questions".to_string(),
                matcher: ReplyMatcher::Prompt {
                    description: "asks about price, cost, or how much something is".to_string(),
                    embedding: Vec::new(),
                    embedding_model: String::new(),
                    threshold: crate::types::default_match_threshold(),
                },
                response: ReplyResponse::Prompt {
                    text: pricing_response.to_string(),
                },
                approval: ApprovalPolicy::Auto,
            },
            ReplyRule {
                id: "after_hours".to_string(),
                label: "After-hours messages".to_string(),
                matcher: ReplyMatcher::Keyword {
                    keywords: vec![
                        "after hours".to_string(),
                        "closed".to_string(),
                        "still open".to_string(),
                    ],
                },
                response: ReplyResponse::Canned {
                    text: after_hours_response.to_string(),
                },
                approval: ApprovalPolicy::Auto,
            },
        ]
    }
}

/// Pure function: render a `PersonaBuilder` to its prompt text — the
/// editable middle that gets envelope-wrapped at AI-call time.
///
/// Shape:
/// ```text
/// Business: {biz_name}, a {biz_type}{ in {city}}.
///
/// {voice_for(archetype)}
///
/// Signature phrases to weave in naturally: "...", "...".
/// Stay off these subjects (redirect to the business): topic; topic.
/// Never {never}.
/// ```
///
/// Sections after the voice line are emitted only when their builder
/// field is non-empty.
pub fn generate(b: &PersonaBuilder) -> String {
    let mut parts: Vec<String> = Vec::new();

    let biz_line = match (b.biz_name.trim(), b.biz_type.trim(), b.city.trim()) {
        ("", "", _) => "Business: a small business.".to_string(),
        ("", t, "") => format!("Business: a {t}."),
        ("", t, c) => format!("Business: a {t} in {c}."),
        (n, "", "") => format!("Business: {n}."),
        (n, "", c) => format!("Business: {n}, in {c}."),
        (n, t, "") => format!("Business: {n}, a {t}."),
        (n, t, c) => format!("Business: {n}, a {t} in {c}."),
    };
    parts.push(biz_line);

    parts.push(b.archetype.voice().to_string());

    let phrases: Vec<String> = b
        .catch_phrases
        .iter()
        .filter(|p| !p.trim().is_empty())
        .map(|p| format!("\"{}\"", p.trim()))
        .collect();
    if !phrases.is_empty() {
        parts.push(format!(
            "Signature phrases to weave in naturally: {}.",
            phrases.join(", ")
        ));
    }

    let topics: Vec<String> = b
        .off_topics
        .iter()
        .filter(|t| !t.trim().is_empty())
        .map(|t| t.trim().to_string())
        .collect();
    if !topics.is_empty() {
        parts.push(format!(
            "Stay off these subjects (redirect to the business): {}.",
            topics.join("; ")
        ));
    }

    if !b.never.trim().is_empty() {
        parts.push(format!("Never {}.", b.never.trim()));
    }

    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PersonaBuilder;

    fn minimal() -> PersonaBuilder {
        PersonaBuilder {
            archetype: PersonaPreset::Friendly,
            biz_name: "Petals & Stems".to_string(),
            biz_type: "florist".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn generate_minimal_builder() {
        let p = generate(&minimal());
        assert!(p.contains("Business: Petals & Stems, a florist."));
        assert!(p.contains("Voice: warm")); // Friendly voice line
        assert!(!p.contains("Signature phrases"));
        assert!(!p.contains("Stay off these subjects"));
        assert!(!p.contains("Never"));
    }

    #[test]
    fn generate_full_builder() {
        let mut b = minimal();
        b.city = "Bandra".to_string();
        b.catch_phrases = vec!["bloom your day".to_string(), "thanks petal!".to_string()];
        b.off_topics = vec!["politics".to_string(), "relationships".to_string()];
        b.never = "quote firm prices".to_string();
        let p = generate(&b);
        assert!(p.contains("Business: Petals & Stems, a florist in Bandra."));
        assert!(p.contains(r#""bloom your day", "thanks petal!""#));
        assert!(p.contains("politics; relationships"));
        assert!(p.contains("Never quote firm prices."));
    }

    #[test]
    fn archetype_voice_threaded_per_archetype() {
        for archetype in PersonaPreset::ALL {
            let mut b = minimal();
            b.archetype = *archetype;
            let p = generate(&b);
            assert!(
                p.contains(archetype.voice()),
                "voice for {:?} not present in generated prompt",
                archetype
            );
        }
    }

    #[test]
    fn empty_builder_still_gets_a_business_line() {
        let p = generate(&PersonaBuilder::default());
        assert!(p.starts_with("Business:"));
        assert!(p.contains(crate::prompt::VOICE_FRIENDLY));
    }
}
