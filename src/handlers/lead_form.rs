//! Lead capture form public handler

use worker::*;

use super::get_origin;
use crate::ai;
use crate::helpers::*;
use crate::storage::*;
use crate::templates::*;
use crate::types::*;
use crate::whatsapp::send_whatsapp_message;

/// Handle /lead/* routes
pub async fn handle_lead_form(
    mut req: Request,
    env: Env,
    path: &str,
    method: Method,
) -> Result<Response> {
    let path_parts: Vec<&str> = path
        .strip_prefix("/lead/")
        .unwrap_or("")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let origin = get_origin(&req);

    match (method, path_parts.as_slice()) {
        // CORS preflight
        (Method::Options, [form_id, _slug]) => {
            let kv = env.kv("KV")?;
            let allowed = match get_lead_form(&kv, form_id).await? {
                Some(f) if f.enabled => f.allowed_origins,
                _ => Vec::new(),
            };
            cors_preflight(origin.as_deref(), &allowed)
        }

        // Serve the lead form
        (Method::Get, [form_id, _slug]) => {
            let kv = env.kv("KV")?;
            let form = match get_lead_form(&kv, form_id).await? {
                Some(f) if f.enabled => f,
                _ => return Response::error("Form not found", 404),
            };

            let resp = Response::from_html(lead_form_html(&form))?;
            Ok(with_cors(resp, origin.as_deref(), &form.allowed_origins))
        }

        // Handle form submission
        (Method::Post, [form_id, _slug]) => {
            let kv = env.kv("KV")?;
            let db = env.d1("DB")?;

            // Rate limit: 10 submissions per form per IP per hour
            let client_ip = req
                .headers()
                .get("CF-Connecting-IP")
                .ok()
                .flatten()
                .unwrap_or_default();
            if !client_ip.is_empty() {
                let rl_key = format!("ratelimit:lead:{}:{}", form_id, client_ip);
                let count: i64 = kv
                    .get(&rl_key)
                    .text()
                    .await
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                if count >= 10 {
                    return Response::error("Too many submissions. try again later.", 429);
                }
                let _ = kv
                    .put(&rl_key, (count + 1).to_string())?
                    .expiration_ttl(3600)
                    .execute()
                    .await;
            }

            let form = match get_lead_form(&kv, form_id).await? {
                Some(f) if f.enabled => f,
                _ => return Response::error("Form not found", 404),
            };

            let form_data = req.form_data().await?;
            let phone = match form_data.get("phone") {
                Some(FormEntry::Field(p)) => {
                    let p = p.trim().to_string();
                    // Basic E.164-ish validation: optional +, then 7-15 digits
                    let digits: String = p.chars().filter(|c| c.is_ascii_digit()).collect();
                    if digits.len() < 7 || digits.len() > 15 {
                        let resp = Response::from_html(lead_form_error_html(
                            &form,
                            "enter a valid phone number (7-15 digits).",
                        ))?;
                        return Ok(with_cors(resp, origin.as_deref(), &form.allowed_origins));
                    }
                    p
                }
                _ => {
                    let resp = Response::from_html(lead_form_error_html(
                        &form,
                        "enter a valid phone number.",
                    ))?;
                    return Ok(with_cors(resp, origin.as_deref(), &form.allowed_origins));
                }
            };

            // Load the linked WhatsApp account
            let wa_account = match get_whatsapp_account(&kv, &form.whatsapp_account_id).await? {
                Some(a) => a,
                None => {
                    let resp = Response::from_html(lead_form_error_html(
                        &form,
                        "This form is not properly configured.",
                    ))?;
                    return Ok(with_cors(resp, origin.as_deref(), &form.allowed_origins));
                }
            };

            // Platform token
            let platform_token = env
                .secret("WHATSAPP_ACCESS_TOKEN")
                .map(|s| s.to_string())
                .unwrap_or_default();

            // Generate message
            let message = match form.reply_mode {
                AutoReplyMode::Ai => {
                    if form.reply_prompt.is_empty() {
                        "Thanks for reaching out! We'll be in touch soon.".to_string()
                    } else {
                        let mut context = serde_json::Map::new();
                        context.insert(
                            "phone_number".to_string(),
                            serde_json::Value::String(phone.clone()),
                        );
                        match ai::generate_response(&env, &form.reply_prompt, &context).await {
                            Ok(r) => r,
                            Err(e) => {
                                console_log!("AI error for lead form: {:?}", e);
                                interpolate_or_default(&form.reply_prompt, &phone)
                            }
                        }
                    }
                }
                AutoReplyMode::Static => {
                    if form.reply_prompt.is_empty() {
                        "Thanks for reaching out! We'll be in touch soon.".to_string()
                    } else {
                        interpolate_or_default(&form.reply_prompt, &phone)
                    }
                }
            };

            // Send WhatsApp message
            if let Err(e) = send_whatsapp_message(
                &platform_token,
                &wa_account.phone_number_id,
                &phone,
                &message,
            )
            .await
            {
                console_log!("Failed to send lead form WhatsApp: {:?}", e);
            }

            // Log to D1
            let reply_mode_str = match form.reply_mode {
                AutoReplyMode::Static => "static",
                AutoReplyMode::Ai => "ai",
            };
            let _ = save_lead_form_submission(
                &db,
                &generate_id(),
                form_id,
                &phone,
                &form.whatsapp_account_id,
                &message,
                reply_mode_str,
                &form.tenant_id,
            )
            .await;

            let resp = Response::from_html(lead_form_success_html(&form))?;
            Ok(with_cors(resp, origin.as_deref(), &form.allowed_origins))
        }

        _ => Response::error("Not Found", 404),
    }
}

fn interpolate_or_default(prompt: &str, phone: &str) -> String {
    let mut fields = serde_json::Map::new();
    fields.insert(
        "phone_number".to_string(),
        serde_json::Value::String(phone.to_string()),
    );
    interpolate_template(prompt, &fields)
}
