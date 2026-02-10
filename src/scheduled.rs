use worker::*;

use crate::ai;
use crate::crypto;
use crate::helpers::{generate_id, now_iso, today_date};
use crate::instagram::{self, InstagramClient};
use crate::responders::{send_resend_email, send_twilio_email, send_twilio_message};
use crate::storage::*;
use crate::templates::{format_booking_digest, format_digest_email};
use crate::types::*;

pub async fn handle_scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    console_log!("Scheduled job started");

    // Run form digests
    if let Err(e) = send_form_digests(&env).await {
        console_log!("Form digest error: {:?}", e);
    }

    // Run booking digests
    if let Err(e) = send_booking_digests(&env).await {
        console_log!("Booking digest error: {:?}", e);
    }

    // Run Instagram sync for calendars
    if let Err(e) = sync_calendar_instagram_sources(&env).await {
        console_log!("Calendar Instagram sync error: {:?}", e);
    }

    // Run Instagram sync for forms
    if let Err(e) = sync_form_instagram_sources(&env).await {
        console_log!("Form Instagram sync error: {:?}", e);
    }

    console_log!("Scheduled job completed");
}

// ============================================================================
// Form Digests
// ============================================================================

async fn send_form_digests(env: &Env) -> Result<()> {
    let kv = env.kv("FORMS_KV")?;
    let db = env.d1("DB")?;
    let forms = list_forms(&kv).await?;

    let now = now_iso();
    let now_date = js_sys::Date::new_0();
    let day_of_week = now_date.get_utc_day(); // 0 = Sunday

    for mut form in forms {
        // Skip if digest not enabled
        if form.digest.frequency == DigestFrequency::None {
            continue;
        }

        // Skip if no responders configured
        if form.digest.responders.is_empty() {
            continue;
        }

        // Check frequency
        let should_send = match form.digest.frequency {
            DigestFrequency::Daily => true,
            DigestFrequency::Weekly => day_of_week == 1, // Monday
            DigestFrequency::None => false,
        };

        if !should_send {
            continue;
        }

        // Query new submissions since last digest
        let since = form
            .digest
            .last_sent_at
            .as_deref()
            .unwrap_or("1970-01-01T00:00:00Z");

        let submissions = get_submissions_since(&db, &form.slug, since).await?;

        // Skip if no new submissions
        if submissions.is_empty() {
            continue;
        }

        // Format the digest content
        let subject = format!(
            "Form Digest: {} - {} new response(s)",
            form.name,
            submissions.len()
        );
        let body = format_digest_email(&form, &submissions);

        // Send to all configured responders
        for responder in &form.digest.responders {
            if !responder.enabled {
                continue;
            }

            let target = &responder.target_field;
            if target.is_empty() {
                continue;
            }

            let result = match responder.channel {
                ResponderChannel::TwilioSms => {
                    if let Ok(from) = env.secret("TWILIO_FROM_SMS") {
                        send_twilio_message(env, target, &from.to_string(), &body).await
                    } else {
                        continue;
                    }
                }
                ResponderChannel::TwilioWhatsapp => {
                    if let Ok(from) = env.secret("TWILIO_FROM_WHATSAPP") {
                        send_twilio_message(
                            env,
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
                        send_twilio_email(env, target, &from.to_string(), &subject, &body).await
                    } else {
                        continue;
                    }
                }
                ResponderChannel::ResendEmail => {
                    if let Ok(from) = env.secret("RESEND_FROM") {
                        send_resend_email(env, target, &from.to_string(), &subject, &body).await
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            if let Err(e) = result {
                console_log!(
                    "Form digest responder error for {}: {:?}",
                    responder.name,
                    e
                );
            }
        }

        // Update last_sent_at
        form.digest.last_sent_at = Some(now.clone());
        if let Err(e) = save_form(&kv, &form).await {
            console_log!("Failed to update last_sent_at for {}: {:?}", form.slug, e);
        }

        console_log!(
            "Sent digest for {} with {} submissions",
            form.slug,
            submissions.len()
        );
    }

    Ok(())
}

// ============================================================================
// Booking Digests
// ============================================================================

async fn send_booking_digests(env: &Env) -> Result<()> {
    let kv = env.kv("CALENDARS_KV")?;
    let db = env.d1("DB")?;
    let calendars = list_calendars(&kv).await?;

    let now = now_iso();
    let now_date = js_sys::Date::new_0();
    let day_of_week = now_date.get_utc_day(); // 0 = Sunday

    for mut calendar in calendars {
        // Skip if digest not enabled
        if calendar.digest.frequency == DigestFrequency::None {
            continue;
        }

        // Skip if no responders configured
        if calendar.digest.responders.is_empty() {
            continue;
        }

        // Check frequency
        let should_send = match calendar.digest.frequency {
            DigestFrequency::Daily => true,
            DigestFrequency::Weekly => day_of_week == 1, // Monday
            DigestFrequency::None => false,
        };

        if !should_send {
            continue;
        }

        // Query new bookings since last digest
        let since = calendar
            .digest
            .last_sent_at
            .as_deref()
            .unwrap_or("1970-01-01T00:00:00Z");

        let bookings = get_bookings_since(&db, &calendar.id, since).await?;

        // Skip if no new bookings
        if bookings.is_empty() {
            continue;
        }

        // Format the digest content
        let subject = format!(
            "Booking Digest: {} - {} new booking(s)",
            calendar.name,
            bookings.len()
        );
        let body = format_booking_digest(&calendar, &bookings);

        // Send to all configured responders
        for responder in &calendar.digest.responders {
            if !responder.enabled {
                continue;
            }

            let target = &responder.target_field;
            if target.is_empty() {
                continue;
            }

            let result = match responder.channel {
                ResponderChannel::TwilioSms => {
                    if let Ok(from) = env.secret("TWILIO_FROM_SMS") {
                        send_twilio_message(env, target, &from.to_string(), &body).await
                    } else {
                        continue;
                    }
                }
                ResponderChannel::TwilioWhatsapp => {
                    if let Ok(from) = env.secret("TWILIO_FROM_WHATSAPP") {
                        send_twilio_message(
                            env,
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
                        send_twilio_email(env, target, &from.to_string(), &subject, &body).await
                    } else {
                        continue;
                    }
                }
                ResponderChannel::ResendEmail => {
                    if let Ok(from) = env.secret("RESEND_FROM") {
                        send_resend_email(env, target, &from.to_string(), &subject, &body).await
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            if let Err(e) = result {
                console_log!(
                    "Booking digest responder error for {}: {:?}",
                    responder.name,
                    e
                );
            }
        }

        // Update last_sent_at
        calendar.digest.last_sent_at = Some(now.clone());
        if let Err(e) = save_calendar(&kv, &calendar).await {
            console_log!(
                "Failed to update last_sent_at for {}: {:?}",
                calendar.id,
                e
            );
        }

        console_log!(
            "Sent booking digest for {} with {} bookings",
            calendar.name,
            bookings.len()
        );
    }

    Ok(())
}

// ============================================================================
// Instagram Sync for Calendars (Event Extraction)
// ============================================================================

async fn sync_calendar_instagram_sources(env: &Env) -> Result<()> {
    let kv = env.kv("CALENDARS_KV")?;
    let db = env.d1("DB")?;

    let encryption_key = env
        .secret("ENCRYPTION_KEY")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if encryption_key.is_empty() {
        console_log!("ENCRYPTION_KEY not configured, skipping Instagram sync");
        return Ok(());
    }

    let calendars = list_calendars(&kv).await?;

    for mut calendar in calendars {
        let calendar_id = calendar.id.clone();
        let timezone = calendar.timezone.clone();
        let mut synced_source_ids: Vec<String> = Vec::new();

        for source in &calendar.instagram_sources {
            if !source.enabled {
                continue;
            }

            let source_id = source.id.clone();
            console_log!(
                "Syncing Instagram source {} for calendar {}",
                source.instagram_username,
                calendar_id
            );

            // Get and validate token
            let token = match get_instagram_token(
                env,
                &kv,
                &calendar_id,
                &source_id,
                &encryption_key,
            )
            .await
            {
                Ok(Some(t)) => t,
                Ok(None) => continue,
                Err(e) => {
                    console_log!("Token error for {}: {:?}", source_id, e);
                    continue;
                }
            };

            // Fetch and process media for events
            let client = InstagramClient::new(token.access_token);
            let media = match client.get_recent_media().await {
                Ok(m) => m,
                Err(e) => {
                    console_log!("Failed to fetch media for {}: {:?}", source_id, e);
                    continue;
                }
            };

            let today = today_date();

            for post in media {
                let caption = match &post.caption {
                    Some(c) if !c.trim().is_empty() => c.clone(),
                    _ => continue,
                };

                if let Err(e) = process_calendar_instagram_post(
                    env,
                    &db,
                    &calendar_id,
                    &source_id,
                    &source.instagram_username,
                    &timezone,
                    &today,
                    &post.id,
                    &post.permalink,
                    &caption,
                )
                .await
                {
                    console_log!("Failed to process post {}: {:?}", post.id, e);
                }
            }

            synced_source_ids.push(source_id);
        }

        // Update last_synced_at for all synced sources
        for source_id in synced_source_ids {
            if let Some(source_mut) = calendar
                .instagram_sources
                .iter_mut()
                .find(|s| s.id == source_id)
            {
                source_mut.last_synced_at = Some(now_iso());
            }
        }

        // Save updated calendar
        calendar.updated_at = now_iso();
        save_calendar(&kv, &calendar).await?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_calendar_instagram_post(
    env: &Env,
    db: &D1Database,
    calendar_id: &str,
    source_id: &str,
    username: &str,
    timezone: &str,
    today: &str,
    post_id: &str,
    permalink: &str,
    caption: &str,
) -> Result<()> {
    let caption_hash = crypto::sha256_hex(caption);

    // Check if already processed
    let existing = get_instagram_post_by_post_id(db, post_id).await?;
    if let Some(ref existing_post) = existing {
        if existing_post.caption_hash == caption_hash {
            return Ok(()); // No changes
        }
        console_log!("Caption changed for post {}, re-processing", post_id);
    }

    // Extract event using AI
    let (extracted, ai_response) =
        match ai::extract_event_from_caption(env, caption, timezone, today).await {
            Ok(result) => result,
            Err(e) => {
                let failed_post = ProcessedPost {
                    id: existing
                        .as_ref()
                        .map(|p| p.id.clone())
                        .unwrap_or_else(generate_id),
                    calendar_id: Some(calendar_id.to_string()),
                    form_slug: None,
                    instagram_source_id: source_id.to_string(),
                    instagram_post_id: post_id.to_string(),
                    instagram_permalink: permalink.to_string(),
                    caption_hash,
                    event_id: None,
                    contact_id: None,
                    event_signature: None,
                    processing_status: ProcessingStatus::Failed,
                    ai_response: Some(format!("Error: {:?}", e)),
                    processed_at: now_iso(),
                    updated_at: Some(now_iso()),
                };
                save_instagram_post(db, &failed_post).await?;
                return Err(e);
            }
        };

    // Check if valid event
    if !ai::event_is_valid(&extracted) {
        let no_event_post = ProcessedPost {
            id: existing
                .as_ref()
                .map(|p| p.id.clone())
                .unwrap_or_else(generate_id),
            calendar_id: Some(calendar_id.to_string()),
            form_slug: None,
            instagram_source_id: source_id.to_string(),
            instagram_post_id: post_id.to_string(),
            instagram_permalink: permalink.to_string(),
            caption_hash,
            event_id: None,
            contact_id: None,
            event_signature: None,
            processing_status: ProcessingStatus::NoEvent,
            ai_response: Some(ai_response),
            processed_at: now_iso(),
            updated_at: Some(now_iso()),
        };
        save_instagram_post(db, &no_event_post).await?;
        return Ok(());
    }

    let event_signature = ai::generate_event_signature(&extracted);

    // Handle cancellation
    if extracted.is_cancellation {
        if let Some(ref sig) = event_signature {
            let existing_posts = get_instagram_posts_by_signature(db, calendar_id, sig).await?;
            for ep in existing_posts {
                if let Some(event_id) = ep.event_id {
                    console_log!("Cancelling event {} due to cancellation post", event_id);
                    delete_event(db, &event_id).await?;
                    delete_event_source_by_event_id(db, &event_id).await?;
                }
            }
        }

        let cancelled_post = ProcessedPost {
            id: existing
                .as_ref()
                .map(|p| p.id.clone())
                .unwrap_or_else(generate_id),
            calendar_id: Some(calendar_id.to_string()),
            form_slug: None,
            instagram_source_id: source_id.to_string(),
            instagram_post_id: post_id.to_string(),
            instagram_permalink: permalink.to_string(),
            caption_hash,
            event_id: None,
            contact_id: None,
            event_signature,
            processing_status: ProcessingStatus::Processed,
            ai_response: Some(ai_response),
            processed_at: now_iso(),
            updated_at: Some(now_iso()),
        };
        save_instagram_post(db, &cancelled_post).await?;
        return Ok(());
    }

    // Check for duplicate
    let mut is_duplicate = false;
    if let Some(ref sig) = event_signature {
        let existing_posts = get_instagram_posts_by_signature(db, calendar_id, sig).await?;
        for ep in &existing_posts {
            if ep.instagram_post_id != post_id && ep.event_id.is_some() {
                is_duplicate = true;
                break;
            }
        }
    }

    if is_duplicate {
        let dup_post = ProcessedPost {
            id: existing
                .as_ref()
                .map(|p| p.id.clone())
                .unwrap_or_else(generate_id),
            calendar_id: Some(calendar_id.to_string()),
            form_slug: None,
            instagram_source_id: source_id.to_string(),
            instagram_post_id: post_id.to_string(),
            instagram_permalink: permalink.to_string(),
            caption_hash,
            event_id: None,
            contact_id: None,
            event_signature,
            processing_status: ProcessingStatus::Processed,
            ai_response: Some(ai_response),
            processed_at: now_iso(),
            updated_at: Some(now_iso()),
        };
        save_instagram_post(db, &dup_post).await?;
        return Ok(());
    }

    // Create event
    let event_id = existing
        .as_ref()
        .and_then(|p| p.event_id.clone())
        .unwrap_or_else(generate_id);

    let date = extracted.date.as_deref().unwrap_or(today);
    let start_time = extracted.start_time.as_deref().unwrap_or("00:00");
    let end_time = extracted.end_time.as_deref().unwrap_or("23:59");

    let start_datetime = format!("{}T{}:00", date, start_time);
    let end_datetime = format!("{}T{}:00", date, end_time);

    let description = format!(
        "{}\n\nSource: Instagram (@{})\n{}",
        extracted.description.as_deref().unwrap_or(""),
        username,
        permalink
    );

    let event = CalendarEvent {
        id: event_id.clone(),
        calendar_id: calendar_id.to_string(),
        title: extracted
            .title
            .unwrap_or_else(|| "Instagram Event".to_string()),
        description: Some(description),
        start_time: start_datetime,
        end_time: end_datetime,
        all_day: extracted.start_time.is_none(),
        recurrence_rule: None,
        created_at: now_iso(),
        updated_at: now_iso(),
    };

    save_event(db, &event).await?;
    console_log!("Saved event {} from post {}", event_id, post_id);

    // Save event source
    if existing.is_none()
        || existing
            .as_ref()
            .and_then(|p| p.event_id.as_ref())
            .is_none()
    {
        let event_source = EventSource {
            id: generate_id(),
            event_id: Some(event_id.clone()),
            contact_id: None,
            source_type: "instagram".to_string(),
            source_id: source_id.to_string(),
            external_id: Some(post_id.to_string()),
            created_at: now_iso(),
        };
        save_event_source(db, &event_source).await?;
    }

    // Save processed post
    let processed_post = ProcessedPost {
        id: existing
            .as_ref()
            .map(|p| p.id.clone())
            .unwrap_or_else(generate_id),
        calendar_id: Some(calendar_id.to_string()),
        form_slug: None,
        instagram_source_id: source_id.to_string(),
        instagram_post_id: post_id.to_string(),
        instagram_permalink: permalink.to_string(),
        caption_hash,
        event_id: Some(event_id),
        contact_id: None,
        event_signature,
        processing_status: ProcessingStatus::Processed,
        ai_response: Some(ai_response),
        processed_at: now_iso(),
        updated_at: Some(now_iso()),
    };
    save_instagram_post(db, &processed_post).await?;

    Ok(())
}

// ============================================================================
// Instagram Sync for Forms (Contact Extraction)
// ============================================================================

async fn sync_form_instagram_sources(env: &Env) -> Result<()> {
    let forms_kv = env.kv("FORMS_KV")?;
    let calendars_kv = env.kv("CALENDARS_KV")?;
    let db = env.d1("DB")?;

    let encryption_key = env
        .secret("ENCRYPTION_KEY")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if encryption_key.is_empty() {
        return Ok(());
    }

    let forms = list_forms(&forms_kv).await?;

    for mut form in forms {
        if form.instagram_sources.is_empty() {
            continue;
        }

        let form_slug = form.slug.clone();
        let mut synced_source_ids: Vec<String> = Vec::new();

        for source in &form.instagram_sources {
            if !source.enabled {
                continue;
            }

            let source_id = source.id.clone();
            console_log!(
                "Syncing Instagram source {} for form {}",
                source.instagram_username,
                form_slug
            );

            // Try to get token (tokens stored with form_slug prefix)
            let token = match get_instagram_token(
                env,
                &calendars_kv,
                &form_slug,
                &source_id,
                &encryption_key,
            )
            .await
            {
                Ok(Some(t)) => t,
                Ok(None) => continue,
                Err(e) => {
                    console_log!("Token error for {}: {:?}", source_id, e);
                    continue;
                }
            };

            let client = InstagramClient::new(token.access_token);
            let media = match client.get_recent_media().await {
                Ok(m) => m,
                Err(e) => {
                    console_log!("Failed to fetch media for {}: {:?}", source_id, e);
                    continue;
                }
            };

            for post in media {
                let caption = match &post.caption {
                    Some(c) if !c.trim().is_empty() => c.clone(),
                    _ => continue,
                };

                if let Err(e) = process_form_instagram_post(
                    env,
                    &db,
                    &form_slug,
                    &source_id,
                    &form.fields,
                    &post.id,
                    &post.permalink,
                    &caption,
                )
                .await
                {
                    console_log!("Failed to process form post {}: {:?}", post.id, e);
                }
            }

            synced_source_ids.push(source_id);
        }

        // Update last_synced_at for all synced sources
        for source_id in synced_source_ids {
            if let Some(source_mut) = form
                .instagram_sources
                .iter_mut()
                .find(|s| s.id == source_id)
            {
                source_mut.last_synced_at = Some(now_iso());
            }
        }

        // Save updated form
        form.updated_at = now_iso();
        save_form(&forms_kv, &form).await?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_form_instagram_post(
    env: &Env,
    db: &D1Database,
    form_slug: &str,
    source_id: &str,
    form_fields: &[FormField],
    post_id: &str,
    permalink: &str,
    caption: &str,
) -> Result<()> {
    let caption_hash = crypto::sha256_hex(caption);

    // Check if already processed
    let existing = get_instagram_post_by_post_id(db, post_id).await?;
    if let Some(ref existing_post) = existing {
        if existing_post.caption_hash == caption_hash {
            return Ok(()); // No changes
        }
    }

    // Extract contact info using AI
    let (fields_data, ai_response) =
        match ai::extract_contact_from_caption(env, caption, form_fields).await {
            Ok(result) => result,
            Err(e) => {
                let failed_post = ProcessedPost {
                    id: existing
                        .as_ref()
                        .map(|p| p.id.clone())
                        .unwrap_or_else(generate_id),
                    calendar_id: None,
                    form_slug: Some(form_slug.to_string()),
                    instagram_source_id: source_id.to_string(),
                    instagram_post_id: post_id.to_string(),
                    instagram_permalink: permalink.to_string(),
                    caption_hash,
                    event_id: None,
                    contact_id: None,
                    event_signature: None,
                    processing_status: ProcessingStatus::Failed,
                    ai_response: Some(format!("Error: {:?}", e)),
                    processed_at: now_iso(),
                    updated_at: Some(now_iso()),
                };
                save_instagram_post(db, &failed_post).await?;
                return Err(e);
            }
        };

    // Skip if no contact info extracted
    if fields_data.is_empty() {
        let no_contact_post = ProcessedPost {
            id: existing
                .as_ref()
                .map(|p| p.id.clone())
                .unwrap_or_else(generate_id),
            calendar_id: None,
            form_slug: Some(form_slug.to_string()),
            instagram_source_id: source_id.to_string(),
            instagram_post_id: post_id.to_string(),
            instagram_permalink: permalink.to_string(),
            caption_hash,
            event_id: None,
            contact_id: None,
            event_signature: None,
            processing_status: ProcessingStatus::NoEvent,
            ai_response: Some(ai_response),
            processed_at: now_iso(),
            updated_at: Some(now_iso()),
        };
        save_instagram_post(db, &no_contact_post).await?;
        return Ok(());
    }

    // Add Instagram source metadata
    let mut enriched_data = fields_data;
    enriched_data.insert(
        "_instagram_post_id".to_string(),
        serde_json::Value::String(post_id.to_string()),
    );
    enriched_data.insert(
        "_instagram_permalink".to_string(),
        serde_json::Value::String(permalink.to_string()),
    );

    let fields_json = serde_json::to_string(&enriched_data).unwrap_or_else(|_| "{}".to_string());

    // Save as form submission
    let contact_id =
        save_submission_with_source(db, form_slug, &fields_json, "instagram", post_id).await?;
    console_log!(
        "Saved contact {} from Instagram post {}",
        contact_id,
        post_id
    );

    // Save processed post record
    let processed_post = ProcessedPost {
        id: existing
            .as_ref()
            .map(|p| p.id.clone())
            .unwrap_or_else(generate_id),
        calendar_id: None,
        form_slug: Some(form_slug.to_string()),
        instagram_source_id: source_id.to_string(),
        instagram_post_id: post_id.to_string(),
        instagram_permalink: permalink.to_string(),
        caption_hash,
        event_id: None,
        contact_id: Some(contact_id),
        event_signature: None,
        processing_status: ProcessingStatus::Processed,
        ai_response: Some(ai_response),
        processed_at: now_iso(),
        updated_at: Some(now_iso()),
    };
    save_instagram_post(db, &processed_post).await?;

    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

async fn get_instagram_token(
    _env: &Env,
    kv: &kv::KvStore,
    owner_id: &str,
    source_id: &str,
    encryption_key: &str,
) -> Result<Option<InstagramToken>> {
    let token_key = format!("instagram_token:{}:{}", owner_id, source_id);
    let encrypted_token = match kv.get(&token_key).text().await? {
        Some(t) => t,
        None => {
            console_log!("No token found for source {}", source_id);
            return Ok(None);
        }
    };

    let mut token = match crypto::decrypt_token(&encrypted_token, encryption_key).await {
        Ok(t) => t,
        Err(e) => {
            console_log!("Failed to decrypt token for {}: {:?}", source_id, e);
            return Ok(None);
        }
    };

    // Check if token is expired
    if instagram::token_is_expired(&token) {
        console_log!("Token expired for source {}", source_id);
        return Ok(None);
    }

    // Refresh if needed
    if instagram::token_needs_refresh(&token) {
        match instagram::refresh_token(&token.access_token).await {
            Ok(new_token) => {
                let encrypted = crypto::encrypt_token(&new_token, encryption_key).await?;
                kv.put(&token_key, encrypted)?.execute().await?;
                token = new_token;
                console_log!("Refreshed token for source {}", source_id);
            }
            Err(e) => {
                console_log!("Failed to refresh token for {}: {:?}", source_id, e);
                return Ok(None);
            }
        }
    }

    Ok(Some(token))
}
