//! Archetype catalog management: list, view, create, edit, delete.
//! Routes are dispatched from `management::mod` under `/manage/archetypes`.
//!
//! Every save resets the row's `safety_status` to Draft and enqueues a
//! `SafetyJob` keyed on the slug. Until the queue consumer flips the
//! row back to Approved, neither the demo nor tenant onboarding will
//! see it.

use worker::*;

use crate::management::audit;
use crate::storage;
use crate::templates::management as tmpl;
use crate::types::{Archetype, PersonaSafety, PersonaSafetyStatus};

pub async fn handle_archetypes(
    mut req: Request,
    env: &Env,
    db: &D1Database,
    sub: &str,
    method: Method,
    actor_email: &str,
    base_url: &str,
) -> Result<Response> {
    let parts: Vec<&str> = sub
        .strip_prefix("archetypes")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let locale = crate::locale::Locale::from_request(&req);
    let kv = env.kv("KV")?;

    match (method, parts.as_slice()) {
        // List all archetypes (drafts + approved + rejected; operators see everything)
        (Method::Get, []) => {
            let rows = storage::list_archetypes(db, false).await?;
            Response::from_html(tmpl::archetypes_list_html(&rows, base_url, &locale))
        }

        // New archetype form
        (Method::Get, ["new"]) => {
            Response::from_html(tmpl::archetype_edit_html(None, base_url, &locale))
        }

        // Create
        (Method::Post, ["new"]) => {
            let form: serde_json::Value = req.json().await?;
            let row = match build_row_from_form(&form) {
                Ok(r) => r,
                Err(msg) => {
                    return Response::from_html(format!(
                        r#"<div class="error">{}</div>"#,
                        crate::helpers::html_escape(&msg)
                    ));
                }
            };

            // Slug uniqueness.
            if storage::get_archetype(db, &row.slug).await?.is_some() {
                return Response::from_html(
                    r#"<div class="error">An archetype with that slug already exists.</div>"#
                        .to_string(),
                );
            }

            storage::upsert_archetype(db, &row).await?;
            let _ = storage::invalidate_archetype_cache(&kv, &row.slug).await;
            let _ = storage::invalidate_demo_personas_cache(&kv).await;
            enqueue_catalog_safety(env, &row).await;

            audit::log_action(
                db,
                actor_email,
                "create_archetype",
                "archetype",
                Some(&row.slug),
                Some(&serde_json::json!({ "label": row.label })),
            )
            .await?;

            let headers = Headers::new();
            headers.set(
                "HX-Redirect",
                &format!("{base_url}/manage/archetypes/{}", row.slug),
            )?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Edit form
        (Method::Get, [slug]) => {
            let row = match storage::get_archetype(db, slug).await? {
                Some(r) => r,
                None => return Response::error("Archetype not found", 404),
            };
            Response::from_html(tmpl::archetype_edit_html(Some(&row), base_url, &locale))
        }

        // Update
        (Method::Post, [slug]) => {
            let _existing = match storage::get_archetype(db, slug).await? {
                Some(r) => r,
                None => return Response::error("Archetype not found", 404),
            };
            let form: serde_json::Value = req.json().await?;
            let mut row = match build_row_from_form(&form) {
                Ok(r) => r,
                Err(msg) => {
                    return Response::from_html(format!(
                        r#"<div class="error">{}</div>"#,
                        crate::helpers::html_escape(&msg)
                    ));
                }
            };
            // Force-keep the slug from the row on disk.
            row.slug = slug.to_string();

            storage::upsert_archetype(db, &row).await?;
            let _ = storage::invalidate_archetype_cache(&kv, &row.slug).await;
            let _ = storage::invalidate_demo_personas_cache(&kv).await;
            enqueue_catalog_safety(env, &row).await;

            audit::log_action(
                db,
                actor_email,
                "edit_archetype",
                "archetype",
                Some(&row.slug),
                Some(&serde_json::json!({ "label": row.label })),
            )
            .await?;

            let headers = Headers::new();
            headers.set(
                "HX-Redirect",
                &format!("{base_url}/manage/archetypes/{}", row.slug),
            )?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Delete
        (Method::Post, [slug, "delete"]) => match storage::delete_archetype(db, slug).await {
            Ok(()) => {
                let _ = storage::invalidate_archetype_cache(&kv, slug).await;
                let _ = storage::invalidate_demo_personas_cache(&kv).await;
                audit::log_action(
                    db,
                    actor_email,
                    "delete_archetype",
                    "archetype",
                    Some(slug),
                    None,
                )
                .await?;
                let headers = Headers::new();
                headers.set("HX-Redirect", &format!("{base_url}/manage/archetypes"))?;
                Ok(Response::empty()?.with_status(200).with_headers(headers))
            }
            Err(e) => Response::from_html(format!(
                r#"<div class="error">{}</div>"#,
                crate::helpers::html_escape(&e.to_string())
            )),
        },

        _ => Response::error("Not Found", 404),
    }
}

/// Construct an `Archetype` from the JSON-encoded form.
fn build_row_from_form(form: &serde_json::Value) -> std::result::Result<Archetype, String> {
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

    let slug = s("slug");
    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        || slug.is_empty()
    {
        return Err("Slug must be lowercase letters, digits, '_' or '-' (no spaces).".into());
    }
    let label = s("label");
    if label.is_empty() {
        return Err("Label is required.".into());
    }
    let description = s("description");
    if description.is_empty() {
        return Err("Description is required.".into());
    }
    let voice_prompt = s("voice_prompt");
    if voice_prompt.is_empty() {
        return Err("Voice prompt is required.".into());
    }
    let greeting = s("greeting");
    if greeting.is_empty() {
        return Err("Greeting is required.".into());
    }
    let default_rules_json = s("default_rules_json");
    if default_rules_json.is_empty() {
        return Err("Default rules JSON is required.".into());
    }
    if let Err(e) = serde_json::from_str::<serde_json::Value>(&default_rules_json) {
        return Err(format!("Invalid JSON in default rules: {}", e));
    }

    Ok(Archetype {
        slug,
        label,
        description,
        voice_prompt,
        greeting,
        default_rules_json,
        catch_phrases: chips("catch_phrases"),
        off_topics: chips("off_topics"),
        never: s("never"),
        handoff_conditions: chips("handoff_conditions"),
        safety: PersonaSafety {
            status: PersonaSafetyStatus::Pending,
            checked_prompt_hash: None,
            checked_at: None,
            vague_reason: None,
        },
        created_at: None,
        updated_at: None,
    })
}

async fn enqueue_catalog_safety(env: &Env, row: &Archetype) {
    let job = crate::safety_queue::SafetyJob {
        target: crate::safety_queue::SafetyJobTarget::Catalog {
            slug: row.slug.clone(),
        },
        prompt_hash: crate::helpers::sha256_hex(&row.voice_prompt),
    };
    let _ = crate::safety_queue::enqueue(env, job).await;
}
