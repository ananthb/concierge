//! Management billing: pricing settings and recurring credit grants.

use worker::*;

use crate::management::audit;
use crate::storage;
use crate::templates::management as tmpl;

pub async fn handle_billing(
    mut req: Request,
    _kv: &kv::KvStore,
    db: &D1Database,
    sub: &str,
    method: Method,
    actor_email: &str,
    base_url: &str,
) -> Result<Response> {
    let parts: Vec<&str> = sub
        .strip_prefix("billing")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let locale = crate::locale::Locale::from_request(&req);

    match (method, parts.as_slice()) {
        // Billing overview: pricing form + recurring grants.
        (Method::Get, []) => {
            let cfg = storage::get_pricing(db).await;
            let scheduled = storage::list_scheduled_grants(db).await.unwrap_or_default();

            Response::from_html(tmpl::billing_overview_html(
                base_url, &locale, &cfg, &scheduled, None,
            ))
        }

        // Update pricing settings. Form posts a flat dict whose keys are
        // either `email_pack_size` or `<concept>__<currency>` (e.g.
        // `unit_price_milli__INR`). We walk the config + every known
        // currency × concept and upsert anything that's positive.
        (Method::Post, ["settings"]) => {
            let form: serde_json::Value = req.json().await?;
            let pick = |key: &str| -> Option<i64> {
                let v = form.get(key)?;
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
            };

            // Currency-agnostic settings.
            if let Some(n) = pick("email_pack_size") {
                if n <= 0 {
                    return Response::from_html(
                        r#"<div class="error">Invalid value for email_pack_size: must be a positive integer.</div>"#.to_string(),
                    );
                }
                storage::update_pricing_config(db, n).await?;
            }

            // Per-(concept, currency) cells. We accept any currency code
            // the form sends, so adding a currency client-side just works.
            let cfg = storage::get_pricing(db).await;
            let mut codes = cfg.currencies();
            // Form may also carry brand-new currency codes via the
            // `__currencies` JSON array (added by the "Add currency" UI).
            if let Some(extra) = form.get("__currencies").and_then(|v| v.as_array()) {
                for c in extra {
                    if let Some(s) = c.as_str() {
                        let s = s.to_uppercase();
                        if !codes.contains(&s) {
                            codes.push(s);
                        }
                    }
                }
            }
            for concept in storage::PricingConcept::ALL {
                for code in &codes {
                    let key = format!("{}__{}", concept.as_wire(), code);
                    if let Some(n) = pick(&key) {
                        if n <= 0 {
                            return Response::from_html(format!(
                                r#"<div class="error">Invalid value for {}: must be a positive integer.</div>"#,
                                crate::helpers::html_escape(&key),
                            ));
                        }
                        storage::upsert_pricing_amount(db, concept, code, n).await?;
                    }
                }
            }

            audit::log_action(
                db,
                actor_email,
                "update_pricing",
                "billing",
                None,
                Some(&form),
            )
            .await?;

            Response::from_html(
                r#"<div class="success">Pricing settings updated.</div>"#.to_string(),
            )
        }

        // Remove every row for a currency.
        (Method::Delete, ["currency", code]) => {
            let code = code.to_uppercase();
            storage::delete_pricing_currency(db, &code).await?;
            audit::log_action(
                db,
                actor_email,
                "delete_pricing_currency",
                "billing",
                Some(&code),
                None,
            )
            .await?;
            rerender_with_msg(
                db,
                base_url,
                &locale,
                Some(&format!(
                    r#"<div class="success">Removed currency {}.</div>"#,
                    crate::helpers::html_escape(&code)
                )),
            )
            .await
        }

        // Create a scheduled credit grant. Always applies to every tenant.
        (Method::Post, ["schedule"]) => {
            let form: serde_json::Value = req.json().await?;
            let cadence_wire = form.get("cadence").and_then(|v| v.as_str()).unwrap_or("");
            let cadence = match crate::types::GrantCadence::from_wire(cadence_wire) {
                Some(c) => c,
                None => {
                    return rerender_with_msg(
                        db,
                        base_url,
                        &locale,
                        Some(&format!(
                            "<div class=\"error\">Unknown cadence \"{}\"</div>",
                            crate::helpers::html_escape(cadence_wire)
                        )),
                    )
                    .await
                }
            };
            let credits = form
                .get("credits")
                .and_then(|v| {
                    v.as_str()
                        .and_then(|s| s.parse::<i64>().ok())
                        .or_else(|| v.as_i64())
                })
                .unwrap_or(0);
            if credits <= 0 {
                return rerender_with_msg(
                    db,
                    base_url,
                    &locale,
                    Some(r#"<div class="error">Credits must be positive.</div>"#),
                )
                .await;
            }
            let expires_in_days = form
                .get("expires_in_days")
                .and_then(|v| {
                    v.as_str()
                        .and_then(|s| s.parse::<i64>().ok())
                        .or_else(|| v.as_i64())
                })
                .unwrap_or(0);

            let now = crate::helpers::now_iso();
            let next_run_at = crate::billing::cadence::next_run_after(&now, cadence);
            let g = crate::types::ScheduledGrant {
                id: crate::helpers::generate_id(),
                cadence,
                credits,
                expires_in_days,
                last_run_at: None,
                next_run_at,
                active: true,
                created_at: now.clone(),
                updated_at: now,
            };
            storage::insert_scheduled_grant(db, &g).await?;
            audit::log_action(
                db,
                actor_email,
                "schedule_grant",
                "billing",
                Some(&g.id),
                Some(&serde_json::json!({
                    "cadence": g.cadence.as_wire(),
                    "credits": g.credits,
                    "expires_in_days": g.expires_in_days,
                })),
            )
            .await?;

            rerender_with_msg(
                db,
                base_url,
                &locale,
                Some(r#"<div class="success">Scheduled grant added.</div>"#),
            )
            .await
        }

        // Remove a scheduled grant.
        (Method::Delete, ["schedule", id]) => {
            storage::delete_scheduled_grant(db, id).await?;
            audit::log_action(
                db,
                actor_email,
                "schedule_grant_remove",
                "billing",
                Some(id),
                None,
            )
            .await?;
            rerender_with_msg(
                db,
                base_url,
                &locale,
                Some(r#"<div class="success">Scheduled grant removed.</div>"#),
            )
            .await
        }

        _ => Response::error("Not Found", 404),
    }
}

/// Re-render the /manage/billing page with an inline message rendered into
/// the schedule form's toast slot. Used by the schedule POST/DELETE handlers
/// so the operator sees validation errors and success notes in context.
async fn rerender_with_msg(
    db: &D1Database,
    base_url: &str,
    locale: &crate::locale::Locale,
    msg: Option<&str>,
) -> Result<Response> {
    let cfg = storage::get_pricing(db).await;
    let scheduled = storage::list_scheduled_grants(db).await.unwrap_or_default();
    Response::from_html(tmpl::billing_overview_html(
        base_url, locale, &cfg, &scheduled, msg,
    ))
}
