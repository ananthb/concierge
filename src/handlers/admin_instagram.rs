//! Admin handlers for Instagram account resources

use worker::*;

use crate::helpers::*;
use crate::storage::*;
use crate::templates::*;

/// Handle /admin/instagram routes
pub async fn handle_instagram_admin(
    mut req: Request,
    env: Env,
    path: &str,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("KV")?;

    let path_parts: Vec<&str> = path
        .strip_prefix("/admin/instagram")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let method = req.method();

    match (method, path_parts.as_slice()) {
        // List all Instagram accounts
        (Method::Get, []) => {
            let accounts = list_instagram_accounts(&kv, tenant_id).await?;
            Response::from_html(admin_instagram_list_html(&accounts, base_url, tenant_id))
        }

        // Edit page for an Instagram account
        (Method::Get, [id]) => {
            let account = match get_instagram_account(&kv, id).await? {
                Some(a) => a,
                None => return Response::error("Instagram account not found", 404),
            };
            if account.tenant_id != tenant_id {
                return Response::error("Not found", 404);
            }
            Response::from_html(admin_instagram_edit_html(&account, base_url))
        }

        // Update Instagram account
        (Method::Put, [id]) => {
            let mut account = match get_instagram_account(&kv, id).await? {
                Some(a) => a,
                None => return Response::error("Instagram account not found", 404),
            };
            if account.tenant_id != tenant_id {
                return Response::error("Not found", 404);
            }

            let form = req.form_data().await?;
            account.enabled = form.get("enabled").is_some();

            // Auto-reply: edit the default rule's response.
            account.auto_reply.enabled = form.get("auto_reply_enabled").is_some();
            let mode = form
                .get("auto_reply_mode")
                .and_then(|v| match v {
                    FormEntry::Field(s) => Some(s),
                    _ => None,
                })
                .unwrap_or_else(|| "canned".to_string());
            let prompt = form
                .get("auto_reply_prompt")
                .and_then(|v| match v {
                    FormEntry::Field(s) => Some(s),
                    _ => None,
                })
                .map(|s| truncate(&s, 2000))
                .unwrap_or_default();
            account.auto_reply.set_default_response(&mode, prompt);
            if let Some(FormEntry::Field(w)) = form.get("wait_seconds") {
                if let Ok(n) = w.parse::<u32>() {
                    account.auto_reply.wait_seconds = n.min(30);
                }
            }

            account.updated_at = crate::helpers::now_iso();
            save_instagram_account(&kv, &account).await?;
            Response::from_html(admin_success_html("Instagram account updated"))
        }

        // Disconnect/delete Instagram account
        (Method::Delete, [id]) => {
            let account = match get_instagram_account(&kv, id).await? {
                Some(a) => a,
                None => return Response::error("Instagram account not found", 404),
            };
            if account.tenant_id != tenant_id {
                return Response::error("Not found", 404);
            }
            delete_instagram_account(&kv, tenant_id, id).await?;
            Response::empty()
        }

        _ => Response::error("Not Found", 404),
    }
}
