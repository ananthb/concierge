//! Admin handlers for forms and calendars

use worker::*;

use super::get_base_url;
use crate::helpers::*;
use crate::storage::*;
use crate::templates::*;
use crate::types::*;

/// Unified admin handler - routes to forms or calendars admin
pub async fn handle_admin(req: Request, env: Env, path: &str, method: Method) -> Result<Response> {
    let is_dev = env
        .var("ENVIRONMENT")
        .map(|v| v.to_string() == "development")
        .unwrap_or(false);

    let access_user = req
        .headers()
        .get("Cf-Access-Authenticated-User-Email")
        .ok()
        .flatten();

    if !is_dev && access_user.is_none() {
        return Response::error(
            "Unauthorized: Admin requires Cloudflare Access. See README for setup instructions.",
            403,
        );
    }

    let base_url = get_base_url(&req);

    if path.starts_with("/admin/forms") {
        return handle_forms_admin(req, env, path, method, &base_url, access_user.as_deref()).await;
    }

    if path.starts_with("/admin/calendars") {
        return handle_calendars_admin(req, env, path, method, &base_url).await;
    }

    if path == "/admin" || path == "/admin/" {
        let forms_kv = env.kv("FORMS_KV")?;
        let calendars_kv = env.kv("CALENDARS_KV")?;

        let forms = list_forms(&forms_kv).await?;
        let calendars = list_calendars(&calendars_kv).await?;

        let mut response_counts = std::collections::HashMap::new();
        if let Ok(db) = env.d1("DB") {
            for form in &forms {
                if let Ok(count) = get_submission_count(&db, &form.slug).await {
                    response_counts.insert(form.slug.clone(), count);
                }
            }
        }

        let mut resp = Response::from_html(admin_dashboard_html(
            &forms,
            &calendars,
            &response_counts,
            &base_url,
        ))?;
        resp.headers_mut().set("Cache-Control", "no-store")?;
        return Ok(resp);
    }

    Response::error("Not Found", 404)
}

async fn handle_forms_admin(
    mut req: Request,
    env: Env,
    path: &str,
    method: Method,
    _base_url: &str,
    access_user: Option<&str>,
) -> Result<Response> {
    let kv = env.kv("FORMS_KV")?;

    match (method, path) {
        (Method::Get, "/admin/forms/new") => {
            let channels = crate::templates::AvailableChannels {
                twilio_sms: env.secret("TWILIO_SID").is_ok()
                    && env.secret("TWILIO_FROM_SMS").is_ok(),
                twilio_whatsapp: env.secret("TWILIO_SID").is_ok()
                    && env.secret("TWILIO_FROM_WHATSAPP").is_ok(),
                twilio_email: env.secret("SENDGRID_API_KEY").is_ok()
                    && env.secret("TWILIO_FROM_EMAIL").is_ok(),
                resend_email: env.secret("RESEND_API_KEY").is_ok()
                    && env.secret("RESEND_FROM").is_ok(),
            };
            Response::from_html(form_editor_html(None, access_user.unwrap_or(""), &channels))
        }

        (Method::Get, p) if p.starts_with("/admin/forms/") && p.ends_with("/responses") => {
            let slug = p
                .strip_prefix("/admin/forms/")
                .and_then(|s| s.strip_suffix("/responses"))
                .unwrap_or("");
            if slug.is_empty() {
                return Response::error("Slug required", 400);
            }
            let form = match get_form(&kv, slug).await? {
                Some(f) => f,
                None => return Response::error("Form not found", 404),
            };

            let db = env.d1("DB")?;
            let submissions = get_submissions(&db, slug, 500).await.unwrap_or_default();
            Response::from_html(responses_view_html(&form, &submissions))
        }

        (Method::Get, p) if p.starts_with("/admin/forms/") => {
            let slug = p.strip_prefix("/admin/forms/").unwrap_or("");
            if slug.is_empty() {
                return Response::error("Slug required", 400);
            }
            match get_form(&kv, slug).await? {
                Some(form) => {
                    let channels = crate::templates::AvailableChannels {
                        twilio_sms: env.secret("TWILIO_SID").is_ok()
                            && env.secret("TWILIO_FROM_SMS").is_ok(),
                        twilio_whatsapp: env.secret("TWILIO_SID").is_ok()
                            && env.secret("TWILIO_FROM_WHATSAPP").is_ok(),
                        twilio_email: env.secret("SENDGRID_API_KEY").is_ok()
                            && env.secret("TWILIO_FROM_EMAIL").is_ok(),
                        resend_email: env.secret("RESEND_API_KEY").is_ok()
                            && env.secret("RESEND_FROM").is_ok(),
                    };
                    Response::from_html(form_editor_html(Some(&form), access_user.unwrap_or(""), &channels))
                }
                None => Response::error("Form not found", 404),
            }
        }

        (Method::Post, "/admin/forms") => {
            let body: serde_json::Value = req.json().await?;

            let slug = body["slug"].as_str().unwrap_or("").to_string();
            if slug.is_empty()
                || !slug
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
            {
                return Response::error("Invalid slug", 400);
            }

            if get_form(&kv, &slug).await?.is_some() {
                return Response::error("Slug already exists", 409);
            }

            let now = now_iso();
            let form = FormConfig {
                slug,
                name: body["name"].as_str().unwrap_or("Untitled").to_string(),
                title: body["title"].as_str().unwrap_or("Contact Us").to_string(),
                submit_button_text: body["submit_button_text"]
                    .as_str()
                    .unwrap_or("Submit")
                    .to_string(),
                success_message: body["success_message"]
                    .as_str()
                    .unwrap_or("Thank you!")
                    .to_string(),
                allowed_origins: body["allowed_origins"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                fields: serde_json::from_value(body["fields"].clone())
                    .unwrap_or_else(|_| FormConfig::default_fields()),
                style: serde_json::from_value(body["style"].clone()).unwrap_or_default(),
                responders: serde_json::from_value(body["responders"].clone()).unwrap_or_default(),
                digest: serde_json::from_value(body["digest"].clone()).unwrap_or_default(),
                google_sheet_url: body["google_sheet_url"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(String::from),
                instagram_sources: Vec::new(),
                archived: false,
                created_at: now.clone(),
                updated_at: now,
            };

            save_form(&kv, &form).await?;
            Response::ok("Created")
        }

        (Method::Put, p) if p.starts_with("/admin/forms/") => {
            let slug = p.strip_prefix("/admin/forms/").unwrap_or("");
            if slug.is_empty() {
                return Response::error("Slug required", 400);
            }

            let existing = match get_form(&kv, slug).await? {
                Some(f) => f,
                None => return Response::error("Form not found", 404),
            };

            let body: serde_json::Value = req.json().await?;

            let new_slug = body["slug"].as_str().unwrap_or(slug).to_string();

            if new_slug != slug {
                if !new_slug
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
                {
                    return Response::error("Invalid slug", 400);
                }
                if get_form(&kv, &new_slug).await?.is_some() {
                    return Response::error("Slug already exists", 409);
                }
                delete_form(&kv, slug).await?;
            }

            let form = FormConfig {
                slug: new_slug,
                name: body["name"].as_str().unwrap_or(&existing.name).to_string(),
                title: body["title"]
                    .as_str()
                    .unwrap_or(&existing.title)
                    .to_string(),
                submit_button_text: body["submit_button_text"]
                    .as_str()
                    .unwrap_or(&existing.submit_button_text)
                    .to_string(),
                success_message: body["success_message"]
                    .as_str()
                    .unwrap_or(&existing.success_message)
                    .to_string(),
                allowed_origins: body["allowed_origins"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or(existing.allowed_origins),
                fields: serde_json::from_value(body["fields"].clone()).unwrap_or(existing.fields),
                style: serde_json::from_value(body["style"].clone()).unwrap_or(existing.style),
                responders: serde_json::from_value(body["responders"].clone())
                    .unwrap_or(existing.responders),
                digest: serde_json::from_value(body["digest"].clone()).unwrap_or(existing.digest),
                google_sheet_url: if body.get("google_sheet_url").is_some() {
                    body["google_sheet_url"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                } else {
                    existing.google_sheet_url
                },
                instagram_sources: existing.instagram_sources,
                archived: existing.archived,
                created_at: existing.created_at,
                updated_at: now_iso(),
            };

            save_form(&kv, &form).await?;
            Response::ok("Updated")
        }

        (Method::Post, p) if p.ends_with("/archive") && p.starts_with("/admin/forms/") => {
            let slug = p
                .strip_prefix("/admin/forms/")
                .and_then(|s| s.strip_suffix("/archive"))
                .unwrap_or("");
            if slug.is_empty() {
                return Response::error("Slug required", 400);
            }
            let mut form = match get_form(&kv, slug).await? {
                Some(f) => f,
                None => return Response::error("Form not found", 404),
            };
            form.archived = true;
            form.updated_at = now_iso();
            save_form(&kv, &form).await?;
            Response::empty()
        }

        (Method::Post, p) if p.ends_with("/unarchive") && p.starts_with("/admin/forms/") => {
            let slug = p
                .strip_prefix("/admin/forms/")
                .and_then(|s| s.strip_suffix("/unarchive"))
                .unwrap_or("");
            if slug.is_empty() {
                return Response::error("Slug required", 400);
            }
            let mut form = match get_form(&kv, slug).await? {
                Some(f) => f,
                None => return Response::error("Form not found", 404),
            };
            form.archived = false;
            form.updated_at = now_iso();
            save_form(&kv, &form).await?;
            Response::empty()
        }

        (Method::Delete, p) if p.starts_with("/admin/forms/") => {
            let slug = p.strip_prefix("/admin/forms/").unwrap_or("");
            if slug.is_empty() {
                return Response::error("Slug required", 400);
            }
            delete_form(&kv, slug).await?;
            Response::ok("Deleted")
        }

        _ => Response::error("Not Found", 404),
    }
}

async fn handle_calendars_admin(
    mut req: Request,
    env: Env,
    path: &str,
    method: Method,
    base_url: &str,
) -> Result<Response> {
    let kv = env.kv("CALENDARS_KV")?;
    let db = env.d1("DB")?;

    let path_parts: Vec<&str> = path
        .strip_prefix("/admin/calendars")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    match (method, path_parts.as_slice()) {
        (Method::Post, []) => {
            let id = generate_id();
            let now = now_iso();
            let calendar = CalendarConfig {
                id: id.clone(),
                name: String::from("New Calendar"),
                description: None,
                timezone: String::from("UTC"),
                booking_links: Vec::new(),
                view_links: Vec::new(),
                feed_links: Vec::new(),
                instagram_sources: Vec::new(),
                style: CalendarStyle::default(),
                allowed_origins: Vec::new(),
                archived: false,
                created_at: now.clone(),
                updated_at: now,
            };
            save_calendar(&kv, &calendar).await?;

            Response::from_html(format!(
                r#"<tr>
                    <td><a href="{base_url}/admin/calendars/{id}">New Calendar</a></td>
                    <td>0 booking links</td>
                    <td>0 view links</td>
                    <td>{date}</td>
                    <td>
                        <a href="{base_url}/admin/calendars/{id}" class="btn btn-sm">Edit</a>
                        <button class="btn btn-sm btn-secondary"
                                hx-post="{base_url}/admin/calendars/{id}/archive"
                                hx-confirm="Archive this calendar? It will become read-only."
                                hx-target="closest tr"
                                hx-swap="outerHTML">Archive</button>
                    </td>
                </tr>"#,
                base_url = base_url,
                id = id,
                date = calendar.updated_at.split('T').next().unwrap_or(""),
            ))
        }

        (Method::Get, [id]) => {
            let calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };
            let time_slots = get_time_slots(&db, id).await?;
            Response::from_html(admin_calendar_html(&calendar, &time_slots, base_url))
        }

        (Method::Put, [id]) => {
            let mut calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };

            let form = req.form_data().await?;
            if let Some(FormEntry::Field(name)) = form.get("name") {
                calendar.name = name;
            }
            if let Some(FormEntry::Field(desc)) = form.get("description") {
                calendar.description = if desc.is_empty() { None } else { Some(desc) };
            }
            if let Some(FormEntry::Field(tz)) = form.get("timezone") {
                calendar.timezone = tz;
            }
            if let Some(FormEntry::Field(origins)) = form.get("allowed_origins") {
                calendar.allowed_origins = origins
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            if let Some(FormEntry::Field(css)) = form.get("custom_css") {
                calendar.style.custom_css = css;
            }
            calendar.updated_at = now_iso();

            save_calendar(&kv, &calendar).await?;
            Response::from_html(calendar_success_html("Calendar updated"))
        }

        (Method::Delete, [id]) => {
            delete_calendar(&kv, id).await?;
            Response::empty()
        }

        (Method::Post, [id, "archive"]) => {
            let mut calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };
            calendar.archived = true;
            calendar.updated_at = now_iso();
            save_calendar(&kv, &calendar).await?;
            Response::empty()
        }

        (Method::Post, [id, "unarchive"]) => {
            let mut calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };
            calendar.archived = false;
            calendar.updated_at = now_iso();
            save_calendar(&kv, &calendar).await?;
            Response::empty()
        }

        (Method::Get, [id, "events"]) => {
            let calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };
            let today = today_date();
            let end_date = add_days(&today, 365);
            let events = get_events(&db, id, &today, &end_date).await?;
            Response::from_html(admin_events_html(&calendar, &events, base_url))
        }

        (Method::Post, [id, "events"]) => {
            let form = req.form_data().await?;
            let title = match form.get("title") {
                Some(FormEntry::Field(t)) => t,
                _ => return Response::error("Title required", 400),
            };
            let start = match form.get("start_time") {
                Some(FormEntry::Field(t)) => t,
                _ => return Response::error("Start time required", 400),
            };
            let end = match form.get("end_time") {
                Some(FormEntry::Field(t)) => t,
                _ => return Response::error("End time required", 400),
            };
            let description = match form.get("description") {
                Some(FormEntry::Field(d)) if !d.is_empty() => Some(d),
                _ => None,
            };

            let now = now_iso();
            let event = CalendarEvent {
                id: generate_id(),
                calendar_id: id.to_string(),
                title,
                description,
                start_time: start.clone(),
                end_time: end.clone(),
                all_day: false,
                recurrence_rule: None,
                created_at: now.clone(),
                updated_at: now,
            };

            save_event(&db, &event).await?;

            Response::from_html(format!(
                r#"<tr>
                    <td>{title}</td>
                    <td>{start}</td>
                    <td>{end}</td>
                    <td>
                        <button class="btn btn-sm btn-danger"
                                hx-delete="{base_url}/admin/calendars/{cal_id}/events/{event_id}"
                                hx-target="closest tr"
                                hx-swap="outerHTML">Delete</button>
                    </td>
                </tr>"#,
                base_url = base_url,
                cal_id = id,
                event_id = event.id,
                title = html_escape(&event.title),
                start = html_escape(&event.start_time),
                end = html_escape(&event.end_time),
            ))
        }

        (Method::Delete, [_id, "events", event_id]) => {
            delete_event(&db, event_id).await?;
            Response::empty()
        }

        (Method::Get, [id, "slots"]) => {
            let calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };
            let slots = get_time_slots(&db, id).await?;
            Response::from_html(admin_slots_html(&calendar, &slots, base_url))
        }

        (Method::Post, [id, "slots"]) => {
            let form = req.form_data().await?;
            let day_of_week = match form.get("day_of_week") {
                Some(FormEntry::Field(d)) => d.parse().ok(),
                _ => None,
            };
            let start_time = match form.get("start_time") {
                Some(FormEntry::Field(t)) => t,
                _ => return Response::error("Start time required", 400),
            };
            let end_time = match form.get("end_time") {
                Some(FormEntry::Field(t)) => t,
                _ => return Response::error("End time required", 400),
            };
            let slot_duration = match form.get("slot_duration") {
                Some(FormEntry::Field(d)) => d.parse().unwrap_or(30),
                _ => 30,
            };
            let max_bookings = match form.get("max_bookings") {
                Some(FormEntry::Field(m)) => m.parse().unwrap_or(1),
                _ => 1,
            };

            let slot = TimeSlot {
                id: generate_id(),
                calendar_id: id.to_string(),
                day_of_week,
                specific_date: None,
                start_time: start_time.clone(),
                end_time: end_time.clone(),
                slot_duration,
                buffer_time: 0,
                max_bookings,
            };

            save_time_slot(&db, &slot).await?;

            let day_display = day_of_week
                .map(|d| day_name(d as u32).to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            Response::from_html(format!(
                r#"<tr>
                    <td>{day}</td>
                    <td>{start} - {end}</td>
                    <td>{duration} min</td>
                    <td>{max}</td>
                    <td>
                        <button class="btn btn-sm btn-danger"
                                hx-delete="{base_url}/admin/calendars/{cal_id}/slots/{slot_id}"
                                hx-target="closest tr"
                                hx-swap="outerHTML">Delete</button>
                    </td>
                </tr>"#,
                base_url = base_url,
                cal_id = id,
                slot_id = slot.id,
                day = html_escape(&day_display),
                start = html_escape(&slot.start_time),
                end = html_escape(&slot.end_time),
                duration = slot.slot_duration,
                max = slot.max_bookings,
            ))
        }

        (Method::Delete, [_id, "slots", slot_id]) => {
            delete_time_slot(&db, slot_id).await?;
            Response::empty()
        }

        (Method::Post, [id, "booking"]) => {
            let mut calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };

            let link = BookingLink {
                id: generate_id(),
                slug: generate_slug(),
                ..BookingLink::default()
            };

            calendar.booking_links.push(link.clone());
            calendar.updated_at = now_iso();
            save_calendar(&kv, &calendar).await?;

            Response::from_html(format!(
                r#"<tr>
                    <td>{name}</td>
                    <td><code>{base_url}/book/{cal_id}/{slug}</code></td>
                    <td>{duration} min</td>
                    <td>Enabled</td>
                    <td>
                        <a href="{base_url}/admin/calendars/{cal_id}/booking/{link_id}" class="btn btn-sm">Edit</a>
                        <button class="btn btn-sm btn-danger"
                                hx-delete="{base_url}/admin/calendars/{cal_id}/booking/{link_id}"
                                hx-target="closest tr"
                                hx-swap="outerHTML">Delete</button>
                    </td>
                </tr>"#,
                base_url = base_url,
                cal_id = id,
                link_id = link.id,
                slug = html_escape(&link.slug),
                name = html_escape(&link.name),
                duration = link.duration,
            ))
        }

        (Method::Get, [id, "booking", link_id]) => {
            let calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };
            let link = match calendar.booking_links.iter().find(|l| l.id == *link_id) {
                Some(l) => l,
                None => return Response::error("Booking link not found", 404),
            };

            // Check which responder channels are available
            let channels = crate::templates::AvailableChannels {
                twilio_sms: env.secret("TWILIO_SID").is_ok()
                    && env.secret("TWILIO_FROM_SMS").is_ok(),
                twilio_whatsapp: env.secret("TWILIO_SID").is_ok()
                    && env.secret("TWILIO_FROM_WHATSAPP").is_ok(),
                twilio_email: env.secret("SENDGRID_API_KEY").is_ok()
                    && env.secret("TWILIO_FROM_EMAIL").is_ok(),
                resend_email: env.secret("RESEND_API_KEY").is_ok()
                    && env.secret("RESEND_FROM").is_ok(),
            };

            Response::from_html(admin_booking_link_html(
                &calendar, link, base_url, &channels,
            ))
        }

        (Method::Put, [id, "booking", link_id]) => {
            let mut calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };

            let form = req.form_data().await?;

            if let Some(link) = calendar.booking_links.iter_mut().find(|l| l.id == *link_id) {
                if let Some(FormEntry::Field(name)) = form.get("name") {
                    link.name = name;
                }
                if let Some(FormEntry::Field(desc)) = form.get("description") {
                    link.description = if desc.is_empty() { None } else { Some(desc) };
                }
                if let Some(FormEntry::Field(duration)) = form.get("duration") {
                    link.duration = duration.parse().unwrap_or(30);
                }
                if let Some(FormEntry::Field(min_notice)) = form.get("min_notice") {
                    link.min_notice = min_notice.parse().unwrap_or(24);
                }
                if let Some(FormEntry::Field(max_advance)) = form.get("max_advance") {
                    link.max_advance = max_advance.parse().unwrap_or(30);
                }
                if let Some(FormEntry::Field(msg)) = form.get("confirmation_message") {
                    link.confirmation_message = msg;
                }
                link.enabled = form.get("enabled").is_some();
                link.auto_accept = form.get("auto_accept").is_some();
                link.hide_title = form.get("hide_title").is_some();
                if let Some(FormEntry::Field(responders_json)) = form.get("responders_json") {
                    if let Ok(responders) = serde_json::from_str(&responders_json) {
                        link.responders = responders;
                    }
                }
                if let Some(FormEntry::Field(admin_responders_json)) =
                    form.get("admin_responders_json")
                {
                    if let Ok(admin_responders) = serde_json::from_str(&admin_responders_json) {
                        link.admin_responders = admin_responders;
                    }
                }
            }

            calendar.updated_at = now_iso();
            save_calendar(&kv, &calendar).await?;
            Response::from_html(calendar_success_html("Booking link updated"))
        }

        (Method::Delete, [id, "booking", link_id]) => {
            let mut calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };
            calendar.booking_links.retain(|l| l.id != *link_id);
            calendar.updated_at = now_iso();
            save_calendar(&kv, &calendar).await?;
            Response::empty()
        }

        (Method::Post, [id, "view"]) => {
            let mut calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };

            let link = ViewLink {
                id: generate_id(),
                slug: generate_slug(),
                ..ViewLink::default()
            };

            calendar.view_links.push(link.clone());
            calendar.updated_at = now_iso();
            save_calendar(&kv, &calendar).await?;

            Response::from_html(format!(
                r#"<tr>
                    <td>{name}</td>
                    <td><code>{base_url}/view/{cal_id}/{slug}</code></td>
                    <td>{view_type:?}</td>
                    <td>Enabled</td>
                    <td>
                        <a href="{base_url}/admin/calendars/{cal_id}/view/{link_id}" class="btn btn-sm">Edit</a>
                        <button class="btn btn-sm btn-danger"
                                hx-delete="{base_url}/admin/calendars/{cal_id}/view/{link_id}"
                                hx-target="closest tr"
                                hx-swap="outerHTML">Delete</button>
                    </td>
                </tr>"#,
                base_url = base_url,
                cal_id = id,
                link_id = link.id,
                slug = html_escape(&link.slug),
                name = html_escape(&link.name),
                view_type = link.view_type,
            ))
        }

        (Method::Get, [id, "view", link_id]) => {
            let calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };
            let link = match calendar.view_links.iter().find(|l| l.id == *link_id) {
                Some(l) => l,
                None => return Response::error("View link not found", 404),
            };
            Response::from_html(admin_view_link_html(&calendar, link, base_url))
        }

        (Method::Put, [id, "view", link_id]) => {
            let mut calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };

            let form = req.form_data().await?;

            if let Some(link) = calendar.view_links.iter_mut().find(|l| l.id == *link_id) {
                if let Some(FormEntry::Field(name)) = form.get("name") {
                    link.name = name;
                }
                if let Some(FormEntry::Field(view_type)) = form.get("view_type") {
                    link.view_type = match view_type.as_str() {
                        "week" => ViewType::Week,
                        "month" => ViewType::Month,
                        "year" => ViewType::Year,
                        "endless" => ViewType::Endless,
                        _ => ViewType::Month,
                    };
                }
                link.show_events = form.get("show_events").is_some();
                link.show_event_details = form.get("show_event_details").is_some();
                link.show_bookings = form.get("show_bookings").is_some();
                link.show_booking_details = form.get("show_booking_details").is_some();
                link.enabled = form.get("enabled").is_some();
            }

            calendar.updated_at = now_iso();
            save_calendar(&kv, &calendar).await?;
            Response::from_html(calendar_success_html("View link updated"))
        }

        (Method::Delete, [id, "view", link_id]) => {
            let mut calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };
            calendar.view_links.retain(|l| l.id != *link_id);
            calendar.updated_at = now_iso();
            save_calendar(&kv, &calendar).await?;
            Response::empty()
        }

        (Method::Post, [id, "feed"]) => {
            let mut calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };

            let link = FeedLink {
                id: generate_id(),
                slug: generate_slug(),
                token: generate_token(),
                ..FeedLink::default()
            };

            calendar.feed_links.push(link.clone());
            calendar.updated_at = now_iso();
            save_calendar(&kv, &calendar).await?;

            Response::from_html(format!(
                r#"<tr>
                    <td>{name}</td>
                    <td><code>{base_url}/feed/{cal_id}/{slug}?token={token}</code></td>
                    <td>Enabled</td>
                    <td>
                        <button class="btn btn-sm btn-danger"
                                hx-delete="{base_url}/admin/calendars/{cal_id}/feed/{link_id}"
                                hx-target="closest tr"
                                hx-swap="outerHTML">Delete</button>
                    </td>
                </tr>"#,
                base_url = base_url,
                cal_id = id,
                link_id = link.id,
                slug = html_escape(&link.slug),
                name = html_escape(&link.name),
                token = html_escape(&link.token),
            ))
        }

        (Method::Delete, [id, "feed", link_id]) => {
            let mut calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };
            calendar.feed_links.retain(|l| l.id != *link_id);
            calendar.updated_at = now_iso();
            save_calendar(&kv, &calendar).await?;
            Response::empty()
        }

        (Method::Get, [id, "bookings"]) => {
            let calendar = match get_calendar(&kv, id).await? {
                Some(c) => c,
                None => return Response::error("Calendar not found", 404),
            };
            let today = today_date();
            let end_date = add_days(&today, 90);
            let bookings = get_bookings(&db, id, &today, &end_date).await?;
            Response::from_html(admin_bookings_html(&calendar, &bookings, base_url))
        }

        (Method::Post, [_id, "bookings", booking_id, "cancel"]) => {
            if let Some(mut booking) = get_booking(&db, booking_id).await? {
                booking.status = BookingStatus::Cancelled;
                booking.updated_at = now_iso();
                save_booking(&db, &booking).await?;
            }
            Response::from_html(r#"<tr><td colspan="6">Booking cancelled</td></tr>"#)
        }

        _ => Response::error("Not Found", 404),
    }
}
