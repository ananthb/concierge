//! Public booking form and confirmation templates

use crate::helpers::*;
use crate::types::*;

use super::base::{wrap_html, CssOptions};
use super::HASH;

#[allow(clippy::too_many_arguments)]
pub fn booking_form_html(
    calendar: &CalendarConfig,
    link: &BookingLink,
    available_slots: &[AvailableSlot],
    base_url: &str,
    css: &CssOptions,
    is_htmx: bool,
    _current_date: &str,
    prev_date: Option<String>,
    next_date: Option<String>,
    days: i32,
    hide_title: bool,
) -> String {
    let slots_by_date: std::collections::BTreeMap<String, Vec<&AvailableSlot>> = {
        let mut map = std::collections::BTreeMap::new();
        for slot in available_slots {
            map.entry(slot.date.clone())
                .or_insert_with(Vec::new)
                .push(slot);
        }
        map
    };

    let date_sections: String = slots_by_date
        .iter()
        .map(|(date, slots)| {
            let (year, month, day) = parse_date(date).unwrap_or((0, 0, 0));
            let dow = day_of_week(date).unwrap_or(0);
            let time_buttons: String = slots
                .iter()
                .filter(|s| s.available)
                .map(|slot| {
                    format!(
                        "<button type=\"button\" class=\"time-slot\" data-date=\"{date}\" data-time=\"{time}\"
                                   onclick=\"selectSlot(this, '{date}', '{time}')\">{display}</button>",
                        date = html_escape(&slot.date),
                        time = html_escape(&slot.time),
                        display = format_time(&slot.time),
                    )
                })
                .collect();

            if time_buttons.is_empty() {
                return String::new();
            }

            format!(
                "<div class=\"date-section\">
                    <h3>{dow}, {month} {day}, {year}</h3>
                    <div class=\"time-slots\">{time_buttons}</div>
                </div>",
                dow = day_name(dow),
                month = month_name(month),
                day = day,
                year = year,
                time_buttons = time_buttons,
            )
        })
        .collect();

    let fields_html: String = link
        .fields
        .iter()
        .map(|field| {
            let field_type = match field.field_type {
                FieldType::Text => "text",
                FieldType::Email => "email",
                FieldType::Phone => "tel",
                FieldType::Mobile => "tel",
                FieldType::LongText => "textarea",
                FieldType::File => "file",
            };
            let required = if field.required { "required" } else { "" };
            let placeholder = field.placeholder.as_deref().unwrap_or("");

            if field_type == "textarea" {
                format!(
                    "<div class=\"form-group\">
                        <label>{label}</label>
                        <textarea name=\"{id}\" placeholder=\"{placeholder}\" {required} rows=\"3\"></textarea>
                    </div>",
                    label = html_escape(&field.label),
                    id = html_escape(&field.id),
                    placeholder = html_escape(placeholder),
                    required = required,
                )
            } else {
                format!(
                    "<div class=\"form-group\">
                        <label>{label}</label>
                        <input type=\"{ftype}\" name=\"{id}\" placeholder=\"{placeholder}\" {required}>
                    </div>",
                    label = html_escape(&field.label),
                    id = html_escape(&field.id),
                    ftype = field_type,
                    placeholder = html_escape(placeholder),
                    required = required,
                )
            }
        })
        .collect();

    let content = format!(
        "<style>
            .booking-container {{ max-width: 600px; margin: 0 auto; }}
            .time-slots {{ display: flex; flex-wrap: wrap; gap: 0.5rem; margin: 1rem 0; }}
            .time-slot {{
                padding: 0.5rem 1rem;
                border: 1px solid {hash}ddd;
                border-radius: var(--cal-border-radius);
                background: white;
                cursor: pointer;
            }}
            .time-slot:hover {{ border-color: var(--cal-primary); }}
            .time-slot.selected {{
                background: var(--cal-primary);
                color: white;
                border-color: var(--cal-primary);
            }}
            .date-section {{ margin-bottom: 1.5rem; }}
            .date-section h3 {{ margin-bottom: 0.5rem; color: var(--cal-text); }}
            {hash}booking-details {{ display: none; }}
            {hash}booking-details.visible {{ display: block; }}
            .date-nav {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }}
            .date-nav h2 {{ margin: 0; }}
            .date-nav .btn {{ min-width: 100px; }}
            .date-nav .btn:disabled {{ opacity: 0.5; cursor: not-allowed; }}
        </style>

        <div class=\"booking-container\" id=\"booking-container\">
            {title_html}
            <p><strong>{duration} minutes</strong></p>

            <div id=\"slot-selection\">
                <div class=\"date-nav\">
                    {prev_btn}
                    <h2>Select a Time</h2>
                    {next_btn}
                </div>
                {slot_content}
            </div>

            <div id=\"booking-details\" class=\"card\" style=\"{details_style}\">
                <h2>Your Details</h2>
                <p id=\"selected-slot-display\"></p>
                <form id=\"booking-form\"
                      hx-post=\"{base_url}/book/{cal_id}/{slug}/submit\"
                      hx-target=\"{hash}booking-container\"
                      hx-swap=\"innerHTML\">
                    <input type=\"hidden\" name=\"date\" id=\"selected-date\">
                    <input type=\"hidden\" name=\"time\" id=\"selected-time\">
                    {fields_html}
                    <div class=\"form-group\">
                        <label>Notes (optional)</label>
                        <textarea name=\"notes\" rows=\"2\" placeholder=\"Any additional information...\"></textarea>
                    </div>
                    <button type=\"submit\" class=\"btn\">
                        Confirm Booking
                        <span class=\"htmx-indicator\"> ...</span>
                    </button>
                    <button type=\"button\" class=\"btn btn-secondary\" onclick=\"clearSelection()\">Back</button>
                </form>
            </div>
        </div>

        <script>
            function selectSlot(el, date, time) {{
                document.querySelectorAll('.time-slot').forEach(s => s.classList.remove('selected'));
                el.classList.add('selected');
                document.getElementById('selected-date').value = date;
                document.getElementById('selected-time').value = time;
                document.getElementById('selected-slot-display').textContent = date + ' at ' + el.textContent;
                document.getElementById('booking-details').classList.add('visible');
            }}
            function clearSelection() {{
                document.querySelectorAll('.time-slot').forEach(s => s.classList.remove('selected'));
                document.getElementById('booking-details').classList.remove('visible');
            }}
        </script>",
        base_url = base_url,
        cal_id = html_escape(&calendar.id),
        slug = html_escape(&link.slug),
        title_html = if hide_title {
            String::new()
        } else {
            format!(
                "<h1>{}</h1>{}",
                html_escape(&link.name),
                link.description.as_ref().map(|d| format!("<p>{}</p>", html_escape(d))).unwrap_or_default()
            )
        },
        duration = link.duration,
        slot_content = if date_sections.is_empty() {
            format!("<div class=\"card\" style=\"text-align: center; padding: 2rem;\">
                <p style=\"color: #666; font-size: 1.1rem;\">No time slots available for this week.</p>
                <p style=\"color: #999; margin-top: 0.5rem;\">{}</p>
            </div>",
                if next_date.is_some() { "Try checking other dates using the navigation above." }
                else { "Please check back later or contact us for assistance." }
            )
        } else {
            date_sections.clone()
        },
        details_style = if date_sections.is_empty() { "display: none;" } else { "" },
        fields_html = fields_html,
        prev_btn = build_nav_button(&prev_date, base_url, &calendar.id, &link.slug, css, days, true),
        next_btn = build_nav_button(&next_date, base_url, &calendar.id, &link.slug, css, days, false),
        hash = HASH,
    );

    wrap_html(&content, &link.name, &calendar.style, css, is_htmx)
}

pub fn build_nav_button(
    date: &Option<String>,
    base_url: &str,
    cal_id: &str,
    slug: &str,
    css: &CssOptions,
    days: i32,
    is_prev: bool,
) -> String {
    // Build query params
    let mut query_params = Vec::new();
    if days != 1 {
        query_params.push(format!("days={}", days));
    }
    if let Some(inline) = css.inline_css {
        query_params.push(format!("css={}", url_encode(inline)));
    }
    if let Some(url) = css.css_url {
        query_params.push(format!("css_url={}", url_encode(url)));
    }

    let (label, disabled_label) = if is_prev {
        ("&larr; Earlier", "&larr; Earlier")
    } else {
        ("Later &rarr;", "Later &rarr;")
    };

    match date {
        Some(d) => {
            query_params.insert(0, format!("date={}", d));
            let query = query_params.join("&");
            format!(
                "<button class=\"btn\" hx-get=\"{base_url}/book/{cal_id}/{slug}?{query}\" hx-target=\"#booking-container\" hx-swap=\"outerHTML\">{label}</button>",
                base_url = base_url,
                cal_id = html_escape(cal_id),
                slug = html_escape(slug),
                query = query,
                label = label,
            )
        }
        None => format!("<button class=\"btn\" disabled>{}</button>", disabled_label),
    }
}

pub fn booking_success_html(
    calendar: &CalendarConfig,
    booking: &Booking,
    link: &BookingLink,
    css: &CssOptions,
    is_htmx: bool,
) -> String {
    let content = format!(
        "<div class=\"success\" id=\"booking-container\">
            <h1>Booking Confirmed!</h1>
            <p>{message}</p>
            <div class=\"card\" style=\"margin-top: 1rem;\">
                <h2>Your Booking</h2>
                <p><strong>Date:</strong> {date}</p>
                <p><strong>Time:</strong> {time}</p>
                <p><strong>Duration:</strong> {duration} minutes</p>
            </div>
        </div>",
        message = html_escape(&link.confirmation_message),
        date = html_escape(&booking.slot_date),
        time = format_time(&booking.slot_time),
        duration = booking.duration,
    );

    wrap_html(&content, "Booking Confirmed", &calendar.style, css, is_htmx)
}

/// Display pending booking status (awaiting admin approval)
pub fn booking_pending_html(
    calendar: &CalendarConfig,
    booking: &Booking,
    link: &BookingLink,
    css: &CssOptions,
    is_htmx: bool,
) -> String {
    let content = format!(
        "<div id=\"booking-container\" style=\"text-align: center;\">
            <div class=\"card\" style=\"background: {hash}fff3cd; border-color: {hash}ffc107; padding: 2rem;\">
                <h1 style=\"color: {hash}856404;\">Booking Request Submitted</h1>
                <p style=\"color: {hash}856404; margin-top: 1rem;\">Your booking request has been received and is awaiting approval.</p>
                <p style=\"color: {hash}856404; margin-top: 0.5rem;\">You will receive a confirmation email once your booking is approved.</p>
            </div>
            <div class=\"card\" style=\"margin-top: 1rem;\">
                <h2>Requested Booking</h2>
                <p><strong>Event:</strong> {event_name}</p>
                <p><strong>Date:</strong> {date}</p>
                <p><strong>Time:</strong> {time}</p>
                <p><strong>Duration:</strong> {duration} minutes</p>
            </div>
        </div>",
        event_name = html_escape(&link.name),
        date = html_escape(&booking.slot_date),
        time = format_time(&booking.slot_time),
        duration = booking.duration,
        hash = HASH,
    );

    wrap_html(
        &content,
        "Booking Pending Approval",
        &calendar.style,
        css,
        is_htmx,
    )
}

/// Display approval success page for admin
pub fn approval_success_html(
    calendar: &CalendarConfig,
    booking: &Booking,
    css: &CssOptions,
    is_htmx: bool,
) -> String {
    let content = format!(
        "<div class=\"success\" id=\"booking-container\">
            <h1>Booking Approved!</h1>
            <p>The booking has been confirmed and the customer has been notified.</p>
            <div class=\"card\" style=\"margin-top: 1rem;\">
                <h2>Booking Details</h2>
                <p><strong>Name:</strong> {name}</p>
                <p><strong>Email:</strong> {email}</p>
                <p><strong>Date:</strong> {date}</p>
                <p><strong>Time:</strong> {time}</p>
                <p><strong>Duration:</strong> {duration} minutes</p>
            </div>
        </div>",
        name = html_escape(&booking.name),
        email = html_escape(&booking.email),
        date = html_escape(&booking.slot_date),
        time = format_time(&booking.slot_time),
        duration = booking.duration,
    );

    wrap_html(&content, "Booking Approved", &calendar.style, css, is_htmx)
}

/// Display denial success page
pub fn denial_success_html(
    calendar: &CalendarConfig,
    booking: &Booking,
    css: &CssOptions,
    is_htmx: bool,
) -> String {
    let content = format!(
        "<div class=\"success\" id=\"booking-container\" style=\"--cal-primary: #dc3545;\">
            <h1>Booking Denied</h1>
            <p>The booking has been denied and the customer has been notified.</p>
            <div class=\"card\" style=\"margin-top: 1rem;\">
                <h2>Booking Details</h2>
                <p><strong>Name:</strong> {name}</p>
                <p><strong>Email:</strong> {email}</p>
                <p><strong>Date:</strong> {date}</p>
                <p><strong>Time:</strong> {time}</p>
                <p><strong>Duration:</strong> {duration} minutes</p>
            </div>
        </div>",
        name = html_escape(&booking.name),
        email = html_escape(&booking.email),
        date = html_escape(&booking.slot_date),
        time = format_time(&booking.slot_time),
        duration = booking.duration,
    );

    wrap_html(&content, "Booking Denied", &calendar.style, css, is_htmx)
}

/// Display approval error page
pub fn approval_error_html(
    calendar: &CalendarConfig,
    message: &str,
    css: &CssOptions,
    is_htmx: bool,
) -> String {
    let content = format!(
        "<div class=\"error\" id=\"booking-container\">
            <h1>Approval Failed</h1>
            <p>{message}</p>
        </div>",
        message = html_escape(message),
    );

    wrap_html(&content, "Approval Error", &calendar.style, css, is_htmx)
}
