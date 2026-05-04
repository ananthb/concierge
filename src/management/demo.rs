//! `/manage/demo`: operator controls for the public homepage demo.
//! Two settings: an `enabled` toggle (gates `/demo/personas`,
//! `/demo/chat`, and the homepage entry point) and the system prompt
//! used to generate fictional sample businesses for each archetype.
//!
//! Saving busts `cache:demo:personas:v1` so the change reaches visitors
//! immediately. A separate preview endpoint runs the prompt against
//! the current archetypes and returns the raw + parsed result so an
//! operator can iterate on the prompt without breaking the live demo.

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
            Response::from_html(tmpl::demo_config_html(&cfg, base_url, &locale))
        }

        (Method::Post, []) => {
            let form: serde_json::Value = req.json().await?;
            let enabled = form
                .get("enabled")
                .and_then(|v| v.as_str())
                .map(|s| s == "true" || s == "on")
                .unwrap_or(false);
            let prompt_raw = form
                .get("persona_generation_prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let prompt = if prompt_raw.is_empty() {
                storage::DEFAULT_DEMO_GENERATION_PROMPT.to_string()
            } else {
                prompt_raw
            };

            let cfg = DemoConfig {
                enabled,
                persona_generation_prompt: prompt,
            };
            storage::save_demo_config(&kv, &cfg).await?;
            // Persona list is regenerated against the new prompt next
            // request. Toggling `enabled` also needs the cache cleared
            // so a previously-cached non-empty list doesn't survive an
            // off→off render.
            let _ = storage::invalidate_demo_personas_cache(&kv).await;

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
                Ok(businesses) => {
                    Response::from_html(tmpl::demo_preview_success_html(&archetypes, &businesses))
                }
                Err(msg) => Response::from_html(tmpl::demo_preview_error_html(&msg)),
            }
        }

        _ => Response::error("Not Found", 404),
    }
}
