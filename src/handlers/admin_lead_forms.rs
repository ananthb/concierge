//! Admin handlers for lead capture form resources

use worker::*;

use crate::helpers::*;
use crate::storage::*;
use crate::templates::*;
use crate::types::*;

/// Handle /admin/lead-forms routes
pub async fn handle_lead_forms_admin(
    mut req: Request,
    env: Env,
    path: &str,
    _method: Method,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("CALENDARS_KV")?;

    let path_parts: Vec<&str> = path
        .strip_prefix("/admin/lead-forms")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let method = req.method();

    match (method, path_parts.as_slice()) {
        // List all lead forms
        (Method::Get, []) => {
            let forms = list_lead_forms(&kv, tenant_id).await?;
            Response::from_html(admin_lead_forms_list_html(&forms, base_url))
        }

        // Create new lead form (GET /admin/lead-forms/new or POST /admin/lead-forms)
        (Method::Get, ["new"]) | (Method::Post, []) => {
            let now = now_iso();
            let slug = generate_slug();
            let id = generate_id();
            let form = LeadCaptureForm {
                id: id.clone(),
                tenant_id: tenant_id.to_string(),
                name: String::from("New Lead Form"),
                slug,
                whatsapp_account_id: String::new(),
                reply_mode: AutoReplyMode::Static,
                reply_prompt: String::from("Thanks for reaching out! We'll be in touch soon."),
                style: LeadFormStyle::default(),
                allowed_origins: Vec::new(),
                enabled: true,
                created_at: now.clone(),
                updated_at: now,
            };
            save_lead_form(&kv, &form).await?;

            let headers = Headers::new();
            headers.set("Location", &format!("{}/admin/lead-forms/{}", base_url, id))?;
            Ok(Response::empty()?.with_status(302).with_headers(headers))
        }

        // Edit page
        (Method::Get, [id]) => {
            let form = match get_lead_form(&kv, id).await? {
                Some(f) => f,
                None => return Response::error("Lead form not found", 404),
            };
            if form.tenant_id != tenant_id {
                return Response::error("Not found", 404);
            }
            let whatsapp_accounts = list_whatsapp_accounts(&kv, tenant_id).await?;
            Response::from_html(admin_lead_form_edit_html(
                &form,
                &whatsapp_accounts,
                base_url,
            ))
        }

        // Update lead form
        (Method::Put, [id]) => {
            let mut form = match get_lead_form(&kv, id).await? {
                Some(f) => f,
                None => return Response::error("Lead form not found", 404),
            };
            if form.tenant_id != tenant_id {
                return Response::error("Not found", 404);
            }

            let data = req.form_data().await?;
            if let Some(FormEntry::Field(name)) = data.get("name") {
                form.name = truncate(&name, 200);
            }
            if let Some(FormEntry::Field(wa_id)) = data.get("whatsapp_account_id") {
                form.whatsapp_account_id = wa_id;
            }
            if let Some(FormEntry::Field(mode)) = data.get("reply_mode") {
                form.reply_mode = match mode.as_str() {
                    "ai" => AutoReplyMode::Ai,
                    _ => AutoReplyMode::Static,
                };
            }
            if let Some(FormEntry::Field(prompt)) = data.get("reply_prompt") {
                form.reply_prompt = truncate(&prompt, 2000);
            }
            if let Some(FormEntry::Field(origins)) = data.get("allowed_origins") {
                form.allowed_origins = origins
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            form.enabled = data.get("enabled").is_some();

            // Style fields
            if let Some(FormEntry::Field(v)) = data.get("style_primary_color") {
                form.style.primary_color = truncate(&v, 50);
            }
            if let Some(FormEntry::Field(v)) = data.get("style_text_color") {
                form.style.text_color = truncate(&v, 50);
            }
            if let Some(FormEntry::Field(v)) = data.get("style_background_color") {
                form.style.background_color = truncate(&v, 50);
            }
            if let Some(FormEntry::Field(v)) = data.get("style_border_radius") {
                form.style.border_radius = truncate(&v, 50);
            }
            if let Some(FormEntry::Field(v)) = data.get("style_button_text") {
                form.style.button_text = truncate(&v, 200);
            }
            if let Some(FormEntry::Field(v)) = data.get("style_placeholder_text") {
                form.style.placeholder_text = truncate(&v, 200);
            }
            if let Some(FormEntry::Field(v)) = data.get("style_success_message") {
                form.style.success_message = truncate(&v, 200);
            }
            // custom_css removed — XSS risk with no benefit

            form.updated_at = now_iso();
            save_lead_form(&kv, &form).await?;
            Response::from_html(admin_success_html("Lead form updated"))
        }

        // Delete lead form
        (Method::Delete, [id]) => {
            let form = match get_lead_form(&kv, id).await? {
                Some(f) => f,
                None => return Response::error("Lead form not found", 404),
            };
            if form.tenant_id != tenant_id {
                return Response::error("Not found", 404);
            }
            delete_lead_form(&kv, tenant_id, id).await?;
            Response::empty()
        }

        _ => Response::error("Not Found", 404),
    }
}
