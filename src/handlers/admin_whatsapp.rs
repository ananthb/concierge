//! Admin handlers for WhatsApp account resources

use worker::*;

use crate::helpers::*;
use crate::storage::*;
use crate::templates::*;
use crate::types::*;

/// Handle /admin/whatsapp routes
pub async fn handle_whatsapp_admin(
    mut req: Request,
    env: Env,
    path: &str,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("KV")?;

    let path_parts: Vec<&str> = path
        .strip_prefix("/admin/whatsapp")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let method = req.method();

    match (method, path_parts.as_slice()) {
        // List all WhatsApp accounts
        (Method::Get, []) => {
            let accounts = list_whatsapp_accounts(&kv, tenant_id).await?;
            Response::from_html(admin_whatsapp_list_html(&accounts, base_url))
        }

        // Embedded Signup page
        (Method::Get, ["new"]) => {
            let app_id = env
                .secret("META_APP_ID")
                .map(|s| s.to_string())
                .unwrap_or_default();
            let config_id = env
                .var("WHATSAPP_SIGNUP_CONFIG_ID")
                .map(|v| v.to_string())
                .unwrap_or_default();

            // Generate CSRF state nonce
            let state = generate_token()?;
            kv.put(&format!("wa_signup_state:{}", state), tenant_id)?
                .expiration_ttl(600)
                .execute()
                .await?;

            Response::from_html(admin_whatsapp_signup_html(
                base_url, &app_id, &config_id, &state,
            ))
        }

        // Manual fallback: create blank account
        (Method::Get, ["manual"]) | (Method::Post, []) => {
            let now = now_iso();
            let account = WhatsAppAccount {
                id: generate_id(),
                tenant_id: tenant_id.to_string(),
                name: String::from("New WhatsApp Number"),
                phone_number: String::new(),
                phone_number_id: String::new(),
                auto_reply: ReplyConfig::default(),
                created_at: now.clone(),
                updated_at: now,
            };
            save_whatsapp_account(&kv, &account).await?;

            let headers = Headers::new();
            headers.set(
                "Location",
                &format!("{}/admin/whatsapp/{}", base_url, account.id),
            )?;
            Ok(Response::empty()?.with_status(302).with_headers(headers))
        }

        // Edit page for a WhatsApp account
        (Method::Get, [id]) => {
            let account = match get_whatsapp_account(&kv, id).await? {
                Some(a) => a,
                None => return Response::error("WhatsApp account not found", 404),
            };
            if account.tenant_id != tenant_id {
                return Response::error("Not found", 404);
            }
            Response::from_html(admin_whatsapp_edit_html(&account, base_url))
        }

        // Update WhatsApp account
        (Method::Put, [id]) => {
            let mut account = match get_whatsapp_account(&kv, id).await? {
                Some(a) => a,
                None => return Response::error("WhatsApp account not found", 404),
            };
            if account.tenant_id != tenant_id {
                return Response::error("Not found", 404);
            }

            let form = req.form_data().await?;
            if let Some(FormEntry::Field(name)) = form.get("name") {
                account.name = truncate(&name, 200);
            }
            if let Some(FormEntry::Field(phone)) = form.get("phone_number") {
                account.phone_number = truncate(&phone, 20);
            }
            if let Some(FormEntry::Field(phone_id)) = form.get("phone_number_id") {
                account.phone_number_id = truncate(&phone_id, 30);
            }

            // Auto-reply: edit the default rule's response (full rules CRUD
            // lives on a separate page).
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

            account.updated_at = now_iso();
            save_whatsapp_account(&kv, &account).await?;
            Response::from_html(admin_success_html("WhatsApp account updated"))
        }

        // Delete WhatsApp account
        (Method::Delete, [id]) => {
            let account = match get_whatsapp_account(&kv, id).await? {
                Some(a) => a,
                None => return Response::error("WhatsApp account not found", 404),
            };
            if account.tenant_id != tenant_id {
                return Response::error("Not found", 404);
            }
            delete_whatsapp_account(&kv, tenant_id, id).await?;
            Response::empty()
        }

        _ => Response::error("Not Found", 404),
    }
}
