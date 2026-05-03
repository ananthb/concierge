//! `/admin/persona` — read + edit the tenant's AI persona.
//!
//! The persona has three modes (PersonaSource): Preset / Builder / Custom.
//! Each save recomputes `active_prompt_hash`; if it differs from the
//! last-vetted hash, the safety check is re-enqueued.

use worker::*;

use crate::personas;
use crate::storage::{get_onboarding, save_onboarding};
use crate::templates::persona::persona_admin_html;
use crate::types::{
    PersonaBuilder, PersonaConfig, PersonaPreset, PersonaSafety, PersonaSafetyStatus, PersonaSource,
};

/// Maximum length of the user-provided custom prompt — re-export of the
/// canonical limit in `crate::prompt` so this handler caps at the same
/// number every other prompt-bearing handler does.
use crate::prompt::MAX_CUSTOM_PROMPT;

pub async fn handle_persona_admin(
    mut req: Request,
    env: Env,
    path: &str,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("KV")?;
    let method = req.method();
    let locale = crate::locale::Locale::from_request(&req);
    let mut state = get_onboarding(&kv, tenant_id).await?;

    match (method, path) {
        (Method::Get, "/admin/persona") => {
            Response::from_html(persona_admin_html(&state.persona, base_url, &locale))
        }

        (Method::Post, "/admin/persona") => {
            let form: serde_json::Value = req.json().await?;

            let mode = form
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("builder");

            let new_source = match mode {
                "builder" => {
                    let s = |k: &str| {
                        form.get(k)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .trim()
                            .to_string()
                    };
                    let parse_chips = |k: &str| -> Vec<String> {
                        form.get(k)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .split('\n')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .take(10)
                            .collect()
                    };
                    let archetype = PersonaPreset::from_slug(&s("archetype")).unwrap_or_default();
                    PersonaSource::Builder(PersonaBuilder {
                        archetype,
                        biz_name: s("biz_name"),
                        biz_type: s("biz_type"),
                        city: s("city"),
                        hours: s("hours"),
                        goal: s("goal").chars().take(120).collect(),
                        goal_url: crate::personas::sanitize_goal_url(&s("goal_url"))
                            .chars()
                            .take(200)
                            .collect(),
                        catch_phrases: parse_chips("catch_phrases").into_iter().take(5).collect(),
                        off_topics: parse_chips("off_topics"),
                        never: s("never"),
                        handoff_conditions: parse_chips("handoff_conditions")
                            .into_iter()
                            .map(|c| c.chars().take(120).collect::<String>())
                            .take(5)
                            .collect(),
                    })
                }
                "custom" => {
                    let raw = form
                        .get("custom_prompt")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let trimmed = raw.trim();
                    if trimmed.is_empty() {
                        return Response::from_html(
                            r#"<div class="error">Write a prompt or pick another mode.</div>"#
                                .to_string(),
                        );
                    }
                    let bounded: String = trimmed.chars().take(MAX_CUSTOM_PROMPT).collect();
                    PersonaSource::Custom(bounded)
                }
                _ => {
                    return Response::from_html(
                        r#"<div class="error">Unknown persona mode.</div>"#.to_string(),
                    );
                }
            };

            // Build a candidate persona and check if its prompt actually
            // differs from what we already vetted. If yes: status -> Pending
            // and enqueue. If no (e.g. user re-selected the same preset):
            // keep the existing safety verdict so the badge doesn't flicker.
            let mut new_persona = PersonaConfig {
                source: new_source,
                safety: state.persona.safety.clone(),
            };
            let new_hash = new_persona.active_prompt_hash();
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
            save_onboarding(&kv, tenant_id, &state).await?;

            if prompt_changed {
                let job = crate::safety_queue::SafetyJob {
                    target: crate::safety_queue::SafetyJobTarget::Tenant {
                        tenant_id: tenant_id.to_string(),
                    },
                    prompt_hash: new_hash,
                };
                let _ = crate::safety_queue::enqueue(&env, job).await;
            }

            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/admin/persona"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Live preview endpoint: takes Builder field values, returns the
        // generated prompt. Lets the UI show a current preview without
        // committing to a save.
        (Method::Post, "/admin/persona/preview") => {
            let form: serde_json::Value = req.json().await?;
            let s = |k: &str| {
                form.get(k)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string()
            };
            let chips = |k: &str| -> Vec<String> {
                form.get(k)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .split('\n')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            };
            let archetype = PersonaPreset::from_slug(&s("archetype")).unwrap_or_default();
            let builder = PersonaBuilder {
                archetype,
                biz_name: s("biz_name"),
                biz_type: s("biz_type"),
                city: s("city"),
                hours: s("hours"),
                goal: s("goal").chars().take(120).collect(),
                goal_url: crate::personas::sanitize_goal_url(&s("goal_url"))
                    .chars()
                    .take(200)
                    .collect(),
                catch_phrases: chips("catch_phrases").into_iter().take(5).collect(),
                off_topics: chips("off_topics").into_iter().take(10).collect(),
                never: s("never"),
                handoff_conditions: chips("handoff_conditions")
                    .into_iter()
                    .map(|c| c.chars().take(120).collect::<String>())
                    .take(5)
                    .collect(),
            };
            let prompt = personas::generate(&builder);
            // Returned to `#prompt-preview` with `hx-swap="outerHTML"` — the
            // bookends are static neighbours rendered server-side once.
            Response::from_html(format!(
                r#"<pre id="prompt-preview" class="prompt-preview prompt-preview-middle">{}</pre>"#,
                crate::helpers::html_escape(&prompt)
            ))
        }

        _ => Response::error("Not Found", 404),
    }
}
