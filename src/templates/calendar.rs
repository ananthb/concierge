//! Calendar view and iCal feed templates

use crate::helpers::*;
use crate::types::*;

use super::base::{wrap_html, CssOptions};
use super::HASH;

#[allow(clippy::too_many_arguments)]
pub fn calendar_view_html(
    calendar: &CalendarConfig,
    link: &ViewLink,
    view_type: &ViewType,
    events: &[CalendarEvent],
    bookings: &[Booking],
    current_date: &str,
    base_url: &str,
    css: &CssOptions,
    is_htmx: bool,
    hide_title: bool,
) -> String {
    let (year, month, _) = parse_date(current_date).unwrap_or((2024, 1, 1));
    let month_start = start_of_month(current_date);
    let month_end = end_of_month(current_date);

    // Build calendar grid
    let first_dow = day_of_week(&month_start).unwrap_or(0);
    let mut current = add_days(&month_start, -(first_dow as i32));

    let mut weeks_html = String::new();
    for _ in 0..6 {
        let mut week_html = String::from("<tr>");
        for _ in 0..7 {
            let (_, cur_month, cur_day) = parse_date(&current).unwrap_or((0, 0, 0));
            let is_current_month = cur_month == month;
            let day_events: Vec<_> = events
                .iter()
                .filter(|e| e.start_time.starts_with(&current))
                .collect();
            let day_bookings: Vec<_> = bookings.iter().filter(|b| b.slot_date == current).collect();

            // Filter based on show_events and show_bookings settings
            let visible_events: Vec<_> = if link.show_events {
                day_events
            } else {
                Vec::new()
            };
            let visible_bookings: Vec<_> = if link.show_bookings {
                day_bookings
            } else {
                Vec::new()
            };

            // Build display for events based on show_event_details setting
            let events_str: String = if link.show_event_details {
                visible_events
                    .iter()
                    .map(|e| format!("<div class=\"event\">{}</div>", html_escape(&e.title)))
                    .collect()
            } else if !visible_events.is_empty() {
                format!(
                    "<div class=\"busy\">{} event{}</div>",
                    visible_events.len(),
                    if visible_events.len() == 1 { "" } else { "s" }
                )
            } else {
                String::new()
            };

            // Build display for bookings based on show_booking_details setting
            let bookings_str: String = if link.show_booking_details {
                visible_bookings
                    .iter()
                    .map(|b| {
                        format!(
                            "<div class=\"booking\">{} - {}</div>",
                            format_time(&b.slot_time),
                            html_escape(&b.name)
                        )
                    })
                    .collect()
            } else if !visible_bookings.is_empty() {
                format!(
                    "<div class=\"busy\">{} booking{}</div>",
                    visible_bookings.len(),
                    if visible_bookings.len() == 1 { "" } else { "s" }
                )
            } else {
                String::new()
            };

            let events_display = format!("{}{}", events_str, bookings_str);

            week_html.push_str(&format!(
                "<td class=\"{class}\">
                    <div class=\"day-number\">{day}</div>
                    {events}
                </td>",
                class = if is_current_month {
                    "current-month"
                } else {
                    "other-month"
                },
                day = cur_day,
                events = events_display,
            ));
            current = add_days(&current, 1);
        }
        week_html.push_str("</tr>");
        weeks_html.push_str(&week_html);
    }

    let prev_month = add_days(&month_start, -1);
    let next_month = add_days(&month_end, 1);

    // Build query string preserving CSS and view params
    let view_str = match view_type {
        ViewType::Week => "week",
        ViewType::Month => "month",
        ViewType::Year => "year",
        ViewType::Endless => "endless",
    };
    let mut extra_params = Vec::new();
    // Only include view param if it differs from link default
    if !matches!(
        (&link.view_type, view_type),
        (ViewType::Week, ViewType::Week)
            | (ViewType::Month, ViewType::Month)
            | (ViewType::Year, ViewType::Year)
            | (ViewType::Endless, ViewType::Endless)
    ) {
        extra_params.push(format!("view={}", view_str));
    }
    if let Some(inline) = css.inline_css {
        extra_params.push(format!("css={}", url_encode(inline)));
    }
    if let Some(url) = css.css_url {
        extra_params.push(format!("css_url={}", url_encode(url)));
    }
    let extra_query = if extra_params.is_empty() {
        String::new()
    } else {
        format!("&{}", extra_params.join("&"))
    };

    // View type selector options
    let view_selector = format!(
        "<select onchange=\"htmx.ajax('GET', '{base_url}/view/{cal_id}/{slug}?date={date}&view=' + this.value + '{css_query}', {{target: '#calendar-view', swap: 'outerHTML'}})\">
            <option value=\"week\" {week_sel}>Week</option>
            <option value=\"month\" {month_sel}>Month</option>
            <option value=\"year\" {year_sel}>Year</option>
            <option value=\"endless\" {endless_sel}>Endless</option>
        </select>",
        base_url = base_url,
        cal_id = html_escape(&calendar.id),
        slug = html_escape(&link.slug),
        date = html_escape(current_date),
        css_query = {
            let mut css_params = Vec::new();
            if let Some(inline) = css.inline_css {
                css_params.push(format!("css={}", url_encode(inline)));
            }
            if let Some(url) = css.css_url {
                css_params.push(format!("css_url={}", url_encode(url)));
            }
            if css_params.is_empty() { String::new() } else { format!("&{}", css_params.join("&")) }
        },
        week_sel = if matches!(view_type, ViewType::Week) { "selected" } else { "" },
        month_sel = if matches!(view_type, ViewType::Month) { "selected" } else { "" },
        year_sel = if matches!(view_type, ViewType::Year) { "selected" } else { "" },
        endless_sel = if matches!(view_type, ViewType::Endless) { "selected" } else { "" },
    );

    // Only show title on initial page load, not on HTMX navigation
    let title_html = if hide_title || is_htmx {
        String::new()
    } else {
        format!(
            "<h1>{}</h1>{}",
            html_escape(&link.name),
            calendar
                .description
                .as_ref()
                .map(|d| format!("<p style=\"margin-bottom: 1rem;\">{}</p>", html_escape(d)))
                .unwrap_or_default()
        )
    };

    let content = format!(
        "<style>
            .calendar {{ width: 100%; }}
            .calendar th {{ text-align: center; padding: 0.5rem; background: {hash}f8f9fa; }}
            .calendar td {{
                vertical-align: top;
                height: 100px;
                border: 1px solid {hash}ddd;
                padding: 0.25rem;
            }}
            .calendar .other-month {{ background: {hash}f8f9fa; color: {hash}999; }}
            .day-number {{ font-weight: bold; margin-bottom: 0.25rem; }}
            .event {{ font-size: 0.75rem; background: var(--cal-primary); color: white; padding: 0.125rem 0.25rem; border-radius: 2px; margin-bottom: 0.125rem; }}
            .booking {{ font-size: 0.75rem; background: {hash}28a745; color: white; padding: 0.125rem 0.25rem; border-radius: 2px; margin-bottom: 0.125rem; }}
            .busy {{ font-size: 0.75rem; color: {hash}666; }}
            .nav {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; gap: 0.5rem; }}
            .nav select {{ padding: 0.5rem; border-radius: var(--cal-border-radius); border: 1px solid {hash}ddd; }}
        </style>

        {title_html}
        <div id=\"calendar-view\">
            <div class=\"nav\">
                <button class=\"btn\"
                        hx-get=\"{base_url}/view/{cal_id}/{slug}?date={prev}{extra_query}\"
                        hx-target=\"{hash}calendar-view\"
                        hx-swap=\"outerHTML\">&larr; Previous</button>
                <div style=\"display: flex; align-items: center; gap: 0.5rem;\">
                    {view_selector}
                    <h2 style=\"margin: 0;\">{month} {year}</h2>
                </div>
                <button class=\"btn\"
                        hx-get=\"{base_url}/view/{cal_id}/{slug}?date={next}{extra_query}\"
                        hx-target=\"{hash}calendar-view\"
                        hx-swap=\"outerHTML\">Next &rarr;</button>
            </div>

            <table class=\"calendar\">
                <thead>
                    <tr>
                        <th>Sun</th><th>Mon</th><th>Tue</th><th>Wed</th><th>Thu</th><th>Fri</th><th>Sat</th>
                    </tr>
                </thead>
                <tbody>
                    {weeks}
                </tbody>
            </table>
        </div>",
        base_url = base_url,
        cal_id = html_escape(&calendar.id),
        slug = html_escape(&link.slug),
        month = month_name(month),
        year = year,
        prev = html_escape(&prev_month),
        next = html_escape(&next_month),
        extra_query = extra_query,
        view_selector = view_selector,
        weeks = weeks_html,
        title_html = title_html,
        hash = HASH,
    );

    wrap_html(&content, &link.name, &calendar.style, css, is_htmx)
}

// ============================================================================
// iCal Feed
// ============================================================================

pub fn ical_feed(
    calendar: &CalendarConfig,
    events: &[CalendarEvent],
    bookings: &[Booking],
) -> String {
    let mut ical = String::from(
        "BEGIN:VCALENDAR\r\n\
         VERSION:2.0\r\n\
         PRODID:-//Calendar Worker//EN\r\n\
         CALSCALE:GREGORIAN\r\n\
         METHOD:PUBLISH\r\n",
    );

    ical.push_str(&format!("X-WR-CALNAME:{}\r\n", calendar.name));
    ical.push_str(&format!("X-WR-TIMEZONE:{}\r\n", calendar.timezone));

    for event in events {
        ical.push_str("BEGIN:VEVENT\r\n");
        ical.push_str(&format!("UID:{}\r\n", event.id));
        ical.push_str(&format!("DTSTAMP:{}\r\n", ical_datetime(&event.created_at)));
        ical.push_str(&format!("DTSTART:{}\r\n", ical_datetime(&event.start_time)));
        ical.push_str(&format!("DTEND:{}\r\n", ical_datetime(&event.end_time)));
        ical.push_str(&format!("SUMMARY:{}\r\n", ical_escape(&event.title)));
        if let Some(desc) = &event.description {
            ical.push_str(&format!("DESCRIPTION:{}\r\n", ical_escape(desc)));
        }
        ical.push_str("END:VEVENT\r\n");
    }

    for booking in bookings {
        if booking.status != BookingStatus::Confirmed {
            continue;
        }
        ical.push_str("BEGIN:VEVENT\r\n");
        ical.push_str(&format!("UID:booking-{}\r\n", booking.id));
        ical.push_str(&format!(
            "DTSTAMP:{}\r\n",
            ical_datetime(&booking.created_at)
        ));
        let start = format!("{}T{}:00", booking.slot_date, booking.slot_time);
        let end_time = add_minutes(&booking.slot_time, booking.duration);
        let end = format!("{}T{}:00", booking.slot_date, end_time);
        ical.push_str(&format!("DTSTART:{}\r\n", ical_datetime(&start)));
        ical.push_str(&format!("DTEND:{}\r\n", ical_datetime(&end)));
        ical.push_str(&format!(
            "SUMMARY:Booking: {}\r\n",
            ical_escape(&booking.name)
        ));
        ical.push_str(&format!(
            "DESCRIPTION:Email: {}\r\n",
            ical_escape(&booking.email)
        ));
        ical.push_str("END:VEVENT\r\n");
    }

    ical.push_str("END:VCALENDAR\r\n");
    ical
}

fn ical_datetime(dt: &str) -> String {
    // Convert ISO to iCal format: 20240115T120000Z
    dt.replace(['-', ':', '.'], "")
        .chars()
        .take(15)
        .collect::<String>()
        + "Z"
}

fn ical_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace(';', "\\;")
        .replace('\n', "\\n")
}

// ============================================================================
// Calendar Error/Success helpers
// ============================================================================

pub fn calendar_error_html(message: &str) -> String {
    format!(
        "<div class=\"error\"><strong>Error:</strong> {}</div>",
        html_escape(message)
    )
}

pub fn calendar_success_html(message: &str) -> String {
    format!("<div class=\"success\">{}</div>", html_escape(message))
}
