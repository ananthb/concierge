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

            Response::from_html(tmpl::billing_overview_html(
                base_url,
                &locale,
                milli_paise,
                milli_cents,
                address_paise,
                address_cents,
                pack_size,
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
            crate::billing::refresh_billing(&mut billing);
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
