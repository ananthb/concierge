//! Persona catalog management: list, view, create, edit, delete.
//! Routes are dispatched from `management::mod` under `/manage/personas`.
//!
//! Every save resets the row's `safety_status` to Draft and enqueues a
//! `SafetyJob` keyed on the slug. Until the queue consumer flips the
//! row back to Approved, neither the demo nor tenant onboarding will
//! see it.

use worker::*;

use crate::management::audit;
use crate::storage;
use crate::templates::management as tmpl;
use crate::types::{
    PersonaBuilder, PersonaCatalogRow, PersonaPreset, PersonaSafety, PersonaSafetyStatus,
    PersonaSource,
};

pub async fn handle_personas(
    mut req: Request,
    env: &Env,
    db: &D1Database,
    sub: &str,
    method: Method,
    actor_email: &str,
    base_url: &str,
) -> Result<Response> {
    let parts: Vec<&str> = sub
        .strip_prefix("personas")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let locale = crate::locale::Locale::from_request(&req);

    match (method, parts.as_slice()) {
        // List all personas (drafts + approved + rejected — operators see everything)
        (Method::Get, []) => {
            let rows = storage::list_personas(db, false).await?;
            Response::from_html(tmpl::personas_list_html(&rows, base_url, &locale))
        }

        // New persona form
        (Method::Get, ["new"]) => {
            Response::from_html(tmpl::persona_edit_html(None, base_url, &locale))
        }

        // Create
        (Method::Post, ["new"]) => {
            let form: serde_json::Value = req.json().await?;
            let row = match build_row_from_form(&form, false) {
                Ok(r) => r,
                Err(msg) => {
                    return Response::from_html(format!(
                        r#"<div class="error">{}</div>"#,
                        crate::helpers::html_escape(&msg)
                    ));
                }
            };

            // Slug uniqueness — if it already exists, refuse so the operator
            // doesn't accidentally overwrite an unrelated row.
            if storage::get_persona(db, &row.slug).await?.is_some() {
                return Response::from_html(
                    r#"<div class="error">A persona with that slug already exists. Pick a different one.</div>"#
                        .to_string(),
                );
            }

            storage::upsert_persona(db, &row).await?;
            enqueue_catalog_safety(env, &row).await;

            audit::log_action(
                db,
                actor_email,
                "create_persona",
                "persona",
                Some(&row.slug),
                Some(&serde_json::json!({ "label": row.label })),
            )
            .await?;

            let headers = Headers::new();
            headers.set(
                "HX-Redirect",
                &format!("{base_url}/manage/personas/{}", row.slug),
            )?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Edit form
        (Method::Get, [slug]) => {
            let row = match storage::get_persona(db, slug).await? {
                Some(r) => r,
                None => return Response::error("Persona not found", 404),
            };
            Response::from_html(tmpl::persona_edit_html(Some(&row), base_url, &locale))
        }

        // Save edits
        (Method::Post, [slug]) => {
            let existing = match storage::get_persona(db, slug).await? {
                Some(r) => r,
                None => return Response::error("Persona not found", 404),
            };
            let form: serde_json::Value = req.json().await?;
            let mut row = match build_row_from_form(&form, existing.is_system) {
                Ok(r) => r,
                Err(msg) => {
                    return Response::from_html(format!(
                        r#"<div class="error">{}</div>"#,
                        crate::helpers::html_escape(&msg)
                    ));
                }
            };
            // Force-keep the slug + system flag from the row on disk.
            row.slug = existing.slug.clone();
            row.is_system = existing.is_system;

            storage::upsert_persona(db, &row).await?;
            enqueue_catalog_safety(env, &row).await;

            audit::log_action(
                db,
                actor_email,
                "edit_persona",
                "persona",
                Some(&row.slug),
                Some(&serde_json::json!({ "label": row.label })),
            )
            .await?;

            let headers = Headers::new();
            headers.set(
                "HX-Redirect",
                &format!("{base_url}/manage/personas/{}", row.slug),
            )?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Delete (refuses on system rows)
        (Method::Post, [slug, "delete"]) => match storage::delete_persona(db, slug).await {
            Ok(()) => {
                audit::log_action(
                    db,
                    actor_email,
                    "delete_persona",
                    "persona",
                    Some(slug),
                    None,
                )
                .await?;
                let headers = Headers::new();
                headers.set("HX-Redirect", &format!("{base_url}/manage/personas"))?;
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

/// Construct a `PersonaCatalogRow` from the JSON-encoded form. Validates
/// required fields (`slug`, `label`, `description`, `greeting`) and the
/// archetype slug. Returns `Err(message)` with a tenant-facing string
/// when the input doesn't pass.
fn build_row_from_form(
    form: &serde_json::Value,
    keep_system: bool,
) -> std::result::Result<PersonaCatalogRow, String> {
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
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
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
    let greeting = s("greeting");
    if greeting.is_empty() {
        return Err("Greeting is required.".into());
    }

    let mode = s("mode");
    let source = if mode == "custom" {
        let middle = s("custom_text");
        if middle.is_empty() {
            return Err("Custom prompt text is required.".into());
        }
        PersonaSource::Custom(
            middle
                .chars()
                .take(crate::prompt::MAX_CUSTOM_PROMPT)
                .collect(),
        )
    } else {
        let archetype = PersonaPreset::from_slug(&s("archetype")).unwrap_or_default();
        let biz_name = s("biz_name");
        let biz_type = s("biz_type");
        if biz_name.is_empty() {
            return Err("Business name is required for the builder mode.".into());
        }
        if biz_type.is_empty() {
            return Err("Business type is required for the builder mode.".into());
        }
        PersonaSource::Builder(PersonaBuilder {
            archetype,
            biz_name,
            biz_type,
            city: s("city"),
            catch_phrases: chips("catch_phrases").into_iter().take(5).collect(),
            off_topics: chips("off_topics").into_iter().take(10).collect(),
            never: s("never"),
        })
    };

    Ok(PersonaCatalogRow {
        slug,
        label,
        description,
        source,
        greeting,
        is_system: keep_system,
        // Always reset to Pending (= Draft on disk) on save; the
        // classifier flips it back to Approved/Rejected.
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

async fn enqueue_catalog_safety(env: &Env, row: &PersonaCatalogRow) {
    let job = crate::safety_queue::SafetyJob {
        target: crate::safety_queue::SafetyJobTarget::Catalog {
            slug: row.slug.clone(),
        },
        prompt_hash: crate::helpers::sha256_hex(&row.source.active_prompt()),
    };
    let _ = crate::safety_queue::enqueue(env, job).await;
}
