//! Public booking handlers

use worker::*;

use super::{get_base_url, get_origin};
use crate::ai;
use crate::helpers::*;
use crate::responders::{send_resend_email, send_twilio_email, send_twilio_message};
use crate::storage::*;
use crate::templates::*;
use crate::types::*;

/// Handle public booking routes (/book/*)
pub async fn handle_booking(
    mut req: Request,
    env: Env,
    path: &str,
    method: Method,
) -> Result<Response> {
    let base_url = get_base_url(&req);
    let origin = get_origin(&req);
    let kv = env.kv("CALENDARS_KV")?;
    let db = env.d1("DB")?;

    let path_parts: Vec<&str> = path
        .strip_prefix("/book/")
        .unwrap_or("")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let calendar_id = path_parts.first().copied().unwrap_or("");
    let slug = path_parts.get(1).copied().unwrap_or("");
    let action = path_parts.get(2).unwrap_or(&"");
    let booking_id = path_parts.get(3).unwrap_or(&"");

    // Handle CORS preflight early - try to load calendar for allowed_origins
    if method == Method::Options {
        if !calendar_id.is_empty() {
            if let Ok(Some(calendar)) = get_calendar(&kv, calendar_id).await {
                return cors_preflight(origin.as_deref(), &calendar.allowed_origins);
            }
        }
        // Calendar not found or invalid path - return permissive CORS for preflight
        return cors_preflight(origin.as_deref(), &[]);
    }

    if calendar_id.is_empty() {
        return Response::error("Calendar ID required", 400);
    }
    if slug.is_empty() {
        return Response::error("Booking link required", 400);
    }

    let calendar = match get_calendar(&kv, calendar_id).await? {
        Some(c) => c,
        None => return Response::error("Calendar not found", 404),
    };

    let link = match calendar
        .booking_links
        .iter()
        .find(|l| l.slug == slug && l.enabled)
    {
        Some(l) => l.clone(),
        None => return Response::error("Booking link not found", 404),
    };

    let url = req.url()?;
    let query_pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let inline_css = query_pairs.get("css").map(|s| s.to_string());
    let css_url = query_pairs.get("css_url").map(|s| s.to_string());
    let css_options = CssOptions {
        inline_css: inline_css.as_deref(),
        css_url: css_url.as_deref(),
    };
    let hide_title = query_pairs
        .get("notitle")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);

    match (method, *action) {
        (Method::Get, "") => {
            let days_to_show: i32 = query_pairs
                .get("days")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1)
                .clamp(1, 30);

            let today = today_date();
            let earliest_date = add_days(&today, (link.min_notice / 24).max(1));
            let latest_date = add_days(&today, link.max_advance);

            let selected_date = query_pairs
                .get("date")
                .map(|s| s.to_string())
                .unwrap_or_else(|| earliest_date.clone());

            let view_start = if selected_date < earliest_date {
                earliest_date.clone()
            } else if selected_date > latest_date {
                latest_date.clone()
            } else {
                selected_date
            };

            let view_end_candidate = add_days(&view_start, days_to_show - 1);
            let view_end = if view_end_candidate > latest_date {
                latest_date.clone()
            } else {
                view_end_candidate
            };

            let slots = get_time_slots(&db, calendar_id).await?;
            let existing_bookings = get_bookings(&db, calendar_id, &view_start, &view_end).await?;

            let mut available_slots = Vec::new();
            let mut current_date = view_start.clone();

            while current_date <= view_end {
                let dow = day_of_week(&current_date).unwrap_or(0) as i32;

                for slot in &slots {
                    let applies = slot.day_of_week.map(|d| d == dow).unwrap_or(false)
                        || slot
                            .specific_date
                            .as_ref()
                            .map(|d| d == &current_date)
                            .unwrap_or(false);

                    if !applies {
                        continue;
                    }

                    let mut time = slot.start_time.clone();
                    while time_to_minutes(&time) + link.duration <= time_to_minutes(&slot.end_time)
                    {
                        let booking_count = existing_bookings
                            .iter()
                            .filter(|b| b.slot_date == current_date && b.slot_time == time)
                            .count() as i32;

                        available_slots.push(AvailableSlot {
                            date: current_date.clone(),
                            time: time.clone(),
                            end_time: add_minutes(&time, link.duration),
                            available: booking_count < slot.max_bookings,
                        });

                        time = add_minutes(&time, slot.slot_duration + slot.buffer_time);
                    }
                }

                current_date = add_days(&current_date, 1);
            }

            let prev_date = add_days(&view_start, -days_to_show);
            let next_date = add_days(&view_start, days_to_show);
            let has_prev = prev_date >= earliest_date || view_start > earliest_date;
            let has_next = next_date <= latest_date;

            let is_htmx = is_htmx_request(&req);
            let html = booking_form_html(
                &calendar,
                &link,
                &available_slots,
                &base_url,
                &css_options,
                is_htmx,
                &view_start,
                has_prev.then(|| {
                    if prev_date < earliest_date {
                        earliest_date.clone()
                    } else {
                        prev_date
                    }
                }),
                has_next.then_some(next_date),
                days_to_show,
                hide_title,
            );
            let response = Response::from_html(html)?;
            Ok(with_cors(
                response,
                origin.as_deref(),
                &calendar.allowed_origins,
            ))
        }

        (Method::Post, "submit") => {
            let form = req.form_data().await?;

            let date = match form.get("date") {
                Some(FormEntry::Field(d)) => d,
                _ => {
                    return Response::from_html(calendar_error_html("Please select a date"))
                        .map(|r| with_cors(r, origin.as_deref(), &calendar.allowed_origins))
                }
            };
            let time = match form.get("time") {
                Some(FormEntry::Field(t)) => t,
                _ => {
                    return Response::from_html(calendar_error_html("Please select a time"))
                        .map(|r| with_cors(r, origin.as_deref(), &calendar.allowed_origins))
                }
            };

            let slot_count = count_bookings_for_slot(&db, calendar_id, &date, &time).await?;
            let slots = get_time_slots(&db, calendar_id).await?;
            let dow = day_of_week(&date).unwrap_or(0) as i32;
            let max_bookings = slots
                .iter()
                .filter(|s| {
                    s.day_of_week.map(|d| d == dow).unwrap_or(false)
                        || s.specific_date
                            .as_ref()
                            .map(|d| d == &date)
                            .unwrap_or(false)
                })
                .map(|s| s.max_bookings)
                .max()
                .unwrap_or(1);

            if slot_count >= max_bookings {
                return Response::from_html(calendar_error_html(
                    "This slot is no longer available",
                ))
                .map(|r| with_cors(r, origin.as_deref(), &calendar.allowed_origins));
            }

            let name = match form.get("name") {
                Some(FormEntry::Field(n)) if !n.is_empty() => n,
                _ => {
                    return Response::from_html(calendar_error_html("Name is required"))
                        .map(|r| with_cors(r, origin.as_deref(), &calendar.allowed_origins))
                }
            };
            let email = match form.get("email") {
                Some(FormEntry::Field(e)) if !e.is_empty() => e,
                _ => {
                    return Response::from_html(calendar_error_html("Email is required"))
                        .map(|r| with_cors(r, origin.as_deref(), &calendar.allowed_origins))
                }
            };
            let phone = match form.get("phone") {
                Some(FormEntry::Field(p)) if !p.is_empty() => Some(p),
                _ => None,
            };
            let notes = match form.get("notes") {
                Some(FormEntry::Field(n)) if !n.is_empty() => Some(n),
                _ => None,
            };

            let mut fields_data = serde_json::Map::new();
            for field in &link.fields {
                if let Some(FormEntry::Field(value)) = form.get(&field.id) {
                    fields_data.insert(field.id.clone(), serde_json::Value::String(value));
                }
            }

            // Add booking info to fields_data for responders
            fields_data.insert("name".to_string(), serde_json::Value::String(name.clone()));
            fields_data.insert(
                "email".to_string(),
                serde_json::Value::String(email.clone()),
            );
            fields_data.insert("date".to_string(), serde_json::Value::String(date.clone()));
            fields_data.insert("time".to_string(), serde_json::Value::String(time.clone()));

            // Determine initial status based on auto_accept setting
            let initial_status = if link.auto_accept {
                BookingStatus::Confirmed
            } else {
                BookingStatus::Pending
            };

            let now = now_iso();
            let booking = Booking {
                id: generate_id(),
                calendar_id: calendar_id.to_string(),
                booking_link_id: link.id.clone(),
                slot_date: date,
                slot_time: time,
                duration: link.duration,
                name,
                email,
                phone,
                notes,
                fields_data: Some(serde_json::Value::Object(fields_data.clone())),
                status: initial_status.clone(),
                confirmation_token: Some(generate_token()),
                created_at: now.clone(),
                updated_at: now,
            };

            save_booking(&db, &booking).await?;

            let is_htmx = is_htmx_request(&req);

            if link.auto_accept {
                // Trigger customer responders immediately
                trigger_customer_responders(&env, &link, &fields_data).await;

                // Show confirmation
                let html = booking_success_html(&calendar, &booking, &link, &css_options, is_htmx);
                let response = Response::from_html(html)?;
                Ok(with_cors(
                    response,
                    origin.as_deref(),
                    &calendar.allowed_origins,
                ))
            } else {
                // Send admin approval notification
                send_admin_approval_notification(&env, &link, &calendar, &booking, &base_url).await;

                // Show "pending" message
                let html = booking_pending_html(&calendar, &booking, &link, &css_options, is_htmx);
                let response = Response::from_html(html)?;
                Ok(with_cors(
                    response,
                    origin.as_deref(),
                    &calendar.allowed_origins,
                ))
            }
        }

        // Approval endpoint: POST /book/{cal_id}/{slug}/approve/{booking_id}?token={confirmation_token}
        (Method::Post, "approve") if !booking_id.is_empty() => {
            let token = query_pairs.get("token").map(|s| s.to_string());

            // Get the booking
            let mut booking = match get_booking(&db, booking_id).await? {
                Some(b) => b,
                None => {
                    let html =
                        approval_error_html(&calendar, "Booking not found", &css_options, false);
                    return Response::from_html(html);
                }
            };

            // Verify token matches
            let expected_token = booking.confirmation_token.as_deref().unwrap_or("");
            let provided_token = token.as_deref().unwrap_or("");
            if expected_token.is_empty() || expected_token != provided_token {
                let html =
                    approval_error_html(&calendar, "Invalid approval token", &css_options, false);
                return Response::from_html(html);
            }

            // Verify booking is still pending
            if booking.status != BookingStatus::Pending {
                let html = approval_error_html(
                    &calendar,
                    &format!("Booking is already {:?}", booking.status),
                    &css_options,
                    false,
                );
                return Response::from_html(html);
            }

            // Update status to Confirmed
            booking.status = BookingStatus::Confirmed;
            booking.updated_at = now_iso();
            save_booking(&db, &booking).await?;

            // Build fields_data from booking for responders
            let mut fields_data = match &booking.fields_data {
                Some(serde_json::Value::Object(map)) => map.clone(),
                _ => serde_json::Map::new(),
            };
            fields_data.insert(
                "name".to_string(),
                serde_json::Value::String(booking.name.clone()),
            );
            fields_data.insert(
                "email".to_string(),
                serde_json::Value::String(booking.email.clone()),
            );
            fields_data.insert(
                "date".to_string(),
                serde_json::Value::String(booking.slot_date.clone()),
            );
            fields_data.insert(
                "time".to_string(),
                serde_json::Value::String(booking.slot_time.clone()),
            );

            // Trigger customer responders now that booking is confirmed
            trigger_customer_responders(&env, &link, &fields_data).await;

            // Return approval success HTML
            let html = approval_success_html(&calendar, &booking, &css_options, false);
            Response::from_html(html)
        }

        // Denial endpoint: POST /book/{cal_id}/{slug}/deny/{booking_id}?token={confirmation_token}
        (Method::Post, "deny") if !booking_id.is_empty() => {
            let token = query_pairs.get("token").map(|s| s.to_string());

            // Get the booking
            let mut booking = match get_booking(&db, booking_id).await? {
                Some(b) => b,
                None => {
                    let html =
                        approval_error_html(&calendar, "Booking not found", &css_options, false);
                    return Response::from_html(html);
                }
            };

            // Verify token matches
            let expected_token = booking.confirmation_token.as_deref().unwrap_or("");
            let provided_token = token.as_deref().unwrap_or("");
            if expected_token.is_empty() || expected_token != provided_token {
                let html = approval_error_html(&calendar, "Invalid token", &css_options, false);
                return Response::from_html(html);
            }

            // Verify booking is still pending
            if booking.status != BookingStatus::Pending {
                let html = approval_error_html(
                    &calendar,
                    &format!("Booking is already {:?}", booking.status),
                    &css_options,
                    false,
                );
                return Response::from_html(html);
            }

            // Update status to Cancelled
            booking.status = BookingStatus::Cancelled;
            booking.updated_at = now_iso();
            save_booking(&db, &booking).await?;

            // Build fields_data from booking for responders
            let mut fields_data = match &booking.fields_data {
                Some(serde_json::Value::Object(map)) => map.clone(),
                _ => serde_json::Map::new(),
            };
            fields_data.insert(
                "name".to_string(),
                serde_json::Value::String(booking.name.clone()),
            );
            fields_data.insert(
                "email".to_string(),
                serde_json::Value::String(booking.email.clone()),
            );
            fields_data.insert(
                "date".to_string(),
                serde_json::Value::String(booking.slot_date.clone()),
            );
            fields_data.insert(
                "time".to_string(),
                serde_json::Value::String(booking.slot_time.clone()),
            );
            fields_data.insert(
                "status".to_string(),
                serde_json::Value::String("denied".to_string()),
            );

            // Trigger denial responders
            trigger_denial_responders(&env, &link, &fields_data).await;

            // Return denial success HTML
            let html = denial_success_html(&calendar, &booking, &css_options, false);
            Response::from_html(html)
        }

        _ => Response::error("Not Found", 404),
    }
}

/// Trigger customer responders (send confirmation emails/messages)
async fn trigger_customer_responders(
    env: &Env,
    link: &BookingLink,
    fields_data: &serde_json::Map<String, serde_json::Value>,
) {
    for responder in &link.responders {
        if !responder.enabled {
            continue;
        }

        let target = if matches!(responder.channel, ResponderChannel::MetaWhatsapp) {
            continue; // WhatsApp only for incoming messages
        } else {
            match fields_data.get(&responder.target_field) {
                Some(serde_json::Value::String(t)) => t.clone(),
                _ => continue,
            }
        };

        let body = if responder.use_ai {
            match ai::generate_response(env, &responder.body, fields_data).await {
                Ok(response) => response,
                Err(e) => {
                    console_log!("AI generation error: {:?}", e);
                    interpolate_template(&responder.body, fields_data)
                }
            }
        } else {
            interpolate_template(&responder.body, fields_data)
        };

        let result = match responder.channel {
            ResponderChannel::TwilioSms => {
                if let Ok(from) = env.secret("TWILIO_FROM_SMS") {
                    send_twilio_message(env, &target, &from.to_string(), &body).await
                } else {
                    continue;
                }
            }
            ResponderChannel::TwilioEmail => {
                if let Ok(from) = env.secret("TWILIO_FROM_EMAIL") {
                    let subject = interpolate_template(&responder.subject, fields_data);
                    send_twilio_email(env, &target, &from.to_string(), &subject, &body).await
                } else {
                    continue;
                }
            }
            ResponderChannel::ResendEmail => {
                if let Ok(from) = env.secret("RESEND_FROM") {
                    let subject = interpolate_template(&responder.subject, fields_data);
                    send_resend_email(env, &target, &from.to_string(), &subject, &body).await
                } else {
                    continue;
                }
            }
            _ => continue,
        };

        if let Err(e) = result {
            console_log!("Booking responder error for {}: {:?}", responder.name, e);
        }
    }
}

/// Trigger denial responders (notify customer that booking was denied)
async fn trigger_denial_responders(
    env: &Env,
    link: &BookingLink,
    fields_data: &serde_json::Map<String, serde_json::Value>,
) {
    // Use the same responders but with a denial message
    for responder in &link.responders {
        if !responder.enabled {
            continue;
        }

        let target = if matches!(responder.channel, ResponderChannel::MetaWhatsapp) {
            continue;
        } else {
            match fields_data.get(&responder.target_field) {
                Some(serde_json::Value::String(t)) => t.clone(),
                _ => {
                    // For email responders, try the email field directly
                    match fields_data.get("email") {
                        Some(serde_json::Value::String(t)) => t.clone(),
                        _ => continue,
                    }
                }
            }
        };

        // Create a denial message based on the responder type
        let name = fields_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Guest");
        let date = fields_data
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("the requested date");
        let time = fields_data
            .get("time")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let body = format!(
            "Hi {},\n\nUnfortunately, your booking request for {} at {} could not be approved at this time.\n\nPlease contact us for more information or to reschedule.\n\nThank you for your understanding.",
            name, date, format_time(time)
        );

        let result = match responder.channel {
            ResponderChannel::TwilioSms => {
                if let Ok(from) = env.secret("TWILIO_FROM_SMS") {
                    send_twilio_message(env, &target, &from.to_string(), &body).await
                } else {
                    continue;
                }
            }
            ResponderChannel::TwilioEmail => {
                if let Ok(from) = env.secret("TWILIO_FROM_EMAIL") {
                    let subject = format!("Booking Request Update for {}", link.name);
                    send_twilio_email(env, &target, &from.to_string(), &subject, &body).await
                } else {
                    continue;
                }
            }
            ResponderChannel::ResendEmail => {
                if let Ok(from) = env.secret("RESEND_FROM") {
                    let subject = format!("Booking Request Update for {}", link.name);
                    send_resend_email(env, &target, &from.to_string(), &subject, &body).await
                } else {
                    continue;
                }
            }
            _ => continue,
        };

        if let Err(e) = result {
            console_log!(
                "Booking denial responder error for {}: {:?}",
                responder.name,
                e
            );
        }
    }
}

/// Send approval notification email to admin
async fn send_admin_approval_notification(
    env: &Env,
    link: &BookingLink,
    calendar: &CalendarConfig,
    booking: &Booking,
    base_url: &str,
) {
    let admin_email = match &link.admin_email {
        Some(email) if !email.is_empty() => email.clone(),
        _ => {
            console_log!("No admin email configured for booking link: {}", link.name);
            return;
        }
    };

    let token = booking.confirmation_token.as_deref().unwrap_or("");
    let approval_url = format!(
        "{}/book/{}/{}/approve/{}?token={}",
        base_url, calendar.id, link.slug, booking.id, token
    );
    let denial_url = format!(
        "{}/book/{}/{}/deny/{}?token={}",
        base_url, calendar.id, link.slug, booking.id, token
    );

    let subject = format!(
        "Booking Approval Required: {} on {}",
        booking.name, booking.slot_date
    );
    let body = format!(
        "A new booking request requires your approval.\n\n\
         Booking Details:\n\
         - Name: {}\n\
         - Email: {}\n\
         - Date: {}\n\
         - Time: {}\n\
         - Duration: {} minutes\n\
         - Event: {}\n\n\
         To APPROVE this booking:\n\
         {}\n\n\
         To DENY this booking:\n\
         {}\n\n\
         If you take no action, the booking will remain pending.",
        booking.name,
        booking.email,
        booking.slot_date,
        format_time(&booking.slot_time),
        booking.duration,
        link.name,
        approval_url,
        denial_url
    );

    // Try Resend first, then Twilio
    let result = if let Ok(from) = env.secret("RESEND_FROM") {
        send_resend_email(env, &admin_email, &from.to_string(), &subject, &body).await
    } else if let Ok(from) = env.secret("TWILIO_FROM_EMAIL") {
        send_twilio_email(env, &admin_email, &from.to_string(), &subject, &body).await
    } else {
        console_log!("No email provider configured for admin approval notifications");
        return;
    };

    if let Err(e) = result {
        console_log!("Failed to send admin approval notification: {:?}", e);
    }
}
