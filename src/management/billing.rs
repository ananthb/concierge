//! Management billing — grant credits, view usage across tenants.

use worker::*;

use crate::management::audit;
use crate::storage;
use crate::templates::management as tmpl;

pub async fn handle_billing(
    mut req: Request,
    kv: &kv::KvStore,
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

    match (method, parts.as_slice()) {
        // Billing overview — grant-credits form lives here.
        (Method::Get, []) => Response::from_html(tmpl::billing_overview_html(base_url)),

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
