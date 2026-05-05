//! `/manage/demo`: operator controls for the public homepage demo.
//!
//! Three settings live on this page: an `enabled` toggle (gates
//! `/demo/personas`, `/demo/chat`, and the homepage entry point), a
//! regeneration cadence in minutes (how often the cron tick re-rolls
//! the stored persona blob), and the system prompt the generator uses.
//!
//! The prompt edit flow is preview-gated: a save only persists the new
//! prompt when the operator has just previewed it and the model
//! returned a parseable JSON array of the right shape. Toggle/cadence
//! edits don't need a preview.
//!
//! Re-rolling the stored personas is its own action (POST /reroll)
//! independent of saving config; the cron tick handles the recurring
//! refresh.

use worker::*;

use crate::management::audit;
use crate::storage::{self, DemoConfig};
use crate::templates::management as tmpl;

pub async fn handle_demo(
    mut req: Request,
    env: &Env,
    db: &D1Database,
    sub: &str,
    method: Method,
    actor_email: &str,
    base_url: &str,
) -> Result<Response> {
    let parts: Vec<&str> = sub
        .strip_prefix("demo")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let kv = env.kv("KV")?;
    let locale = crate::locale::Locale::from_request(&req);

    match (method, parts.as_slice()) {
        (Method::Get, []) => {
            let cfg = storage::get_demo_config(&kv).await.unwrap_or_default();
            let stored = storage::get_stored_demo_personas(&kv).await.unwrap_or(None);
            Response::from_html(tmpl::demo_config_html(
                &cfg,
                stored.as_ref(),
                base_url,
                &locale,
            ))
        }

        // Dedicated toggle endpoint so the checkbox at the top of the
        // page can fire on change without resaving cadence/prompt or
        // requiring a separate Save click. Reads the current config,
        // flips just the `enabled` field, persists, redirects.
        (Method::Post, ["toggle"]) => {
            let form: serde_json::Value = req.json().await?;
            let enabled = form
                .get("enabled")
                .and_then(|v| v.as_str())
                .map(|s| s == "true" || s == "on")
                .unwrap_or(false);

            let mut cfg = storage::get_demo_config(&kv).await.unwrap_or_default();
            cfg.enabled = enabled;
            storage::save_demo_config(&kv, &cfg).await?;
            // Toggling off clears the stored personas so a re-enable
            // starts from a clean slate.
            if !cfg.enabled {
                let _ = storage::delete_stored_demo_personas(&kv).await;
            }

            audit::log_action(
                db,
                actor_email,
                "edit_demo_config",
                "demo_config",
                None,
                Some(&serde_json::json!({ "enabled": cfg.enabled })),
            )
            .await?;

            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/manage/demo"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        (Method::Post, []) => {
            let form: serde_json::Value = req.json().await?;
            let parse_u32 = |key: &str, fallback: u32| -> u32 {
                form.get(key)
                    .and_then(|v| {
                        v.as_str()
                            .and_then(|s| s.parse::<u32>().ok())
                            .or_else(|| v.as_u64().map(|n| n as u32))
                    })
                    .unwrap_or(fallback)
            };
            let cadence = parse_u32(
                "regeneration_cadence_mins",
                storage::DEFAULT_DEMO_REGEN_CADENCE_MINS,
            );
            // Idle timer + turn limit. Clamp to sane bounds so a typo
            // can't lock visitors out (turns=0) or bake in a 24h idle.
            let idle_timeout_secs =
                parse_u32("idle_timeout_secs", storage::DEFAULT_DEMO_IDLE_TIMEOUT_SECS)
                    .clamp(5, 600);
            let max_user_turns =
                parse_u32("max_user_turns", storage::DEFAULT_DEMO_MAX_USER_TURNS).clamp(1, 20);
            let new_prompt = form
                .get("persona_generation_prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            let existing = storage::get_demo_config(&kv).await.unwrap_or_default();

            // Prompt-edit gate: only accept the new prompt if the
            // operator just verified it via /preview (the form
            // includes `prompt_verified=true`) OR if it's unchanged
            // from what's already stored. Empty input falls back to
            // the default (also pre-verified).
            let verified = form
                .get("prompt_verified")
                .and_then(|v| v.as_str())
                .map(|s| s == "true")
                .unwrap_or(false);
            let prompt = if new_prompt.is_empty() {
                storage::DEFAULT_DEMO_GENERATION_PROMPT.to_string()
            } else if new_prompt == existing.persona_generation_prompt || verified {
                new_prompt
            } else {
                return Response::from_html(
                    r#"<div class="error">Click "Preview generation" before saving — the new prompt must produce a valid JSON array first.</div>"#
                        .to_string(),
                );
            };

            let cfg = DemoConfig {
                enabled: existing.enabled,
                persona_generation_prompt: prompt,
                regeneration_cadence_mins: cadence,
                idle_timeout_secs,
                max_user_turns,
            };
            storage::save_demo_config(&kv, &cfg).await?;

            // If the operator just verified the prompt via Preview, the
            // success template embedded the rolled businesses as a
            // hidden field. Persist them as the live personas blob so
            // Save effectively saves both prompt + previewed output —
            // no second LLM roll, no drift between what was previewed
            // and what visitors see.
            if verified && cfg.enabled {
                if let Some(raw) = form
                    .get("rolled_personas_json")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                {
                    if let Ok(businesses) = serde_json::from_str::<
                        Vec<crate::handlers::demo_personas_list::DemoBusiness>,
                    >(raw)
                    {
                        let archetypes =
                            storage::list_archetypes(db, true).await.unwrap_or_default();
                        if businesses.len() == archetypes.len() && !archetypes.is_empty() {
                            let response = crate::handlers::demo_personas_list::build_response(
                                &archetypes,
                                businesses,
                            );
                            if let Ok(value) = serde_json::to_value(&response) {
                                let stored = storage::StoredDemoPersonas {
                                    generated_at: crate::helpers::now_iso(),
                                    response: value,
                                };
                                let _ = storage::save_stored_demo_personas(&kv, &stored).await;
                            }
                        }
                    }
                }
            }

            audit::log_action(
                db,
                actor_email,
                "edit_demo_config",
                "demo_config",
                None,
                Some(&serde_json::json!({
                    "regeneration_cadence_mins": cfg.regeneration_cadence_mins,
                })),
            )
            .await?;

            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/manage/demo"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Preview: run the operator's draft prompt against the current
        // approved archetypes and render the model's reply (raw + a
        // parse status) inline. Does NOT save the prompt.
        (Method::Post, ["preview"]) => {
            let form: serde_json::Value = req.json().await?;
            let prompt = form
                .get("persona_generation_prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if prompt.is_empty() {
                return Response::from_html(
                    r#"<div class="error">Add a prompt before previewing.</div>"#.to_string(),
                );
            }

            let archetypes = storage::list_archetypes(db, true).await.unwrap_or_default();
            if archetypes.is_empty() {
                return Response::from_html(
                    r#"<div class="muted">No Approved archetypes yet — add and approve at least one before previewing.</div>"#
                        .to_string(),
                );
            }

            match crate::handlers::demo_personas_list::generate_demo_businesses(
                env,
                &prompt,
                &archetypes,
            )
            .await
            {
                Ok(businesses) if businesses.len() == archetypes.len() => {
                    Response::from_html(tmpl::demo_preview_success_html(&archetypes, &businesses))
                }
                Ok(businesses) => Response::from_html(tmpl::demo_preview_shape_mismatch_html(
                    archetypes.len(),
                    businesses.len(),
                )),
                Err(msg) => Response::from_html(tmpl::demo_preview_error_html(&msg)),
            }
        }

        // Operator-driven re-roll. Generates a fresh personas blob
        // against the *currently saved* prompt (so a save+reroll
        // sequence reflects the new prompt) and writes it back.
        (Method::Post, ["reroll"]) => {
            let cfg = storage::get_demo_config(&kv).await.unwrap_or_default();
            if !cfg.enabled {
                return Response::from_html(
                    r#"<div class="error">Enable the demo before re-rolling personas.</div>"#
                        .to_string(),
                );
            }
            match crate::handlers::demo_personas_list::regenerate_and_store(
                env,
                &kv,
                db,
                &cfg.persona_generation_prompt,
            )
            .await
            {
                Ok(_) => {
                    audit::log_action(
                        db,
                        actor_email,
                        "reroll_demo_personas",
                        "demo_personas",
                        None,
                        None,
                    )
                    .await?;
                    let headers = Headers::new();
                    headers.set("HX-Redirect", &format!("{base_url}/manage/demo"))?;
                    Ok(Response::empty()?.with_status(200).with_headers(headers))
                }
                Err(msg) => Response::from_html(format!(
                    r#"<div class="error">Re-roll failed: {}</div>"#,
                    crate::helpers::html_escape(&msg)
                )),
            }
        }

        _ => Response::error("Not Found", 404),
    }
}
