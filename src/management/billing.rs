//! Management billing — grant credits, view usage across tenants.

use worker::*;

use crate::billing;
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
        // Billing overview — grant-credits form and settings live here.
        (Method::Get, []) => {
            let milli_paise = storage::get_config_price(
                db,
                "unit_price_millipaise",
                billing::UNIT_PRICE_MILLIPAISE,
            )
            .await;
            let milli_cents = storage::get_config_price(
                db,
                "unit_price_millicents",
                billing::UNIT_PRICE_MILLICENTS,
            )
            .await;
            let address_paise =
                storage::get_config_price(db, "address_price_paise", billing::ADDRESS_PRICE_PAISE)
                    .await;
            let address_cents =
                storage::get_config_price(db, "address_price_cents", billing::ADDRESS_PRICE_CENTS)
                    .await;
            let pack_size =
                storage::get_config_price(db, "email_pack_size", billing::EMAIL_PACK_SIZE).await;
            let free_monthly =
                storage::get_config_price(db, "free_monthly_credits", billing::FREE_MONTHLY_AMOUNT)
                    .await;
            let scheduled = storage::list_scheduled_grants(db).await.unwrap_or_default();

            Response::from_html(tmpl::billing_overview_html(
                base_url,
                &locale,
                milli_paise,
                milli_cents,
                address_paise,
                address_cents,
                pack_size,
                free_monthly,
                &scheduled,
                None,
            ))
        }

        // Update global pricing settings
        (Method::Post, ["settings"]) => {
            let form: serde_json::Value = req.json().await?;
            let keys = [
                "unit_price_millipaise",
                "unit_price_millicents",
                "address_price_paise",
                "address_price_cents",
                "email_pack_size",
                "free_monthly_credits",
            ];

            for key in keys {
                let raw = match form.get(key) {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    _ => continue,
                };
                let parsed: i64 = match raw.parse() {
                    Ok(n) if n > 0 => n,
                    _ => {
                        return Response::from_html(format!(
                            r#"<div class="error">Invalid value for {}: must be a positive integer.</div>"#,
                            crate::helpers::html_escape(key),
                        ))
                    }
                };
                let value = parsed.to_string();
                db.prepare(
                    "INSERT INTO global_settings (key, value, updated_at) \
                     VALUES (?, ?, datetime('now')) \
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
                )
                .bind(&[key.into(), value.as_str().into()])?
                .run()
                .await?;
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

        // Create a scheduled credit grant. Validates each provided email
        // against tenants; if any are unknown, the form re-renders with a
        // warning and nothing is saved.
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

            let audience_kind = form
                .get("audience_kind")
                .and_then(|v| v.as_str())
                .unwrap_or("everyone");
            let raw_emails = form.get("emails").and_then(|v| v.as_str()).unwrap_or("");
            let parsed_emails: Vec<String> = raw_emails
                .split(|c: char| c == ',' || c == '\n' || c == ';' || c.is_whitespace())
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();

            let audience = if audience_kind == "emails" {
                if parsed_emails.is_empty() {
                    return rerender_with_msg(
                        db,
                        base_url,
                        &locale,
                        Some(r#"<div class="error">Pick at least one email when audience is "Specific tenants".</div>"#),
                    )
                    .await;
                }
                // Verify every email maps to a tenant; collect the misses.
                let mut unknown: Vec<String> = Vec::new();
                for em in &parsed_emails {
                    if storage::get_tenant_by_email(db, em).await?.is_none() {
                        unknown.push(em.clone());
                    }
                }
                if !unknown.is_empty() {
                    let listed = unknown
                        .iter()
                        .map(|e| crate::helpers::html_escape(e))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let msg = format!(
                        r#"<div class="error">No tenant found for: <strong>{}</strong>. Fix the emails or pick "Every tenant" and resubmit.</div>"#,
                        listed
                    );
                    return rerender_with_msg(db, base_url, &locale, Some(&msg)).await;
                }
                crate::types::GrantAudience::Emails(parsed_emails)
            } else {
                crate::types::GrantAudience::Everyone
            };

            let now = crate::helpers::now_iso();
            let next_run_at = crate::billing::cadence::next_run_after(&now, cadence);
            let g = crate::types::ScheduledGrant {
                id: crate::helpers::generate_id(),
                cadence,
                audience,
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
                    "audience_kind": audience_kind,
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

        // Grant credits to a tenant with expiry
        (Method::Post, ["grant", tenant_id]) => {
            let form: serde_json::Value = req.json().await?;
            let count = form
                .get("replies")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let expires_days = form
                .get("expires_days")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(365);

            if count <= 0 {
                return Response::from_html(
                    r#"<div class="error">Reply count must be positive</div>"#.to_string(),
                );
            }

            crate::billing::grant_with_expiry(db, tenant_id, count, expires_days).await?;

            let expires_at = crate::helpers::days_from_now(expires_days);
            audit::log_action(
                db,
                actor_email,
                "grant_replies",
                "billing",
                Some(tenant_id),
                Some(&serde_json::json!({"replies": count, "expires_in_days": expires_days, "expires_at": expires_at})),
            )
            .await?;

            let mut billing = storage::get_tenant_billing(db, tenant_id).await?;
            crate::billing::refresh_billing_async(db, &mut billing).await;
            Response::from_html(format!(
                r#"<div class="success">Granted {count} replies to {tid} (expires in {days} days). Balance: {bal}</div>"#,
                count = count,
                tid = crate::helpers::html_escape(tenant_id),
                days = expires_days,
                bal = billing.total_remaining(),
            ))
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
    let milli_paise =
        storage::get_config_price(db, "unit_price_millipaise", billing::UNIT_PRICE_MILLIPAISE)
            .await;
    let milli_cents =
        storage::get_config_price(db, "unit_price_millicents", billing::UNIT_PRICE_MILLICENTS)
            .await;
    let address_paise =
        storage::get_config_price(db, "address_price_paise", billing::ADDRESS_PRICE_PAISE).await;
    let address_cents =
        storage::get_config_price(db, "address_price_cents", billing::ADDRESS_PRICE_CENTS).await;
    let pack_size =
        storage::get_config_price(db, "email_pack_size", billing::EMAIL_PACK_SIZE).await;
    let free_monthly =
        storage::get_config_price(db, "free_monthly_credits", billing::FREE_MONTHLY_AMOUNT).await;
    let scheduled = storage::list_scheduled_grants(db).await.unwrap_or_default();
    Response::from_html(tmpl::billing_overview_html(
        base_url,
        locale,
        milli_paise,
        milli_cents,
        address_paise,
        address_cents,
        pack_size,
        free_monthly,
        &scheduled,
        msg,
    ))
}
