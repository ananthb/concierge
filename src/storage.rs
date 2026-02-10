use wasm_bindgen::JsValue;
use worker::*;

use crate::types::{
    Booking, BookingStatus, CalendarConfig, CalendarEvent, EventSource, FormConfig, ProcessedPost,
    ProcessingStatus, Submission, TimeSlot,
};

// ============================================================================
// Form KV Operations (FORMS_KV)
// ============================================================================

pub async fn get_form(kv: &kv::KvStore, slug: &str) -> Result<Option<FormConfig>> {
    match kv
        .get(&format!("form:{}", slug))
        .json::<FormConfig>()
        .await?
    {
        Some(form) => Ok(Some(form)),
        None => Ok(None),
    }
}

pub async fn save_form(kv: &kv::KvStore, form: &FormConfig) -> Result<()> {
    kv.put(&format!("form:{}", form.slug), form)?
        .execute()
        .await?;
    Ok(())
}

pub async fn delete_form(kv: &kv::KvStore, slug: &str) -> Result<()> {
    kv.delete(&format!("form:{}", slug)).await?;
    Ok(())
}

pub async fn list_forms(kv: &kv::KvStore) -> Result<Vec<FormConfig>> {
    let list = kv.list().prefix("form:".into()).execute().await?;
    let mut forms = Vec::new();
    for key in list.keys {
        if let Some(form) = kv.get(&key.name).json::<FormConfig>().await? {
            forms.push(form);
        }
    }
    forms.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(forms)
}

// ============================================================================
// Calendar KV Operations (CALENDARS_KV)
// ============================================================================

pub async fn get_calendar(kv: &kv::KvStore, id: &str) -> Result<Option<CalendarConfig>> {
    kv.get(&format!("calendar:{}", id))
        .json::<CalendarConfig>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

pub async fn save_calendar(kv: &kv::KvStore, calendar: &CalendarConfig) -> Result<()> {
    kv.put(&format!("calendar:{}", calendar.id), calendar)?
        .execute()
        .await?;
    Ok(())
}

pub async fn delete_calendar(kv: &kv::KvStore, id: &str) -> Result<()> {
    kv.delete(&format!("calendar:{}", id)).await?;
    Ok(())
}

pub async fn list_calendars(kv: &kv::KvStore) -> Result<Vec<CalendarConfig>> {
    let list = kv
        .list()
        .prefix("calendar:".into())
        .execute()
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    let mut calendars = Vec::new();

    for key in list.keys {
        if let Some(calendar) = kv
            .get(&key.name)
            .json::<CalendarConfig>()
            .await
            .map_err(|e| Error::from(e.to_string()))?
        {
            calendars.push(calendar);
        }
    }

    calendars.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(calendars)
}

// ============================================================================
// D1 Operations (Contacts / Form Submissions)
// ============================================================================

pub async fn save_submission(
    db: &D1Database,
    form_slug: &str,
    fields_json: &str,
    attachments: Option<&str>,
) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO contacts (form_slug, fields_data, attachments, name, email, message) VALUES (?1, ?2, ?3, '', '', '')",
    );

    stmt.bind(&[
        form_slug.into(),
        fields_json.into(),
        attachments.unwrap_or("").into(),
    ])?
    .run()
    .await?;

    Ok(())
}

pub async fn save_submission_with_source(
    db: &D1Database,
    form_slug: &str,
    fields_json: &str,
    source_type: &str,
    source_id: &str,
) -> Result<i64> {
    let stmt = db.prepare(
        "INSERT INTO contacts (form_slug, fields_data, attachments, name, email, message, source_type, source_id)
         VALUES (?1, ?2, '', '', '', '', ?3, ?4) RETURNING id",
    );

    let results = stmt
        .bind(&[
            form_slug.into(),
            fields_json.into(),
            source_type.into(),
            source_id.into(),
        ])?
        .all()
        .await?;

    let rows = results.results::<serde_json::Value>()?;
    if let Some(row) = rows.into_iter().next() {
        Ok(row.get("id").and_then(|v| v.as_i64()).unwrap_or(0))
    } else {
        Ok(0)
    }
}

pub async fn get_submission_count(db: &D1Database, form_slug: &str) -> Result<i64> {
    let stmt = db.prepare("SELECT COUNT(*) as count FROM contacts WHERE form_slug = ?1");

    let result = stmt
        .bind(&[form_slug.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    if let Some(row) = result {
        Ok(row.get("count").and_then(|c| c.as_i64()).unwrap_or(0))
    } else {
        Ok(0)
    }
}

pub async fn get_submissions(
    db: &D1Database,
    form_slug: &str,
    limit: u32,
) -> Result<Vec<Submission>> {
    let stmt = db.prepare(
        "SELECT id, fields_data, created_at FROM contacts WHERE form_slug = ?1 ORDER BY created_at DESC LIMIT ?2",
    );

    let results = stmt.bind(&[form_slug.into(), limit.into()])?.all().await?;

    let mut submissions = Vec::new();
    for row in results.results::<serde_json::Value>()? {
        let id = row.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let created_at = row
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let fields_data: serde_json::Map<String, serde_json::Value> = row
            .get("fields_data")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        submissions.push(Submission {
            id,
            fields_data,
            created_at,
        });
    }

    Ok(submissions)
}

pub async fn get_submissions_since(
    db: &D1Database,
    form_slug: &str,
    since: &str,
) -> Result<Vec<Submission>> {
    let stmt = db.prepare(
        "SELECT id, fields_data, created_at FROM contacts WHERE form_slug = ?1 AND created_at > ?2 ORDER BY created_at DESC",
    );

    let results = stmt.bind(&[form_slug.into(), since.into()])?.all().await?;

    let mut submissions = Vec::new();
    for row in results.results::<serde_json::Value>()? {
        let id = row.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let created_at = row
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let fields_data: serde_json::Map<String, serde_json::Value> = row
            .get("fields_data")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        submissions.push(Submission {
            id,
            fields_data,
            created_at,
        });
    }

    Ok(submissions)
}

// ============================================================================
// D1 Operations (Events)
// ============================================================================

pub async fn get_events(
    db: &D1Database,
    calendar_id: &str,
    start: &str,
    end: &str,
) -> Result<Vec<CalendarEvent>> {
    let stmt = db.prepare(
        "SELECT id, calendar_id, title, description, start_time, end_time,
                all_day, recurrence_rule, created_at, updated_at
         FROM events
         WHERE calendar_id = ? AND start_time <= ? AND end_time >= ?
         ORDER BY start_time ASC",
    );

    let results = stmt
        .bind(&[calendar_id.into(), end.into(), start.into()])?
        .all()
        .await?;

    let mut events = Vec::new();
    for row in results.results::<serde_json::Value>()? {
        if let Ok(event) = serde_json::from_value::<CalendarEvent>(row) {
            events.push(event);
        }
    }
    Ok(events)
}

pub async fn save_event(db: &D1Database, event: &CalendarEvent) -> Result<()> {
    let stmt = db.prepare(
        "INSERT OR REPLACE INTO events
         (id, calendar_id, title, description, start_time, end_time, all_day, recurrence_rule, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    );

    stmt.bind(&[
        event.id.clone().into(),
        event.calendar_id.clone().into(),
        event.title.clone().into(),
        event.description.clone().unwrap_or_default().into(),
        event.start_time.clone().into(),
        event.end_time.clone().into(),
        (if event.all_day { 1 } else { 0 }).into(),
        event.recurrence_rule.clone().unwrap_or_default().into(),
        event.created_at.clone().into(),
        event.updated_at.clone().into(),
    ])?
    .run()
    .await?;

    Ok(())
}

pub async fn delete_event(db: &D1Database, id: &str) -> Result<()> {
    let stmt = db.prepare("DELETE FROM events WHERE id = ?");
    stmt.bind(&[id.into()])?.run().await?;
    Ok(())
}

// ============================================================================
// D1 Operations (Time Slots)
// ============================================================================

pub async fn get_time_slots(db: &D1Database, calendar_id: &str) -> Result<Vec<TimeSlot>> {
    let stmt = db.prepare(
        "SELECT id, calendar_id, day_of_week, specific_date, start_time, end_time,
                slot_duration, buffer_time, max_bookings
         FROM time_slots
         WHERE calendar_id = ?
         ORDER BY day_of_week ASC, start_time ASC",
    );

    let results = stmt.bind(&[calendar_id.into()])?.all().await?;

    let mut slots = Vec::new();
    for row in results.results::<serde_json::Value>()? {
        if let Ok(slot) = serde_json::from_value::<TimeSlot>(row) {
            slots.push(slot);
        }
    }
    Ok(slots)
}

pub async fn save_time_slot(db: &D1Database, slot: &TimeSlot) -> Result<()> {
    let stmt = db.prepare(
        "INSERT OR REPLACE INTO time_slots
         (id, calendar_id, day_of_week, specific_date, start_time, end_time, slot_duration, buffer_time, max_bookings, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))",
    );

    stmt.bind(&[
        slot.id.clone().into(),
        slot.calendar_id.clone().into(),
        slot.day_of_week.map(|d| d.into()).unwrap_or(JsValue::NULL),
        slot.specific_date
            .clone()
            .map(|s| s.into())
            .unwrap_or(JsValue::NULL),
        slot.start_time.clone().into(),
        slot.end_time.clone().into(),
        slot.slot_duration.into(),
        slot.buffer_time.into(),
        slot.max_bookings.into(),
    ])?
    .run()
    .await?;

    Ok(())
}

pub async fn delete_time_slot(db: &D1Database, id: &str) -> Result<()> {
    let stmt = db.prepare("DELETE FROM time_slots WHERE id = ?");
    stmt.bind(&[id.into()])?.run().await?;
    Ok(())
}

// ============================================================================
// D1 Operations (Bookings)
// ============================================================================

pub async fn get_bookings(
    db: &D1Database,
    calendar_id: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<Booking>> {
    let stmt = db.prepare(
        "SELECT id, calendar_id, booking_link_id, slot_date, slot_time, duration,
                name, email, phone, notes, fields_data, status, confirmation_token,
                created_at, updated_at
         FROM bookings
         WHERE calendar_id = ? AND slot_date >= ? AND slot_date <= ? AND status != 'cancelled'
         ORDER BY slot_date ASC, slot_time ASC",
    );

    let results = stmt
        .bind(&[calendar_id.into(), start_date.into(), end_date.into()])?
        .all()
        .await?;

    let mut bookings = Vec::new();
    for row in results.results::<serde_json::Value>()? {
        if let Ok(booking) = serde_json::from_value::<Booking>(row) {
            bookings.push(booking);
        }
    }
    Ok(bookings)
}

pub async fn get_booking(db: &D1Database, id: &str) -> Result<Option<Booking>> {
    let stmt = db.prepare(
        "SELECT id, calendar_id, booking_link_id, slot_date, slot_time, duration,
                name, email, phone, notes, fields_data, status, confirmation_token,
                created_at, updated_at
         FROM bookings WHERE id = ?",
    );

    let results = stmt.bind(&[id.into()])?.all().await?;
    let rows = results.results::<serde_json::Value>()?;

    if let Some(row) = rows.into_iter().next() {
        Ok(serde_json::from_value(row).ok())
    } else {
        Ok(None)
    }
}

pub async fn get_bookings_since(
    db: &D1Database,
    calendar_id: &str,
    since: &str,
) -> Result<Vec<Booking>> {
    let stmt = db.prepare(
        "SELECT id, calendar_id, booking_link_id, slot_date, slot_time, duration,
                name, email, phone, notes, fields_data, status, confirmation_token,
                created_at, updated_at
         FROM bookings
         WHERE calendar_id = ? AND created_at > ?
         ORDER BY created_at ASC",
    );

    let results = stmt
        .bind(&[calendar_id.into(), since.into()])?
        .all()
        .await?;

    let mut bookings = Vec::new();
    for row in results.results::<serde_json::Value>()? {
        if let Ok(booking) = serde_json::from_value::<Booking>(row) {
            bookings.push(booking);
        }
    }
    Ok(bookings)
}

pub async fn save_booking(db: &D1Database, booking: &Booking) -> Result<()> {
    let status_str = match booking.status {
        BookingStatus::Pending => "pending",
        BookingStatus::Confirmed => "confirmed",
        BookingStatus::Cancelled => "cancelled",
        BookingStatus::Completed => "completed",
    };

    let stmt = db.prepare(
        "INSERT OR REPLACE INTO bookings
         (id, calendar_id, booking_link_id, slot_date, slot_time, duration, name, email, phone, notes, fields_data, status, confirmation_token, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    );

    stmt.bind(&[
        booking.id.clone().into(),
        booking.calendar_id.clone().into(),
        booking.booking_link_id.clone().into(),
        booking.slot_date.clone().into(),
        booking.slot_time.clone().into(),
        booking.duration.into(),
        booking.name.clone().into(),
        booking.email.clone().into(),
        booking.phone.clone().unwrap_or_default().into(),
        booking.notes.clone().unwrap_or_default().into(),
        booking
            .fields_data
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default()
            .into(),
        status_str.into(),
        booking
            .confirmation_token
            .clone()
            .unwrap_or_default()
            .into(),
        booking.created_at.clone().into(),
        booking.updated_at.clone().into(),
    ])?
    .run()
    .await?;

    Ok(())
}

pub async fn count_bookings_for_slot(
    db: &D1Database,
    calendar_id: &str,
    date: &str,
    time: &str,
) -> Result<i32> {
    // Count both pending and confirmed bookings to prevent overbooking
    let stmt = db.prepare(
        "SELECT COUNT(*) as count FROM bookings
         WHERE calendar_id = ? AND slot_date = ? AND slot_time = ? AND status IN ('confirmed', 'pending')",
    );

    let results = stmt
        .bind(&[calendar_id.into(), date.into(), time.into()])?
        .all()
        .await?;

    let rows = results.results::<serde_json::Value>()?;
    if let Some(row) = rows.into_iter().next() {
        Ok(row.get("count").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
    } else {
        Ok(0)
    }
}

// ============================================================================
// D1 Operations (Instagram Posts)
// ============================================================================

pub async fn get_instagram_post_by_post_id(
    db: &D1Database,
    instagram_post_id: &str,
) -> Result<Option<ProcessedPost>> {
    let stmt = db.prepare(
        "SELECT id, calendar_id, form_slug, instagram_source_id, instagram_post_id, instagram_permalink,
                caption_hash, event_id, contact_id, event_signature, processing_status, ai_response,
                processed_at, updated_at
         FROM instagram_posts WHERE instagram_post_id = ?",
    );

    let results = stmt.bind(&[instagram_post_id.into()])?.all().await?;
    let rows = results.results::<serde_json::Value>()?;

    if let Some(row) = rows.into_iter().next() {
        Ok(serde_json::from_value(row).ok())
    } else {
        Ok(None)
    }
}

pub async fn get_instagram_posts_by_signature(
    db: &D1Database,
    calendar_id: &str,
    event_signature: &str,
) -> Result<Vec<ProcessedPost>> {
    let stmt = db.prepare(
        "SELECT id, calendar_id, form_slug, instagram_source_id, instagram_post_id, instagram_permalink,
                caption_hash, event_id, contact_id, event_signature, processing_status, ai_response,
                processed_at, updated_at
         FROM instagram_posts
         WHERE calendar_id = ? AND event_signature = ?",
    );

    let results = stmt
        .bind(&[calendar_id.into(), event_signature.into()])?
        .all()
        .await?;

    let mut posts = Vec::new();
    for row in results.results::<serde_json::Value>()? {
        if let Ok(post) = serde_json::from_value::<ProcessedPost>(row) {
            posts.push(post);
        }
    }
    Ok(posts)
}

pub async fn save_instagram_post(db: &D1Database, post: &ProcessedPost) -> Result<()> {
    let status_str = match post.processing_status {
        ProcessingStatus::Pending => "pending",
        ProcessingStatus::Processed => "processed",
        ProcessingStatus::NoEvent => "no_event",
        ProcessingStatus::Failed => "failed",
    };

    let stmt = db.prepare(
        "INSERT OR REPLACE INTO instagram_posts
         (id, calendar_id, form_slug, instagram_source_id, instagram_post_id, instagram_permalink,
          caption_hash, event_id, contact_id, event_signature, processing_status, ai_response,
          processed_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    );

    stmt.bind(&[
        post.id.clone().into(),
        post.calendar_id.clone().unwrap_or_default().into(),
        post.form_slug.clone().unwrap_or_default().into(),
        post.instagram_source_id.clone().into(),
        post.instagram_post_id.clone().into(),
        post.instagram_permalink.clone().into(),
        post.caption_hash.clone().into(),
        post.event_id.clone().unwrap_or_default().into(),
        post.contact_id.map(|id| id.into()).unwrap_or(JsValue::NULL),
        post.event_signature.clone().unwrap_or_default().into(),
        status_str.into(),
        post.ai_response.clone().unwrap_or_default().into(),
        post.processed_at.clone().into(),
        post.updated_at.clone().unwrap_or_default().into(),
    ])?
    .run()
    .await?;

    Ok(())
}

// ============================================================================
// D1 Operations (Event Sources)
// ============================================================================

pub async fn save_event_source(db: &D1Database, source: &EventSource) -> Result<()> {
    let stmt = db.prepare(
        "INSERT OR REPLACE INTO event_sources
         (id, event_id, contact_id, source_type, source_id, external_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    );

    stmt.bind(&[
        source.id.clone().into(),
        source.event_id.clone().unwrap_or_default().into(),
        source
            .contact_id
            .map(|id| id.into())
            .unwrap_or(JsValue::NULL),
        source.source_type.clone().into(),
        source.source_id.clone().into(),
        source.external_id.clone().unwrap_or_default().into(),
        source.created_at.clone().into(),
    ])?
    .run()
    .await?;

    Ok(())
}

pub async fn delete_event_source_by_event_id(db: &D1Database, event_id: &str) -> Result<()> {
    let stmt = db.prepare("DELETE FROM event_sources WHERE event_id = ?");
    stmt.bind(&[event_id.into()])?.run().await?;
    Ok(())
}
