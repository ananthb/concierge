//! Tenant management: list, view, update, delete tenants.

use worker::*;

use crate::management::audit;
use crate::storage::*;
use crate::templates::management as tmpl;
use crate::types::*;

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

    match (method, parts.as_slice()) {
        // List all tenants
        (Method::Get, []) => {
            let tenants = list_tenants(db).await?;
            Response::from_html(tmpl::tenants_list_html(&tenants, base_url))
        }

        // View single tenant
        (Method::Get, [id]) => {
            let tenant = match get_tenant(db, id).await? {
                Some(t) => t,
                None => return Response::error("Tenant not found", 404),
            };
            let wa = list_whatsapp_accounts(kv, id).await?;
            let ig = list_instagram_accounts(kv, id).await?;
            let domains = get_email_subdomains(kv, id).await?;
            Response::from_html(tmpl::tenant_detail_html(
                &tenant, &wa, &ig, &domains, base_url,
            ))
        }

        // Update tenant (plan)
        (Method::Put, [id]) => {
            let form: serde_json::Value = req.json().await?;
            let mut tenant = match get_tenant(db, id).await? {
                Some(t) => t,
                None => return Response::error("Tenant not found", 404),
            };

            if let Some(plan) = form.get("plan").and_then(|v| v.as_str()) {
                tenant.plan = plan.to_string();
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
