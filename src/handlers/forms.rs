//! Public form handlers - rendering and submission

use worker::*;

use crate::ai;
use crate::helpers::*;
use crate::responders::{send_resend_email, send_twilio_email, send_twilio_message};
use crate::sheets;
use crate::storage::*;
use crate::templates::*;
use crate::types::*;

/// Handle public form routes (/f/*)
pub async fn handle_form_routes(
    req: Request,
    env: Env,
    path: &str,
    method: Method,
) -> Result<Response> {
    let kv = env.kv("FORMS_KV")?;

    let path_parts: Vec<&str> = path.strip_prefix("/f/").unwrap_or("").split('/').collect();
    let slug = path_parts.first().copied().unwrap_or("");
    let action = path_parts.get(1).unwrap_or(&"");

    let origin = req.headers().get("Origin").ok().flatten();

    // Handle CORS preflight early - try to load form for allowed_origins
    if method == Method::Options {
        if !slug.is_empty() {
            if let Ok(Some(form)) = get_form(&kv, slug).await {
                return cors_preflight(origin.as_deref(), &form.allowed_origins);
            }
        }
        // Form not found or invalid path - return permissive CORS for preflight
        return cors_preflight(origin.as_deref(), &[]);
    }

    if slug.is_empty() {
        return Response::error("Form slug required", 400);
    }

    let form = match get_form(&kv, slug).await? {
        Some(f) => f,
        None => return Response::error("Form not found", 404),
    };

    // Check origin for non-GET requests
    let self_origin = req.url()?.origin().ascii_serialization();
    let is_same_origin = origin.as_ref().map(|o| o == &self_origin).unwrap_or(false);

    if method != Method::Get && !is_same_origin
        && !form.allowed_origins.is_empty() {
            match &origin {
                Some(o) if !is_origin_allowed(o, &form.allowed_origins) => {
                    return Response::error("Origin not allowed", 403);
                }
                None => {
                    return Response::error("Origin header required", 403);
                }
                _ => {}
            }
        }

    let is_htmx = is_htmx_request(&req);
    let url = req.url()?;
    let base_url = format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""));

    let response = match (method, *action) {
        (Method::Get, "") => {
            let query_pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();
            let inline_css = query_pairs.get("css").map(|s| s.to_string());
            let css_url = query_pairs.get("css_url").map(|s| s.to_string());
            Response::from_html(render_form(
                &form,
                inline_css.as_deref(),
                css_url.as_deref(),
                &base_url,
                is_htmx,
            ))
        }

        (Method::Post, "submit") => handle_form_submit(req, env, &form, &base_url).await,

        _ => Response::error("Not Found", 404),
    }?;

    let allowed = if form.allowed_origins.is_empty() {
        vec!["*".to_string()]
    } else {
        form.allowed_origins.clone()
    };
    Ok(add_cors_headers(response, origin.as_deref(), &allowed))
}

async fn handle_form_submit(
    mut req: Request,
    env: Env,
    form: &FormConfig,
    base_url: &str,
) -> Result<Response> {
    let db = match env.d1("DB") {
        Ok(db) => db,
        Err(e) => {
            console_log!("D1 binding error: {:?}", e);
            return Response::from_html(form_error_html("Database not configured"))
                .map(|r| r.with_status(500));
        }
    };
    let r2 = env.bucket("UPLOADS").ok();

    let form_data = match req.form_data().await {
        Ok(data) => data,
        Err(_) => {
            return Response::from_html(form_error_html("Invalid form data"))
                .map(|r| r.with_status(400));
        }
    };

    let mut fields_data: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    let mut attachments: Vec<String> = Vec::new();

    for field in &form.fields {
        match form_data.get(&field.id) {
            Some(FormEntry::Field(value)) => {
                if field.required && value.trim().is_empty() {
                    return Response::from_html(form_error_html(&format!(
                        "{} is required",
                        field.label
                    )))
                    .map(|r| r.with_status(400));
                }

                if matches!(field.field_type, FieldType::Email) && !value.is_empty()
                    && (!value.contains('@') || !value.contains('.')) {
                        return Response::from_html(form_error_html(
                            "Please enter a valid email address",
                        ))
                        .map(|r| r.with_status(400));
                    }

                if matches!(field.field_type, FieldType::Mobile) && !value.is_empty() {
                    let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
                    if digits.len() < 7 || digits.len() > 15 {
                        return Response::from_html(form_error_html(
                            "Please enter a valid phone number",
                        ))
                        .map(|r| r.with_status(400));
                    }
                }

                fields_data.insert(field.id.clone(), serde_json::Value::String(value));
            }

            Some(FormEntry::File(file)) => {
                let bytes = file.bytes().await?;

                if bytes.len() > 2 * 1024 * 1024 {
                    return Response::from_html(form_error_html("File size must be under 2MB"))
                        .map(|r| r.with_status(400));
                }

                if field.required && bytes.is_empty() {
                    return Response::from_html(form_error_html(&format!(
                        "{} is required",
                        field.label
                    )))
                    .map(|r| r.with_status(400));
                }

                if !bytes.is_empty() {
                    if let Some(r2) = &r2 {
                        let file_key = format!(
                            "{}/{}/{}",
                            form.slug,
                            now_iso().replace([':', '-', '.'], ""),
                            file.name()
                        );

                        r2.put(&file_key, bytes).execute().await?;

                        attachments.push(file_key.clone());
                        fields_data.insert(
                            field.id.clone(),
                            serde_json::Value::String(format!("file:{}", file_key)),
                        );
                    } else {
                        return Response::from_html(form_error_html("File uploads not configured"))
                            .map(|r| r.with_status(500));
                    }
                }
            }

            None if field.required => {
                return Response::from_html(form_error_html(&format!(
                    "{} is required",
                    field.label
                )))
                .map(|r| r.with_status(400));
            }

            None => {}
        }
    }

    let fields_json = serde_json::to_string(&fields_data).unwrap_or_else(|_| "{}".into());
    let attachments_json = if attachments.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&attachments).unwrap_or_else(|_| "[]".into()))
    };

    save_submission(&db, &form.slug, &fields_json, attachments_json.as_deref()).await?;

    // Append to Google Sheet if configured
    if let Some(sheet_url) = &form.google_sheet_url {
        if let Err(e) = sheets::append_to_sheet(&env, sheet_url, &form.fields, &fields_data).await {
            console_log!("Google Sheets error: {:?}", e);
        }
    }

    // Send auto-responses
    for responder in &form.responders {
        if !responder.enabled {
            continue;
        }

        if matches!(responder.channel, ResponderChannel::MetaWhatsapp) {
            continue;
        }

        let Some(serde_json::Value::String(target)) = fields_data.get(&responder.target_field)
        else {
            continue;
        };

        let body = if responder.use_ai {
            match ai::generate_response(&env, &responder.body, &fields_data).await {
                Ok(response) => response,
                Err(e) => {
                    console_log!("AI generation error: {:?}", e);
                    interpolate_template(&responder.body, &fields_data)
                }
            }
        } else {
            interpolate_template(&responder.body, &fields_data)
        };

        let result = match responder.channel {
            ResponderChannel::TwilioSms => {
                if let Ok(from) = env.secret("TWILIO_FROM_SMS") {
                    send_twilio_message(&env, target, &from.to_string(), &body).await
                } else {
                    continue;
                }
            }
            ResponderChannel::TwilioRcs => {
                if let Ok(from) = env.secret("TWILIO_FROM_SMS") {
                    send_twilio_message(&env, target, &from.to_string(), &body).await
                } else {
                    continue;
                }
            }
            ResponderChannel::TwilioWhatsapp => {
                if let Ok(from) = env.secret("TWILIO_FROM_WHATSAPP") {
                    send_twilio_message(
                        &env,
                        &format!("whatsapp:{}", target),
                        &from.to_string(),
                        &body,
                    )
                    .await
                } else {
                    continue;
                }
            }
            ResponderChannel::TwilioEmail => {
                if let Ok(from) = env.secret("TWILIO_FROM_EMAIL") {
                    let subject = interpolate_template(&responder.subject, &fields_data);
                    send_twilio_email(&env, target, &from.to_string(), &subject, &body).await
                } else {
                    continue;
                }
            }
            ResponderChannel::ResendEmail => {
                if let Ok(from) = env.secret("RESEND_FROM") {
                    let subject = interpolate_template(&responder.subject, &fields_data);
                    send_resend_email(&env, target, &from.to_string(), &subject, &body).await
                } else {
                    continue;
                }
            }
            ResponderChannel::MetaWhatsapp => {
                continue;
            }
        };

        if let Err(e) = result {
            console_log!("Responder error for {}: {:?}", responder.name, e);
        }
    }

    Response::from_html(form_success_html(
        &form.success_message,
        &form.slug,
        base_url,
    ))
}
