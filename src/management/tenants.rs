//! Tenant management — list, view, suspend, delete tenants.

use worker::*;

use crate::helpers::generate_id;
use crate::management::audit;
use crate::storage::*;
use crate::templates::management as tmpl;
use crate::types::*;

pub async fn handle_tenants(
    mut req: Request,
    env: &Env,
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
            let tenants = list_all_tenants(kv).await?;
            Response::from_html(tmpl::tenants_list_html(&tenants, base_url))
        }

        // View single tenant
        (Method::Get, [id]) => {
            let tenant = match get_tenant(kv, id).await? {
                Some(t) => t,
                None => return Response::error("Tenant not found", 404),
            };
            let wa = list_whatsapp_accounts(kv, id).await?;
            let ig = list_instagram_accounts(kv, id).await?;
            let domains = get_email_domains(kv, id).await?;
            Response::from_html(tmpl::tenant_detail_html(
                &tenant, &wa, &ig, &domains, base_url,
            ))
        }

        // Update tenant (plan, suspend)
        (Method::Put, [id]) => {
            let form: serde_json::Value = req.json().await?;
            let mut tenant = match get_tenant(kv, id).await? {
                Some(t) => t,
                None => return Response::error("Tenant not found", 404),
            };

            if let Some(plan) = form.get("plan").and_then(|v| v.as_str()) {
                tenant.plan = plan.to_string();
            }
            tenant.updated_at = crate::helpers::now_iso();
            save_tenant(kv, &tenant).await?;

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

/// List all tenants by scanning KV prefix.
async fn list_all_tenants(kv: &kv::KvStore) -> Result<Vec<Tenant>> {
    let list = kv
        .list()
        .prefix("tenant:".to_string())
        .execute()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let mut tenants = Vec::new();
    for key in &list.keys {
        // Skip reverse indexes like tenant_email: and sub-keys like tenant:id:whatsapp:
        if key.name.starts_with("tenant:") && key.name.matches(':').count() == 1 {
            if let Some(tenant) = get_tenant(kv, &key.name[7..]).await? {
                tenants.push(tenant);
            }
        }
    }
    tenants.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(tenants)
}
