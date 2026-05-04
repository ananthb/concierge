//! Persona archetypes (voice only) + the builder → prompt generator.
//!
//! Archetypes (voice / tone) are now dynamic and managed in the D1
//! `archetypes` table. Tenants build their persona by referencing an
//! archetype and providing their business details.

use crate::types::PersonaBuilder;

/// Pure function: render a `PersonaBuilder` to its prompt text. The
/// editable middle that gets envelope-wrapped at AI-call time.
///
/// Shape:
/// ```text
/// Business: {biz_name}, a {biz_type}{ in {city}}.
///
/// {voice_prompt}
///
/// Signature phrases to weave in naturally: "...", "...".
/// Stay off these subjects (redirect to the business): topic; topic.
/// Never {never}.
/// ```
///
/// Sections after the voice line are emitted only when their builder
/// field is non-empty.
pub fn generate(b: &PersonaBuilder, voice_prompt: &str) -> String {
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

    parts.push(voice_prompt.trim().to_string());

    // Goal is policy-adjacent to voice and is *always* emitted. When
    // blank, we substitute a sensible default so the rendered middle is
    // deterministic (and the safety classifier sees the same prompt the
    // model will). Order matters: keep Goal next to Voice so the model
    // reads "what tone, what outcome" together.
    if !b.goal.trim().is_empty() {
        if !b.goal_url.trim().is_empty() {
            parts.push(format!(
                "Goal: guide the customer to {} at {}.",
                b.goal.trim(),
                b.goal_url.trim()
            ));
        } else {
            parts.push(format!(
                "Goal: guide the customer to {}. Do not invent or include a URL or path for this goal.",
                b.goal.trim()
            ));
        }
    } else {
        parts.push(
            "Goal: answer the customer's question and let them know a human will follow up."
                .to_string(),
        );
    }

    if !b.hours.trim().is_empty() {
        parts.push(format!("Hours: {}.", b.hours.trim()));
    }

    if !b.catch_phrases.is_empty() {
        let clean: Vec<&str> = b
            .catch_phrases
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !clean.is_empty() {
            let quoted: Vec<String> = clean.into_iter().map(|s| format!("\"{s}\"")).collect();
            parts.push(format!(
                "Signature phrases to weave in naturally: {}.",
                quoted.join(", ")
            ));
        }
    }

    if !b.off_topics.is_empty() {
        let clean: Vec<&str> = b
            .off_topics
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !clean.is_empty() {
            parts.push(format!(
                "Stay off these subjects (redirect to the business): {}.",
                clean.join("; ")
            ));
        }
    }

    if !b.never.trim().is_empty() {
        parts.push(format!("Never {}.", b.never.trim()));
    }

    if !b.handoff_conditions.is_empty() {
        let clean: Vec<&str> = b
            .handoff_conditions
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !clean.is_empty() {
            parts.push(format!(
                "Hand off to a human if any of these come up: {}.",
                clean.join("; ")
            ));
        }
    }

    parts.join("\n\n")
}

/// Scrub a goal URL from form input.
pub fn sanitize_goal_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('/') {
        return trimmed.to_string();
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("https://") && trimmed.len() > "https://".len() {
        return trimmed.to_string();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PersonaBuilder;

    fn minimal() -> PersonaBuilder {
        PersonaBuilder {
            archetype_slug: "friendly".to_string(),
            biz_name: "Petals & Stems".to_string(),
            biz_type: "florist".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn generate_minimal_builder() {
        let p = generate(&minimal(), "Voice: warm.");
        assert!(p.contains("Business: Petals & Stems, a florist."));
        assert!(p.contains("Voice: warm."));
        assert!(!p.contains("Signature phrases"));
        assert!(!p.contains("Stay off these subjects"));
        assert!(!p.contains("Never"));
    }

    #[test]
    fn generate_full_builder() {
        let mut b = minimal();
        b.city = "Bandra".to_string();
        b.hours = "Tue–Sun 9am–7pm".to_string();
        b.goal = "book a delivery slot".to_string();
        b.goal_url = "/book".to_string();
        b.catch_phrases = vec!["bloom your day".to_string(), "thanks petal!".to_string()];
        b.off_topics = vec!["politics".to_string(), "relationships".to_string()];
        b.never = "quote firm prices".to_string();
        b.handoff_conditions = vec!["weddings".to_string(), "complaints".to_string()];
        let p = generate(&b, "Voice: warm.");
        assert!(p.contains("Business: Petals & Stems, a florist in Bandra."));
        assert!(p.contains("Hours: Tue–Sun 9am–7pm."));
        assert!(p.contains("Goal: guide the customer to book a delivery slot at /book."));
        assert!(p.contains(r#""bloom your day", "thanks petal!""#));
        assert!(p.contains("politics; relationships"));
        assert!(p.contains("Never quote firm prices."));
        assert!(p.contains("Hand off to a human if any of these come up: weddings; complaints."));
    }

    #[test]
    fn generate_omits_hours_when_blank() {
        let p = generate(&minimal(), "Voice: warm.");
        assert!(!p.contains("Hours:"));
    }

    #[test]
    fn generate_includes_goal_when_set() {
        let mut b = minimal();
        b.goal = "book a delivery slot".to_string();
        let p = generate(&b, "Voice: warm.");
        assert!(p.contains("Goal: guide the customer to book a delivery slot."));
        // Goal set, URL unset: model must not fabricate a path.
        assert!(p.contains("Do not invent or include a URL or path"));
    }

    #[test]
    fn generate_omits_no_url_clause_when_url_provided() {
        let mut b = minimal();
        b.goal = "book a delivery slot".to_string();
        b.goal_url = "/book".to_string();
        let p = generate(&b, "Voice: warm.");
        assert!(!p.contains("Do not invent or include a URL"));
    }

    #[test]
    fn generate_includes_goal_url_when_both_set() {
        let mut b = minimal();
        b.goal = "book an appointment".to_string();
        b.goal_url = "/book".to_string();
        let p = generate(&b, "Voice: warm.");
        assert!(p.contains("Goal: guide the customer to book an appointment at /book."));
    }

    #[test]
    fn generate_uses_default_goal_when_empty() {
        let p = generate(&minimal(), "Voice: warm.");
        assert!(p.contains(
            "Goal: answer the customer's question and let them know a human will follow up."
        ));
    }

    #[test]
    fn generate_includes_handoff_conditions() {
        let mut b = minimal();
        b.handoff_conditions = vec![
            "the customer is upset".to_string(),
            "  ".to_string(), // dropped
            "any refund or complaint".to_string(),
        ];
        let p = generate(&b, "Voice: warm.");
        assert!(p.contains(
            "Hand off to a human if any of these come up: the customer is upset; any refund or complaint."
        ));
    }

    #[test]
    fn generate_omits_handoff_block_when_empty() {
        let p = generate(&minimal(), "Voice: warm.");
        assert!(!p.contains("Hand off to a human"));
    }

    #[test]
    fn sanitize_goal_url_accepts_relative_and_https() {
        assert_eq!(sanitize_goal_url("/book"), "/book");
        assert_eq!(sanitize_goal_url("  /book  "), "/book");
        assert_eq!(
            sanitize_goal_url("https://example.com/book"),
            "https://example.com/book"
        );
    }

    #[test]
    fn sanitize_goal_url_rejects_dangerous_or_bare() {
        assert_eq!(sanitize_goal_url(""), "");
        assert_eq!(sanitize_goal_url("   "), "");
        assert_eq!(sanitize_goal_url("javascript:alert(1)"), "");
        assert_eq!(sanitize_goal_url("data:text/html,foo"), "");
        assert_eq!(sanitize_goal_url("http://insecure.example"), "");
        assert_eq!(sanitize_goal_url("example.com/book"), "");
        assert_eq!(sanitize_goal_url("https://"), ""); // empty after scheme
    }

    #[test]
    fn empty_builder_still_gets_a_business_line() {
        let p = generate(&PersonaBuilder::default(), "Voice: friendly.");
        assert!(p.starts_with("Business:"));
        assert!(p.contains("Voice: friendly."));
    }
}
