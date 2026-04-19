//! Management panel — super-admin routes gated by Cloudflare Access.
//! All /manage/* routes require Cf-Access-Authenticated-User-Email matching MANAGEMENT_EMAILS.

pub mod audit;
pub mod billing;
pub mod tenants;

use worker::*;

use crate::helpers::generate_id;
use crate::storage;
use crate::templates::management as tmpl;

/// Handle /manage/* routes. Requires Cloudflare Access.
pub async fn handle_management(
    req: Request,
    env: Env,
    path: &str,
    method: Method,
) -> Result<Response> {
    let email = match verify_access(&req, &env) {
        Some(e) => e,
        None => return Response::error("Forbidden: Cloudflare Access required", 403),
    };

    let kv = env.kv("CALENDARS_KV")?;
    let db = env.d1("DB")?;
    let base_url = crate::handlers::get_base_url(&req);

    let sub = path
        .strip_prefix("/manage")
        .unwrap_or("")
        .trim_start_matches('/');

    // Route subroutes first (before consuming method in match)
    if sub.starts_with("tenants") {
        return tenants::handle_tenants(req, &env, &kv, &db, sub, method, &email, &base_url).await;
    }

    if sub.starts_with("billing") {
        return billing::handle_billing(req, &kv, &db, sub, method, &email, &base_url).await;
    }

    match (method, sub) {
        (Method::Get, "" | "/") => {
            let tenant_count = count_tenants(&kv).await;
            Response::from_html(tmpl::dashboard_html(&email, tenant_count, &base_url))
        }

        (Method::Get, "audit") => {
            let log = audit::get_audit_log(&db, 100).await?;
            Response::from_html(tmpl::audit_html(&log, &base_url))
        }

        _ => Response::error("Not Found", 404),
    }
}

/// Verify the request has a valid Cloudflare Access email matching MANAGEMENT_EMAILS.
fn verify_access(req: &Request, env: &Env) -> Option<String> {
    let email = req
        .headers()
        .get("Cf-Access-Authenticated-User-Email")
        .ok()??;

    let allowed = env
        .var("MANAGEMENT_EMAILS")
        .map(|v| v.to_string())
        .unwrap_or_default();

    let email_lower = email.to_lowercase();
    let is_allowed = allowed
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .any(|e| e == email_lower);

    if is_allowed {
        Some(email)
    } else {
        None
    }
}

/// Count tenants by scanning KV prefix. Returns approximate count.
async fn count_tenants(kv: &kv::KvStore) -> usize {
    match kv.list().prefix("tenant:".to_string()).execute().await {
        Ok(list) => list.keys.iter().filter(|k| !k.name.contains(':')).count(),
        Err(_) => 0,
    }
}
