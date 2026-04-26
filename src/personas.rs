//! Curated persona presets and the builder → prompt generator.
//!
//! Each preset bundles three things:
//! 1. A persona prompt string (what the LLM is told about voice/boundaries).
//! 2. A small set of default reply rules (canned + prompt) that get copied
//!    into every newly connected channel's `ReplyConfig`.
//! 3. Display label + description for the wizard's preset picker.
//!
//! Adding a preset: add a variant to `PersonaPreset` in `types.rs` and a
//! match arm in every method here. The compiler enforces completeness.

use crate::types::{
    ApprovalPolicy, PersonaBuilder, PersonaPreset, ReplyMatcher, ReplyResponse, ReplyRule,
};

impl PersonaPreset {
    pub const ALL: &'static [PersonaPreset] = &[
        PersonaPreset::FriendlyFlorist,
        PersonaPreset::ProfessionalSalon,
        PersonaPreset::PlayfulCafe,
        PersonaPreset::OldSchoolClinic,
    ];

    pub fn slug(&self) -> &'static str {
        match self {
            PersonaPreset::FriendlyFlorist => "friendly_florist",
            PersonaPreset::ProfessionalSalon => "professional_salon",
            PersonaPreset::PlayfulCafe => "playful_cafe",
            PersonaPreset::OldSchoolClinic => "old_school_clinic",
        }
    }

    pub fn from_slug(s: &str) -> Option<PersonaPreset> {
        PersonaPreset::ALL.iter().copied().find(|p| p.slug() == s)
    }

    pub fn label(&self) -> &'static str {
        match self {
            PersonaPreset::FriendlyFlorist => "Friendly Florist",
            PersonaPreset::ProfessionalSalon => "Professional Salon",
            PersonaPreset::PlayfulCafe => "Playful Cafe",
            PersonaPreset::OldSchoolClinic => "Old-school Clinic",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PersonaPreset::FriendlyFlorist => {
                "Warm and chatty. Defers pricing and delivery details to the owner."
            }
            PersonaPreset::ProfessionalSalon => {
                "Concise and professional. Books appointments, recites cancellation policy."
            }
            PersonaPreset::PlayfulCafe => {
                "Playful with emoji. Answers hours and menu questions, pings the owner for orders."
            }
            PersonaPreset::OldSchoolClinic => {
                "Formal and polite. Routes emergencies to 911, never diagnoses."
            }
        }
    }

    pub fn prompt(&self) -> &'static str {
        match self {
            PersonaPreset::FriendlyFlorist => FRIENDLY_FLORIST_PROMPT,
            PersonaPreset::ProfessionalSalon => PROFESSIONAL_SALON_PROMPT,
            PersonaPreset::PlayfulCafe => PLAYFUL_CAFE_PROMPT,
            PersonaPreset::OldSchoolClinic => OLD_SCHOOL_CLINIC_PROMPT,
        }
    }

    /// Default reply rules seeded into a new channel's ReplyConfig when this
    /// preset is selected. The list does NOT include the default rule itself
    /// (that's a separate field on ReplyConfig).
    pub fn default_rules(&self) -> Vec<ReplyRule> {
        match self {
            PersonaPreset::FriendlyFlorist => vec![
                ReplyRule {
                    id: "delivery".to_string(),
                    label: "Delivery questions".to_string(),
                    matcher: ReplyMatcher::Prompt {
                        description: "asks about delivery, shipping, or how to receive an order"
                            .to_string(),
                        embedding: Vec::new(),
                        embedding_model: String::new(),
                        threshold: crate::types::default_match_threshold(),
                    },
                    response: ReplyResponse::Prompt {
                        text: "Confirm we deliver locally, ask for delivery address and date, \
                         and say a human will confirm the slot."
                            .to_string(),
                    },
                    approval: ApprovalPolicy::Auto,
                },
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
                        text: "Politely defer pricing to the owner; ask what arrangement and \
                         budget they have in mind so we can come back with a quote."
                            .to_string(),
                    },
                    approval: ApprovalPolicy::Auto,
                },
            ],
            PersonaPreset::ProfessionalSalon => vec![
                ReplyRule {
                    id: "booking".to_string(),
                    label: "Booking requests".to_string(),
                    matcher: ReplyMatcher::Prompt {
                        description: "wants to book, reschedule, or check availability for an \
                                      appointment"
                            .to_string(),
                        embedding: Vec::new(),
                        embedding_model: String::new(),
                        threshold: crate::types::default_match_threshold(),
                    },
                    response: ReplyResponse::Prompt {
                        text: "Ask which service and which day they prefer; mention a stylist \
                         will confirm shortly."
                            .to_string(),
                    },
                    approval: ApprovalPolicy::Auto,
                },
                ReplyRule {
                    id: "cancellation".to_string(),
                    label: "Cancellation policy".to_string(),
                    matcher: ReplyMatcher::StaticText {
                        keywords: vec!["cancel".to_string(), "refund".to_string()],
                    },
                    response: ReplyResponse::Canned {
                        text: "Cancellations are free up to 24 hours before your appointment. \
                         Within 24 hours, a 50% fee applies. Reply with your appointment time \
                         and we'll take care of it."
                            .to_string(),
                    },
                    approval: ApprovalPolicy::Auto,
                },
            ],
            PersonaPreset::PlayfulCafe => vec![
                ReplyRule {
                    id: "hours".to_string(),
                    label: "Hours / location".to_string(),
                    matcher: ReplyMatcher::StaticText {
                        keywords: vec![
                            "hours".to_string(),
                            "open".to_string(),
                            "closed".to_string(),
                            "address".to_string(),
                        ],
                    },
                    response: ReplyResponse::Canned {
                        text: "We're open 7am-7pm every day. Come say hi! ☕".to_string(),
                    },
                    approval: ApprovalPolicy::Auto,
                },
                ReplyRule {
                    id: "menu".to_string(),
                    label: "Menu questions".to_string(),
                    matcher: ReplyMatcher::Prompt {
                        description: "asks about the menu, drinks, food, or what we serve"
                            .to_string(),
                        embedding: Vec::new(),
                        embedding_model: String::new(),
                        threshold: crate::types::default_match_threshold(),
                    },
                    response: ReplyResponse::Prompt {
                        text: "Cheerfully describe our coffee + pastry lineup and invite them \
                         in. If they ask about specifics we don't know, offer to ping the owner."
                            .to_string(),
                    },
                    approval: ApprovalPolicy::Auto,
                },
            ],
            PersonaPreset::OldSchoolClinic => vec![
                ReplyRule {
                    id: "emergency".to_string(),
                    label: "Emergencies".to_string(),
                    matcher: ReplyMatcher::StaticText {
                        keywords: vec![
                            "emergency".to_string(),
                            "urgent".to_string(),
                            "bleeding".to_string(),
                            "chest pain".to_string(),
                        ],
                    },
                    response: ReplyResponse::Canned {
                        text: "If this is a medical emergency, please call your local emergency \
                         services or visit the nearest emergency room immediately. We are \
                         unable to provide emergency care via message."
                            .to_string(),
                    },
                    approval: ApprovalPolicy::Auto,
                },
                ReplyRule {
                    id: "appointment".to_string(),
                    label: "Appointment requests".to_string(),
                    matcher: ReplyMatcher::Prompt {
                        description: "asks to schedule, book, or change an appointment".to_string(),
                        embedding: Vec::new(),
                        embedding_model: String::new(),
                        threshold: crate::types::default_match_threshold(),
                    },
                    response: ReplyResponse::Prompt {
                        text: "Acknowledge the request, ask for the patient's name and a \
                         preferred day, and note our front desk will confirm during business \
                         hours."
                            .to_string(),
                    },
                    approval: ApprovalPolicy::Auto,
                },
            ],
        }
    }
}

/// Pure function: render a `PersonaBuilder` to its prompt text. Used both by
/// `PersonaConfig::active_prompt()` and the admin UI's live preview.
pub fn generate(b: &PersonaBuilder) -> String {
    let mut parts: Vec<String> =
        vec!["You are a helpful messaging assistant for a small business.".to_string()];

    if !b.biz_type.is_empty() {
        let loc = if b.city.is_empty() {
            String::new()
        } else {
            format!(" in {}", b.city)
        };
        parts.push(format!("The business is a {}{}.", b.biz_type, loc));
    }

    if !b.tone.is_empty() {
        parts.push(format!("Tone: {}. Match this tone in every reply.", b.tone));
    }

    if !b.catch_phrases.is_empty() {
        let phrases: Vec<String> = b
            .catch_phrases
            .iter()
            .filter(|p| !p.trim().is_empty())
            .map(|p| format!("\"{}\"", p.trim()))
            .collect();
        if !phrases.is_empty() {
            parts.push(format!(
                "Naturally weave in these signature phrases when fitting: {}.",
                phrases.join(", ")
            ));
        }
    }

    if !b.off_topics.is_empty() {
        let topics: Vec<String> = b
            .off_topics
            .iter()
            .filter(|t| !t.trim().is_empty())
            .map(|t| t.trim().to_string())
            .collect();
        if !topics.is_empty() {
            parts.push(format!(
                "Do not engage on these off-topic subjects: {}. Politely redirect to the \
                 business at hand.",
                topics.join("; ")
            ));
        }
    }

    if !b.never.is_empty() {
        parts.push(format!(
            "Never {}. If asked, politely defer to a human.",
            b.never
        ));
    }

    parts.push("Keep replies short (1-3 sentences). If unsure, hand off to the owner.".to_string());
    parts.join("\n")
}

const FRIENDLY_FLORIST_PROMPT: &str = "\
You are a warm, friendly assistant for a small florist. \
Speak like a kind shopkeeper who's known the customer for years. \
Confirm you'd love to help, ask one clarifying question if you need it, \
and let the customer know the owner will follow up to confirm details. \
Never quote firm prices; never promise a delivery date or arrangement detail. \
Politely decline non-flower topics like politics or relationship advice. \
Keep replies short (1-3 sentences) and kind.";

const PROFESSIONAL_SALON_PROMPT: &str = "\
You are a concise, professional assistant for a hair and beauty salon. \
Greet briefly, confirm what's possible, and ask for the missing detail (service, day, stylist). \
Defer firm bookings to the salon's front desk. \
Never give medical advice, hair-loss diagnoses, or product allergy guidance. \
Keep replies short (1-3 sentences) and to the point.";

const PLAYFUL_CAFE_PROMPT: &str = "\
You are a playful, upbeat assistant for a neighborhood cafe. \
Use emoji sparingly (☕ or 🌿) when it fits. \
Answer simple questions about hours and the menu cheerfully; for orders, ask the customer to \
swing by or say a human will confirm. \
Never give nutrition or allergy advice. \
Keep replies short (1-2 sentences) and warm.";

const OLD_SCHOOL_CLINIC_PROMPT: &str = "\
You are a polite, formal assistant for a small medical clinic. \
Address the patient respectfully. \
For appointments, ask for the patient's name and preferred day; confirm a human will follow up \
during clinic hours. \
Never diagnose, prescribe, suggest medications, or interpret symptoms. \
For anything that sounds urgent, advise contacting emergency services. \
Keep replies short (1-3 sentences) and respectful.";
