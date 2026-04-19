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
        // Billing overview with pack management
        (Method::Get, []) => {
            let packs = storage::get_all_credit_packs(db).await?;
            Response::from_html(tmpl::billing_overview_html(&packs, base_url))
        }

        // Add a new pack
        (Method::Post, ["packs"]) => {
            let form: serde_json::Value = req.json().await?;
            let name = form
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("New Pack");
            let replies = form
                .get("replies")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100);
            let price_inr = form
                .get("price_inr")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let price_usd = form
                .get("price_usd")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let sort_order = form
                .get("sort_order")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(99);

            storage::save_credit_pack(db, name, replies, price_inr, price_usd, sort_order).await?;

            audit::log_action(
                db,
                actor_email,
                "create_pack",
                "credit_pack",
                None,
                Some(&form),
            )
            .await?;

            let headers = worker::Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/manage/billing"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Update a pack
        (Method::Put, ["packs", id_str]) => {
            let id = id_str
                .parse::<i64>()
                .map_err(|_| Error::from("Invalid ID"))?;
            let form: serde_json::Value = req.json().await?;
            let name = form.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let replies = form
                .get("replies")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let price_inr = form
                .get("price_inr")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let price_usd = form
                .get("price_usd")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let active = form.get("active").and_then(|v| v.as_str()) == Some("true");
            let sort_order = form
                .get("sort_order")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);

            storage::update_credit_pack(
                db, id, name, replies, price_inr, price_usd, active, sort_order,
            )
            .await?;

            audit::log_action(
                db,
                actor_email,
                "update_pack",
                "credit_pack",
                Some(id_str),
                Some(&form),
            )
            .await?;

            Response::from_html(r#"<div class="success">Pack updated</div>"#.to_string())
        }

        // Delete a pack
        (Method::Delete, ["packs", id_str]) => {
            let id = id_str
                .parse::<i64>()
                .map_err(|_| Error::from("Invalid ID"))?;
            storage::delete_credit_pack(db, id).await?;
            audit::log_action(
                db,
                actor_email,
                "delete_pack",
                "credit_pack",
                Some(id_str),
                None,
            )
            .await?;
            Ok(Response::empty()?.with_status(200))
        }

        // Grant free credits to a tenant
        (Method::Post, ["grant", tenant_id]) => {
            let form: serde_json::Value = req.json().await?;
            let count = form
                .get("replies")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);

            if count <= 0 {
                return Response::from_html(
                    r#"<div class="error">Reply count must be positive</div>"#.to_string(),
                );
            }

            crate::billing::grant_replies(kv, tenant_id, count).await?;

            audit::log_action(
                db,
                actor_email,
                "grant_replies",
                "billing",
                Some(tenant_id),
                Some(&serde_json::json!({"replies": count})),
            )
            .await?;

            let billing = storage::get_tenant_billing(kv, tenant_id).await?;
            Response::from_html(format!(
                r#"<div class="success">Granted {count} replies to {tid}. Balance: {bal}</div>"#,
                count = count,
                tid = crate::helpers::html_escape(tenant_id),
                bal = billing.replies_remaining,
            ))
        }

        _ => Response::error("Not Found", 404),
    }
}
