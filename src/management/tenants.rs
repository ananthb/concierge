//! Tenant management: list, view, update, delete tenants.

use worker::*;

use crate::management::audit;
use crate::storage::*;
use crate::templates::management as tmpl;

pub async fn handle_tenants(
    mut req: Request,
    _env: &Env,
    kv: &kv::KvStore,
    db: &D1Database,
    sub: &str,
    method: Method,
    actor_email: &str,
    base_url: &str,
) -> Result<Response> {
    let parts: Vec<&str> = sub
        .strip_prefix("tenants")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let locale = crate::locale::Locale::from_request(&req);

    match (method, parts.as_slice()) {
        // List all tenants
        (Method::Get, []) => {
            let tenants = list_tenants(db).await?;
            Response::from_html(tmpl::tenants_list_html(
                &tenants,
                actor_email,
                base_url,
                &locale,
            ))
        }

        // View single tenant
        (Method::Get, [id]) => {
            let tenant = match get_tenant(db, id).await? {
                Some(t) => t,
                None => return Response::error("Tenant not found", 404),
            };
            let wa = list_whatsapp_accounts(kv, id).await?;
            let ig = list_instagram_accounts(kv, id).await?;
            let addrs = get_email_addresses(kv, id).await?;
            let mut billing = get_tenant_billing(db, id).await?;
            crate::billing::refresh_billing(&mut billing);
            Response::from_html(tmpl::tenant_detail_html(
                &tenant,
                &wa,
                &ig,
                &addrs,
                &billing,
                actor_email,
                base_url,
                &locale,
            ))
        }

        // Grant reply credits to a tenant. The form lives on the tenant
        // detail page so operators can pick a specific tenant in context.
        (Method::Post, [id, "grant-replies"]) => {
            if get_tenant(db, id).await?.is_none() {
                return Response::error("Tenant not found", 404);
            }
            let form: serde_json::Value = req.json().await?;
            let count = parse_form_i64(&form, "replies").unwrap_or(0);
            let expires_days = parse_form_i64(&form, "expires_days").unwrap_or(365);

            if count <= 0 {
                return Response::from_html(
                    r#"<div class="error">Reply count must be positive.</div>"#.to_string(),
                );
            }
            if expires_days <= 0 {
                return Response::from_html(
                    r#"<div class="error">Expiry must be at least 1 day.</div>"#.to_string(),
                );
            }

            crate::billing::grant_with_expiry(db, id, count, expires_days).await?;

            let expires_at = crate::helpers::days_from_now(expires_days);
            audit::log_action(
                db,
                actor_email,
                "grant_replies",
                "tenant",
                Some(id),
                Some(&serde_json::json!({
                    "replies": count,
                    "expires_in_days": expires_days,
                    "expires_at": expires_at,
                })),
            )
            .await?;

            let mut billing = get_tenant_billing(db, id).await?;
            crate::billing::refresh_billing(&mut billing);
            Response::from_html(format!(
                r#"<div class="success">Granted {count} reply credits (expires in {days} days). Balance: {bal}.</div>"#,
                count = count,
                days = expires_days,
                bal = billing.total_remaining(),
            ))
        }

        // Grant reply-email address slots to a tenant. Bumps the tenant's
        // purchased-extras counter so the quota gate at /admin/email opens
        // up `count` more local-parts. Mirrors what a paid pack purchase
        // does, minus the Razorpay round-trip.
        (Method::Post, [id, "grant-addresses"]) => {
            let mut tenant = match get_tenant(db, id).await? {
                Some(t) => t,
                None => return Response::error("Tenant not found", 404),
            };
            let form: serde_json::Value = req.json().await?;
            let count = parse_form_i64(&form, "addresses").unwrap_or(0);

            if count <= 0 {
                return Response::from_html(
                    r#"<div class="error">Address count must be positive.</div>"#.to_string(),
                );
            }

            tenant.email_address_extras_purchased = tenant
                .email_address_extras_purchased
                .saturating_add(count as u32);
            tenant.updated_at = crate::helpers::now_iso();
            save_tenant(db, &tenant).await?;

            audit::log_action(
                db,
                actor_email,
                "grant_addresses",
                "tenant",
                Some(id),
                Some(&serde_json::json!({ "addresses": count })),
            )
            .await?;

            Response::from_html(format!(
                r#"<div class="success">Granted {count} address slots. New quota: {quota}.</div>"#,
                count = count,
                quota = tenant.email_address_quota(),
            ))
        }

        // Update tenant (plan)
        (Method::Put, [id]) => {
            let form: serde_json::Value = req.json().await?;
            let mut tenant = match get_tenant(db, id).await? {
                Some(t) => t,
                None => return Response::error("Tenant not found", 404),
            };

            if let Some(plan) = form
                .get("plan")
                .and_then(|v| v.as_str())
                .and_then(crate::types::Plan::from_wire)
            {
                tenant.plan = plan;
            }
            tenant.updated_at = crate::helpers::now_iso();
            save_tenant(db, &tenant).await?;

            audit::log_action(
                db,
                actor_email,
                "update_tenant",
                "tenant",
                Some(id),
                Some(&form),
            )
            .await?;

            Response::from_html(r#"<div class="success">Tenant updated</div>"#.to_string())
        }

        // Delete tenant
        (Method::Delete, [id]) => {
            audit::log_action(db, actor_email, "delete_tenant", "tenant", Some(id), None).await?;

            delete_tenant_data(kv, db, id).await?;
            Ok(Response::empty()?.with_status(200))
        }

        _ => Response::error("Not Found", 404),
    }
}

/// Pull a stringy or numeric form field as i64. The management forms post
/// as JSON-encoded HTMX values, which arrive as strings; the same handler
/// is reused by tests that may pass numbers, so accept both.
fn parse_form_i64(form: &serde_json::Value, key: &str) -> Option<i64> {
    let v = form.get(key)?;
    v.as_i64()
        .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
}
