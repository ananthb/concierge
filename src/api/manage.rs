//! `/api/manage/*` — operator endpoints, gated by Cloudflare Access.
//!
//! Not a tenant surface: authentication here is the Access JWT
//! ([`crate::management::verify_access`]), not the session cookie, and the
//! `email` claim from that JWT is the actor recorded in the audit log.
//!
//! Scope is deliberately narrow. This is what the platform can't run
//! without: the archetype catalog (the persona picker and the landing-page
//! demo both read it), the demo config, pricing, tenant administration, the
//! audit trail, and the schema reseed. The richer HTML panel that used to
//! live at `/manage` is not reproduced endpoint-for-endpoint.
//!
//! `Page.Manage` in the frontend is the console over these. It does not
//! cover the archetype endpoints, which is why they read as write-only from
//! the UI side.

use serde::Serialize;
use worker::*;

use crate::management::audit;

/// Audit entries returned alongside a single tenant. Enough to see recent
/// operator actions on that account without paging.
const AUDIT_PER_RESOURCE: u32 = 25;
use crate::storage::{self, DemoConfig};
use crate::types::{Archetype, PersonaSafety, PersonaSafetyStatus};

#[derive(Serialize)]
struct Overview {
    actor: String,
    tenant_count: usize,
    /// Same component report the health endpoint returns, so the operator
    /// panel and `/health` can never disagree about what's configured.
    health: serde_json::Value,
}

#[derive(Serialize)]
struct TenantList {
    tenants: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct ArchetypeList {
    archetypes: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct AuditList {
    entries: Vec<serde_json::Value>,
    /// True when the page came back full, so there may be older entries.
    has_more: bool,
}

#[derive(Serialize)]
struct PricingView {
    min_credits: i64,
    max_credits: i64,
    /// Absolute bound on `max_credits`, so the operator UI can validate
    /// before submitting rather than learning about it from a 400.
    max_credits_ceiling: i64,
    /// `[{concept, currency, amount}]`, flattened from the keyed map so the
    /// frontend gets a plain list to render as a table.
    amounts: Vec<PricingAmount>,
    /// Every concept that can be priced, with the labels and unit captions
    /// the editor renders. Sent rather than hardcoded in the frontend so a
    /// new concept shows up in the UI without a frontend change.
    concepts: Vec<ConceptView>,
    /// Currency codes that already have at least one amount configured.
    currencies: Vec<String>,
}

#[derive(Serialize)]
struct PricingAmount {
    concept: String,
    currency: String,
    amount: i64,
}

#[derive(Serialize)]
struct ConceptView {
    wire: &'static str,
    label: &'static str,
    unit_caption: &'static str,
    /// True when the amount is 1/1000 of the currency's minor unit, which
    /// the editor needs in order to show a sane decimal.
    is_milli: bool,
}

pub async fn handle(mut req: Request, env: Env, rest: &[&str]) -> Result<Response> {
    let Some(actor) = crate::management::verify_access(&req, &env).await else {
        return super::error(
            "access_required",
            "This endpoint requires Cloudflare Access.",
            403,
        );
    };

    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;
    let method = req.method();

    match (method, rest) {
        // --- overview -------------------------------------------------
        (Method::Get, []) | (Method::Get, ["overview"]) => {
            let tenant_count = storage::count_tenants(&db).await.unwrap_or(0);
            let report = crate::handlers::health::run_checks(&env, true).await;
            super::json(&Overview {
                actor,
                tenant_count,
                health: serde_json::to_value(&report).unwrap_or(serde_json::Value::Null),
            })
        }

        // --- tenants --------------------------------------------------
        (Method::Get, ["tenants"]) => {
            let url = req.url()?;
            let q = url
                .query_pairs()
                .find(|(k, _)| k == "q")
                .map(|(_, v)| v.into_owned())
                .unwrap_or_default();
            let tenants = if q.trim().is_empty() {
                storage::list_tenants(&db).await?
            } else {
                storage::search_tenants(&db, q.trim()).await?
            };
            super::json(&TenantList {
                tenants: tenants
                    .iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect(),
            })
        }

        (Method::Get, ["tenants", id]) => {
            let Some(tenant) = storage::get_tenant(&db, id).await? else {
                return super::not_found("Tenant");
            };
            let mut bill = storage::get_tenant_billing(&db, id).await?;
            crate::billing::refresh_billing(&mut bill);
            super::json(&serde_json::json!({
                "tenant": tenant,
                "billing": bill,
                "onboarding": storage::get_onboarding(&kv, id).await?,
                "whatsapp": storage::list_whatsapp_accounts(&kv, id).await?,
                "audit": audit::get_audit_for_resource(&db, "tenant", id, AUDIT_PER_RESOURCE).await?,
            }))
        }

        (Method::Post, ["tenants", id, "grant-replies"]) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let Some(count) = super::field_i64(&body, "count").filter(|n| *n > 0) else {
                return super::invalid("Grant a positive number of replies.");
            };
            // `expires_in_days` absent means a permanent grant. Zero or
            // negative is treated as absent rather than as an instantly-expired
            // grant, which would silently do nothing.
            match super::field_i64(&body, "expires_in_days").filter(|d| *d > 0) {
                Some(days) => crate::billing::grant_with_expiry(&db, id, count, days).await?,
                None => crate::billing::grant_purchased(&db, id, count).await?,
            }
            audit::log_action(
                &db,
                &actor,
                "grant_replies",
                "tenant",
                Some(id),
                Some(&serde_json::json!({ "count": count })),
            )
            .await?;
            super::no_content()
        }

        (Method::Delete, ["tenants", id]) => {
            // Logged before the delete: afterwards there's no tenant row to
            // tie the entry to, and a delete that vanished without a trace
            // is worse than one logged slightly early.
            audit::log_action(&db, &actor, "delete_tenant", "tenant", Some(id), None).await?;
            storage::delete_tenant_data(&kv, &db, id).await?;
            super::no_content()
        }

        // --- archetype catalog ----------------------------------------
        (Method::Get, ["archetypes"]) => {
            // `false`: operators see pending and rejected rows too, which is
            // the whole point of the panel. Tenants only ever see approved.
            let rows = storage::list_archetypes(&db, false).await?;
            super::json(&ArchetypeList {
                archetypes: rows
                    .iter()
                    .filter_map(|a| serde_json::to_value(a).ok())
                    .collect(),
            })
        }

        (Method::Put, ["archetypes", slug]) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let row = match read_archetype(slug, &body) {
                Ok(r) => r,
                Err(msg) => return super::invalid(&msg),
            };
            storage::upsert_archetype(&db, &row).await?;
            // The rolled demo-persona blob was generated from the old
            // catalog, so it's stale the moment a row changes.
            let _ = storage::delete_stored_demo_personas(&kv).await;
            let _ = storage::invalidate_archetype_cache(&kv, slug).await;
            enqueue_catalog_safety(&env, &row).await;
            audit::log_action(
                &db,
                &actor,
                "upsert_archetype",
                "archetype",
                Some(slug),
                None,
            )
            .await?;
            super::json(&row)
        }

        (Method::Delete, ["archetypes", slug]) => {
            storage::delete_archetype(&db, slug).await?;
            let _ = storage::delete_stored_demo_personas(&kv).await;
            let _ = storage::invalidate_archetype_cache(&kv, slug).await;
            audit::log_action(
                &db,
                &actor,
                "delete_archetype",
                "archetype",
                Some(slug),
                None,
            )
            .await?;
            super::no_content()
        }

        // --- landing-page demo ----------------------------------------
        (Method::Get, ["demo"]) => {
            let cfg = storage::get_demo_config(&kv).await.unwrap_or_default();
            let stored = storage::get_stored_demo_personas(&kv).await.ok().flatten();
            super::json(&serde_json::json!({
                "config": cfg,
                "generated_at": stored.as_ref().map(|s| s.generated_at.clone()),
                "default_prompt": storage::DEFAULT_DEMO_GENERATION_PROMPT,
            }))
        }

        (Method::Put, ["demo"]) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let existing = storage::get_demo_config(&kv).await.unwrap_or_default();
            let u32_field = |key: &str, fallback: u32| -> u32 {
                super::field_i64(&body, key)
                    .map(|n| n.max(0) as u32)
                    .unwrap_or(fallback)
            };

            let prompt = {
                let new_prompt = super::field(&body, "persona_generation_prompt");
                if new_prompt.is_empty() {
                    storage::DEFAULT_DEMO_GENERATION_PROMPT.to_string()
                } else {
                    new_prompt
                }
            };

            let cfg = DemoConfig {
                enabled: body
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(existing.enabled),
                persona_generation_prompt: prompt,
                regeneration_cadence_mins: u32_field(
                    "regeneration_cadence_mins",
                    existing.regeneration_cadence_mins,
                ),
                // Clamped so a typo can't lock visitors out (turns=0) or
                // bake in a 24-hour idle window.
                idle_timeout_secs: u32_field("idle_timeout_secs", existing.idle_timeout_secs)
                    .clamp(5, 600),
                max_user_turns: u32_field("max_user_turns", existing.max_user_turns).clamp(1, 20),
            };
            storage::save_demo_config(&kv, &cfg).await?;

            // A changed generation prompt invalidates the rolled blob; the
            // next visitor (or the cron tick) regenerates against the new one.
            if cfg.persona_generation_prompt != existing.persona_generation_prompt || !cfg.enabled {
                let _ = storage::delete_stored_demo_personas(&kv).await;
            }
            audit::log_action(&db, &actor, "update_demo_config", "demo", None, None).await?;
            super::json(&cfg)
        }

        (Method::Post, ["demo", "reroll"]) => {
            let cfg = storage::get_demo_config(&kv).await.unwrap_or_default();
            match crate::handlers::demo_personas_list::regenerate_and_store(
                &env,
                &kv,
                &db,
                &cfg.persona_generation_prompt,
            )
            .await
            {
                Ok(response) => {
                    audit::log_action(&db, &actor, "reroll_demo_personas", "demo", None, None)
                        .await?;
                    super::json(&response)
                }
                Err(msg) => super::error("reroll_failed", &msg, 502),
            }
        }

        // --- pricing --------------------------------------------------
        (Method::Get, ["pricing"]) => super::json(&pricing_view(&storage::get_pricing(&db).await)),

        (Method::Put, ["pricing"]) => {
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let min_credits = super::field_i64(&body, "min_credits");
            let max_credits = super::field_i64(&body, "max_credits");
            if let (Some(min), Some(max)) = (min_credits, max_credits) {
                if min <= 0 || max < min {
                    return super::invalid("Credit bounds must be positive, with max ≥ min.");
                }
                // The ceiling isn't a preference: `calculate_total`
                // multiplies credits by the milli price, and a large enough
                // max would overflow that.
                if max > crate::billing::MAX_CREDITS_CEILING {
                    return super::invalid(&format!(
                        "Maximum credits can't exceed {}.",
                        crate::billing::MAX_CREDITS_CEILING
                    ));
                }
                storage::update_pricing_config(&db, min, max).await?;
            }

            // Amounts arrive as the same flat list `GET` returns.
            if let Some(items) = body.get("amounts").and_then(|v| v.as_array()) {
                for item in items {
                    let concept = super::field(item, "concept");
                    let currency = super::field(item, "currency").to_uppercase();
                    let Some(amount) = super::field_i64(item, "amount") else {
                        continue;
                    };
                    let Some(concept) = storage::PricingConcept::from_wire(&concept) else {
                        return super::invalid(&format!("Unknown pricing concept {concept:?}."));
                    };
                    if amount < 0 {
                        return super::invalid("Amounts can't be negative.");
                    }
                    storage::upsert_pricing_amount(&db, concept, &currency, amount).await?;
                }
            }

            audit::log_action(&db, &actor, "update_pricing", "pricing", None, None).await?;
            super::json(&pricing_view(&storage::get_pricing(&db).await))
        }

        (Method::Delete, ["pricing", "currency", code]) => {
            let code = code.to_uppercase();
            storage::delete_pricing_currency(&db, &code).await?;
            audit::log_action(
                &db,
                &actor,
                "delete_pricing_currency",
                "pricing",
                Some(&code),
                None,
            )
            .await?;
            super::no_content()
        }

        // --- audit log ------------------------------------------------
        (Method::Get, ["audit"]) => {
            const PAGE_SIZE: u32 = 50;
            let url = req.url()?;
            let mut actor_filter = String::new();
            let mut action = String::new();
            let mut resource_type = String::new();
            let mut before = String::new();
            for (k, v) in url.query_pairs() {
                match k.as_ref() {
                    "actor" => actor_filter = v.into_owned(),
                    "action" => action = v.into_owned(),
                    "resource_type" => resource_type = v.into_owned(),
                    "before" => before = v.into_owned(),
                    _ => {}
                }
            }
            let log = audit::search_audit_log(
                &db,
                &actor_filter,
                &action,
                &resource_type,
                &before,
                PAGE_SIZE,
            )
            .await?;
            let has_more = log.len() as u32 == PAGE_SIZE;
            super::json(&AuditList {
                entries: log
                    .iter()
                    .filter_map(|e| serde_json::to_value(e).ok())
                    .collect(),
                has_more,
            })
        }

        // --- schema reseed --------------------------------------------
        // Destructive, and gated a second time inside on ALLOW_SCHEMA_RESEED.
        // POST only: a GET that wiped the database would fire on a prefetch.
        (Method::Post, ["reseed"]) => {
            crate::management::reseed::handle_reseed(req, &env, &db, &actor).await
        }

        _ => super::not_found("Endpoint"),
    }
}

fn pricing_view(cfg: &storage::Pricing) -> PricingView {
    PricingView {
        min_credits: cfg.min_credits,
        max_credits: cfg.max_credits,
        max_credits_ceiling: crate::billing::MAX_CREDITS_CEILING,
        amounts: cfg
            .amounts
            .iter()
            .map(|((concept, currency), amount)| PricingAmount {
                // `as_wire`, not Debug: this string round-trips through
                // `from_wire` on the way back in.
                concept: concept.as_wire().to_string(),
                currency: currency.clone(),
                amount: *amount,
            })
            .collect(),
        concepts: storage::PricingConcept::ALL
            .iter()
            .map(|c| ConceptView {
                wire: c.as_wire(),
                label: c.label(),
                unit_caption: c.unit_caption(),
                is_milli: c.is_milli(),
            })
            .collect(),
        currencies: cfg.currencies(),
    }
}

/// Build an `Archetype` from an operator payload, validating the fields a
/// persona can't be composed without.
fn read_archetype(slug: &str, body: &serde_json::Value) -> std::result::Result<Archetype, String> {
    let slug = slug.trim().to_lowercase();
    if slug.is_empty() {
        return Err("Slug is required.".into());
    }

    let required = |key: &str, label: &str| -> std::result::Result<String, String> {
        let v = super::field(body, key);
        if v.is_empty() {
            Err(format!("{label} is required."))
        } else {
            Ok(v)
        }
    };
    let chips = |key: &str| -> Vec<String> {
        body.get(key)
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };

    Ok(Archetype {
        slug,
        label: required("label", "Label")?,
        description: required("description", "Description")?,
        voice_prompt: required("voice_prompt", "Voice prompt")?,
        greeting: required("greeting", "Greeting")?,
        catch_phrases: chips("catch_phrases"),
        off_topics: chips("off_topics"),
        never: super::field(body, "never"),
        handoff_conditions: chips("handoff_conditions"),
        // Every write re-enters the safety queue: an operator editing a
        // voice prompt must not inherit the old row's approval.
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
