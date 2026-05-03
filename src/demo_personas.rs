//! Personas exposed by the public live-demo chat (`/demo/chat`).
//!
//! Bundles the existing tenant-facing `PersonaPreset` library (florist,
//! salon, cafe, clinic) with a "Concierge talking about itself" entry,
//! and ships each one with a slug, label, short description, full
//! system prompt, and a starter greeting. The welcome page embeds this
//! list as JSON so the chat modal can render a picker, reset the
//! transcript on switch, and let visitors view the active prompt.

use serde::Serialize;

use crate::types::PersonaPreset;

/// Slug used when no persona is sent or when the slug doesn't match
/// any known preset. The handler falls back to this so a malformed
/// client doesn't surface a 400.
pub const DEFAULT_SLUG: &str = "concierge";

/// One row of the demo persona table. Cheap-copy: every field is a
/// `&'static str`, including the prompt body.
#[derive(Clone, Copy, Serialize)]
pub struct DemoPersona {
    pub slug: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub prompt: &'static str,
    pub greeting: &'static str,
}

// Concierge's demo persona is "talking about itself" — the editable
// middle of the envelope. The shared POSTAMBLE in `crate::prompt`
// supplies the brevity / no-invented-facts / no-actions / jailbreak
// rails; what's in `CONCIERGE_DEMO` is just the on-topic guard and
// the product copy.
const CONCIERGE_PERSONA: DemoPersona = DemoPersona {
    slug: DEFAULT_SLUG,
    label: "Concierge",
    description: "Talks about Concierge — what I am, channels, pricing, setup.",
    prompt: crate::prompt::CONCIERGE_DEMO,
    greeting: "Hi! I'm Concierge. Ask me what I do, which channels I cover, how pricing works, or how to set me up.",
};

fn greeting_for(p: PersonaPreset) -> &'static str {
    match p {
        PersonaPreset::FriendlyFlorist => {
            "Hi there! Welcome to the shop — what kind of flowers can we put together for you?"
        }
        PersonaPreset::ProfessionalSalon => "Thanks for reaching out. How can we help you today?",
        PersonaPreset::PlayfulCafe => "hi 👋 what can we get u? ☕🥐",
        PersonaPreset::OldSchoolClinic => "Good day. How may we assist you?",
    }
}

/// All demo personas in display order. Concierge first; the four tenant
/// presets after, in `PersonaPreset::ALL` order.
pub fn all() -> Vec<DemoPersona> {
    let mut out = Vec::with_capacity(1 + PersonaPreset::ALL.len());
    out.push(CONCIERGE_PERSONA);
    for &p in PersonaPreset::ALL {
        out.push(DemoPersona {
            slug: p.slug(),
            label: p.label(),
            description: p.description(),
            prompt: p.prompt(),
            greeting: greeting_for(p),
        });
    }
    out
}

/// Resolve a slug to a persona, falling back to Concierge when the slug
/// is empty or unknown. Returns owned `DemoPersona` (cheap — all fields
/// are `&'static`).
pub fn lookup(slug: &str) -> DemoPersona {
    if slug.is_empty() || slug == DEFAULT_SLUG {
        return CONCIERGE_PERSONA;
    }
    if let Some(p) = PersonaPreset::from_slug(slug) {
        return DemoPersona {
            slug: p.slug(),
            label: p.label(),
            description: p.description(),
            prompt: p.prompt(),
            greeting: greeting_for(p),
        };
    }
    CONCIERGE_PERSONA
}
