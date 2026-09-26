//! `/api/persona` and `/api/archetypes` — the tenant's AI voice.
//!
//! Two modes (`PersonaSource`): a guided **builder** over a
//! platform-curated archetype, or a **custom** raw prompt for power users.
//!
//! Every save that changes the composed prompt resets the safety verdict to
//! Pending and re-enqueues the classifier. AI replies stay blocked
//! tenant-wide until the verdict comes back Approved *for that exact prompt
//! hash* — so a tenant can't edit a prompt past a stale approval. A save
//! that doesn't change the prompt (re-picking the same archetype) keeps the
//! existing verdict, so the badge doesn't flicker.

use serde::Serialize;
use worker::*;

use crate::personas;
use crate::prompt::MAX_CUSTOM_PROMPT;
use crate::storage::{get_archetype, get_onboarding, list_archetypes, save_onboarding};
use crate::types::*;

#[derive(Serialize)]
struct PersonaDetail {
    mode: &'static str,
    builder: PersonaBuilder,
    custom_prompt: String,
    safety: SafetyView,
    /// The composed middle as it stands — what the model actually receives
    /// between the fixed preamble and postamble.
    prompt: String,
    /// The fixed bookends, shown around the editable middle so there's no
    /// mystery about what gets sent.
    preamble: &'static str,
    postamble: &'static str,
    max_custom_prompt: usize,
}

#[derive(Serialize)]
struct SafetyView {
    status: &'static str,
    /// Deliberately vague on rejection: the classifier's actual category is
    /// logged, never returned, so a prompt can't be iterated against it.
    reason: Option<String>,
    checked_at: Option<String>,
    /// True when AI replies are permitted right now.
    ai_ready: bool,
}

#[derive(Serialize)]
struct ArchetypeView {
    slug: String,
    label: String,
    description: String,
    greeting: String,
    catch_phrases: Vec<String>,
    off_topics: Vec<String>,
    never: String,
    handoff_conditions: Vec<String>,
}

#[derive(Serialize)]
struct ArchetypeList {
    archetypes: Vec<ArchetypeView>,
}

#[derive(Serialize)]
struct PreviewView {
    prompt: String,
}

/// Stable wire string for a safety status.
pub fn safety_wire(status: &PersonaSafetyStatus) -> &'static str {
    match status {
        PersonaSafetyStatus::Pending => "pending",
        PersonaSafetyStatus::Approved => "approved",
        PersonaSafetyStatus::Rejected => "rejected",
    }
}

/// `GET /api/archetypes` — the approved persona catalog for the picker.
pub async fn list_archetypes_handler(req: Request, env: Env) -> Result<Response> {
    if let Err(r) = super::require_tenant(&req, &env).await {
        return r;
    }
    let db = env.d1("DB")?;
    let rows = list_archetypes(&db, true).await?;
    super::json(&ArchetypeList {
        archetypes: rows
            .into_iter()
            .map(|a| ArchetypeView {
                slug: a.slug,
                label: a.label,
                description: a.description,
                greeting: a.greeting,
                catch_phrases: a.catch_phrases,
                off_topics: a.off_topics,
                never: a.never,
                handoff_conditions: a.handoff_conditions,
            })
            .collect(),
    })
}

pub async fn handle(mut req: Request, env: Env, rest: &[&str]) -> Result<Response> {
    let tenant_id = match super::require_tenant(&req, &env).await {
        Ok(t) => t,
        Err(r) => return r,
    };
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;
    let method = req.method();
    let mut state = get_onboarding(&kv, &tenant_id).await?;

    match (method, rest) {
        (Method::Get, []) => detail(&kv, &db, &state.persona).await,

        (Method::Put, []) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let mode = super::field(&body, "mode");

            let new_source = match mode.as_str() {
                "builder" => {
                    let Some(b) = body.get("builder") else {
                        return super::invalid("Missing builder fields.");
                    };
                    PersonaSource::Builder(read_builder(b))
                }
                "custom" => {
                    let raw = super::field(&body, "custom_prompt");
                    if raw.is_empty() {
                        return super::invalid("Write a prompt, or switch to the guided builder.");
                    }
                    PersonaSource::Custom(raw.chars().take(MAX_CUSTOM_PROMPT).collect())
                }
                _ => return super::invalid("Unknown persona mode."),
            };

            let voice_prompt = voice_for(&db, &new_source).await?;

            // Compare the composed prompt against what was last vetted. Only
            // a genuine change costs a new classifier run.
            let mut new_persona = PersonaConfig {
                source: new_source,
                safety: state.persona.safety.clone(),
            };
            let new_hash = new_persona.active_prompt_hash(&voice_prompt);
            let prompt_changed =
                state.persona.safety.checked_prompt_hash.as_deref() != Some(new_hash.as_str());

            if prompt_changed {
                new_persona.safety = PersonaSafety {
                    status: PersonaSafetyStatus::Pending,
                    checked_prompt_hash: None,
                    checked_at: None,
                    vague_reason: None,
                };
            }

            state.persona = new_persona;
            save_onboarding(&kv, &tenant_id, &state).await?;

            if prompt_changed {
                let job = crate::safety_queue::SafetyJob {
                    target: crate::safety_queue::SafetyJobTarget::Tenant {
                        tenant_id: tenant_id.clone(),
                    },
                    prompt_hash: new_hash,
                };
                let _ = crate::safety_queue::enqueue(&env, job).await;
            }

            detail(&kv, &db, &state.persona).await
        }

        // Compose a prompt from unsaved builder fields. Pure: writes
        // nothing, spends nothing, and never touches the safety queue.
        (Method::Post, ["preview"]) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let builder = read_builder(body.get("builder").unwrap_or(&body));
            let voice_prompt = match get_archetype(&db, &builder.archetype_slug).await? {
                Some(a) => a.voice_prompt,
                None => String::new(),
            };
            super::json(&PreviewView {
                prompt: personas::generate(&builder, &voice_prompt),
            })
        }

        _ => super::not_found("Endpoint"),
    }
}

/// Read a `PersonaBuilder` from JSON, applying every field cap.
///
/// Chip lists arrive as JSON arrays now rather than the newline-delimited
/// textarea strings the HTML form posted.
fn read_builder(b: &serde_json::Value) -> PersonaBuilder {
    let chips = |key: &str, max: usize, char_cap: usize| -> Vec<String> {
        b.get(key)
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim().chars().take(char_cap).collect::<String>())
                    .filter(|s| !s.is_empty())
                    .take(max)
                    .collect()
            })
            .unwrap_or_default()
    };

    PersonaBuilder {
        archetype_slug: super::field(b, "archetype_slug"),
        biz_name: super::field(b, "biz_name"),
        biz_type: super::field(b, "biz_type"),
        city: super::field(b, "city"),
        hours: super::field(b, "hours"),
        goal: super::field(b, "goal").chars().take(120).collect(),
        goal_url: personas::sanitize_goal_url(&super::field(b, "goal_url"))
            .chars()
            .take(200)
            .collect(),
        catch_phrases: chips("catch_phrases", 5, 200),
        off_topics: chips("off_topics", 10, 200),
        never: super::field(b, "never"),
        handoff_conditions: chips("handoff_conditions", 5, 120),
    }
}

/// The archetype voice prompt a persona composes against. Empty for a
/// custom prompt, which is used verbatim.
async fn voice_for(db: &D1Database, source: &PersonaSource) -> Result<String> {
    Ok(match source {
        PersonaSource::Builder(b) => get_archetype(db, &b.archetype_slug)
            .await?
            .map(|a| a.voice_prompt)
            .unwrap_or_default(),
        PersonaSource::Custom(_) => String::new(),
    })
}

async fn detail(kv: &kv::KvStore, db: &D1Database, persona: &PersonaConfig) -> Result<Response> {
    let voice_prompt = match &persona.source {
        PersonaSource::Builder(b) => {
            match crate::storage::get_archetype_cached(kv, db, &b.archetype_slug).await {
                Ok(Some(a)) => a.voice_prompt,
                _ => String::new(),
            }
        }
        PersonaSource::Custom(_) => String::new(),
    };

    let (mode, builder, custom_prompt) = match &persona.source {
        PersonaSource::Builder(b) => ("builder", b.clone(), String::new()),
        PersonaSource::Custom(s) => ("custom", PersonaBuilder::default(), s.clone()),
    };

    super::json(&PersonaDetail {
        mode,
        builder,
        custom_prompt,
        safety: SafetyView {
            status: safety_wire(&persona.safety.status),
            reason: persona.safety.vague_reason.clone(),
            checked_at: persona.safety.checked_at.clone(),
            ai_ready: persona.is_safe_to_use(&voice_prompt),
        },
        prompt: persona.active_prompt(&voice_prompt),
        preamble: crate::prompt::PREAMBLE,
        postamble: crate::prompt::POSTAMBLE,
        max_custom_prompt: MAX_CUSTOM_PROMPT,
    })
}
