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
    // Build CSS query params for navigation
    let css_query = {
        let mut params = Vec::new();
        if let Some(inline) = css.inline_css {
            params.push(format!("css={}", url_encode(inline)));
        }
        if let Some(url) = css.css_url {
            params.push(format!("css_url={}", url_encode(url)));
        }
        if params.is_empty() {
            String::new()
        } else {
            format!("&{}", params.join("&"))
        }
    };

    // View type selector
    let view_str = match view_type {
        ViewType::Week => "week",
        ViewType::Month => "month",
        ViewType::Year => "year",
        ViewType::Endless => "endless",
    };
    let view_selector = format!(
        "<select onchange=\"htmx.ajax('GET', '{base_url}/view/{cal_id}/{slug}?date={date}&view=' + this.value + '{css_query}', {{target: '#calendar-view', swap: 'outerHTML'}})\">
            <option value=\"week\" {week_sel}>Week</option>
            <option value=\"month\" {month_sel}>Month</option>
            <option value=\"year\" {year_sel}>Year</option>
            <option value=\"endless\" {endless_sel}>List</option>
        </select>",
        base_url = base_url,
        cal_id = html_escape(&calendar.id),
        slug = html_escape(&link.slug),
        date = html_escape(current_date),
        css_query = css_query,
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

    // Render the appropriate view based on view_type
    let view_content = match view_type {
        ViewType::Week => render_week_view(calendar, link, events, bookings, current_date, base_url, &css_query, view_str, &view_selector),
        ViewType::Month => render_month_view(calendar, link, events, bookings, current_date, base_url, &css_query, view_str, &view_selector),
        ViewType::Year => render_year_view(calendar, link, events, bookings, current_date, base_url, &css_query, view_str, &view_selector),
        ViewType::Endless => render_list_view(calendar, link, events, bookings, current_date, base_url, &css_query, view_str, &view_selector),
    };

    let content = format!("{}{}", title_html, view_content);
    wrap_html(&content, &link.name, &calendar.style, css, is_htmx)
}

#[allow(clippy::too_many_arguments)]
fn render_week_view(
    calendar: &CalendarConfig,
    link: &ViewLink,
    events: &[CalendarEvent],
    bookings: &[Booking],
    current_date: &str,
    base_url: &str,
    css_query: &str,
    view_str: &str,
    view_selector: &str,
) -> String {
    let week_start = start_of_week(current_date);
    let (year, month, _) = parse_date(current_date).unwrap_or((2024, 1, 1));

    let mut days_html = String::new();
    for i in 0..7 {
        let day = add_days(&week_start, i);
        let (_, _, day_num) = parse_date(&day).unwrap_or((0, 0, 0));
        let dow = day_of_week(&day).unwrap_or(0);

        let day_events: Vec<_> = events.iter().filter(|e| e.start_time.starts_with(&day)).collect();
        let day_bookings: Vec<_> = bookings.iter().filter(|b| b.slot_date == day).collect();

        let events_html = render_day_items(link, &day_events, &day_bookings, true);

        days_html.push_str(&format!(
            "<div class=\"week-day\">
                <div class=\"week-day-header\">{dow} {day}</div>
                <div class=\"week-day-content\">{events}</div>
            </div>",
            dow = day_name(dow),
            day = day_num,
            events = events_html,
        ));
    }

    let prev_week = add_days(&week_start, -7);
    let next_week = add_days(&week_start, 7);

    format!(
        "<style>
            .week-day {{ border: 1px solid {hash}ddd; margin-bottom: 0.5rem; border-radius: var(--cal-border-radius); }}
            .week-day-header {{ background: {hash}f8f9fa; padding: 0.5rem; font-weight: bold; }}
            .week-day-content {{ padding: 0.5rem; min-height: 60px; }}
            .event {{ font-size: 0.875rem; background: var(--cal-primary); color: white; padding: 0.25rem 0.5rem; border-radius: 4px; margin-bottom: 0.25rem; }}
            .booking {{ font-size: 0.875rem; background: {hash}28a745; color: white; padding: 0.25rem 0.5rem; border-radius: 4px; margin-bottom: 0.25rem; }}
            .busy {{ font-size: 0.875rem; color: {hash}666; }}
            .nav {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; gap: 0.5rem; }}
            .nav select {{ padding: 0.5rem; border-radius: var(--cal-border-radius); border: 1px solid {hash}ddd; }}
        </style>
        <div id=\"calendar-view\">
            <div class=\"nav\">
                <button class=\"btn\" hx-get=\"{base_url}/view/{cal_id}/{slug}?date={prev}&view={view}{css_query}\" hx-target=\"{hash}calendar-view\" hx-swap=\"outerHTML\">&larr; Previous</button>
                <div style=\"display: flex; align-items: center; gap: 0.5rem;\">
                    {view_selector}
                    <h2 style=\"margin: 0;\">{month} {year}</h2>
                </div>
                <button class=\"btn\" hx-get=\"{base_url}/view/{cal_id}/{slug}?date={next}&view={view}{css_query}\" hx-target=\"{hash}calendar-view\" hx-swap=\"outerHTML\">Next &rarr;</button>
            </div>
            {days}
        </div>",
        base_url = base_url,
        cal_id = html_escape(&calendar.id),
        slug = html_escape(&link.slug),
        prev = html_escape(&prev_week),
        next = html_escape(&next_week),
        view = view_str,
        css_query = css_query,
        view_selector = view_selector,
        month = month_name(month),
        year = year,
        days = days_html,
        hash = HASH,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_month_view(
    calendar: &CalendarConfig,
    link: &ViewLink,
    events: &[CalendarEvent],
    bookings: &[Booking],
    current_date: &str,
    base_url: &str,
    css_query: &str,
    view_str: &str,
    view_selector: &str,
) -> String {
    let (year, month, _) = parse_date(current_date).unwrap_or((2024, 1, 1));
    let month_start = start_of_month(current_date);
    let month_end = end_of_month(current_date);

    let first_dow = day_of_week(&month_start).unwrap_or(0);
    let mut current = add_days(&month_start, -(first_dow as i32));

    let mut weeks_html = String::new();
    for _ in 0..6 {
        let mut week_html = String::from("<tr>");
        for _ in 0..7 {
            let (_, cur_month, cur_day) = parse_date(&current).unwrap_or((0, 0, 0));
            let is_current_month = cur_month == month;
            let day_events: Vec<_> = events.iter().filter(|e| e.start_time.starts_with(&current)).collect();
            let day_bookings: Vec<_> = bookings.iter().filter(|b| b.slot_date == current).collect();

            let events_display = render_day_items(link, &day_events, &day_bookings, false);

            week_html.push_str(&format!(
                "<td class=\"{class}\"><div class=\"day-number\">{day}</div>{events}</td>",
                class = if is_current_month { "current-month" } else { "other-month" },
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

    format!(
        "<style>
            .calendar {{ width: 100%; border-collapse: collapse; }}
            .calendar th {{ text-align: center; padding: 0.5rem; background: {hash}f8f9fa; }}
            .calendar td {{ vertical-align: top; height: 100px; border: 1px solid {hash}ddd; padding: 0.25rem; }}
            .calendar .other-month {{ background: {hash}f8f9fa; color: {hash}999; }}
            .day-number {{ font-weight: bold; margin-bottom: 0.25rem; }}
            .event {{ font-size: 0.75rem; background: var(--cal-primary); color: white; padding: 0.125rem 0.25rem; border-radius: 2px; margin-bottom: 0.125rem; }}
            .booking {{ font-size: 0.75rem; background: {hash}28a745; color: white; padding: 0.125rem 0.25rem; border-radius: 2px; margin-bottom: 0.125rem; }}
            .busy {{ font-size: 0.75rem; color: {hash}666; }}
            .nav {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; gap: 0.5rem; }}
            .nav select {{ padding: 0.5rem; border-radius: var(--cal-border-radius); border: 1px solid {hash}ddd; }}
        </style>
        <div id=\"calendar-view\">
            <div class=\"nav\">
                <button class=\"btn\" hx-get=\"{base_url}/view/{cal_id}/{slug}?date={prev}&view={view}{css_query}\" hx-target=\"{hash}calendar-view\" hx-swap=\"outerHTML\">&larr; Previous</button>
                <div style=\"display: flex; align-items: center; gap: 0.5rem;\">
                    {view_selector}
                    <h2 style=\"margin: 0;\">{month} {year}</h2>
                </div>
                <button class=\"btn\" hx-get=\"{base_url}/view/{cal_id}/{slug}?date={next}&view={view}{css_query}\" hx-target=\"{hash}calendar-view\" hx-swap=\"outerHTML\">Next &rarr;</button>
            </div>
            <table class=\"calendar\">
                <thead><tr><th>Sun</th><th>Mon</th><th>Tue</th><th>Wed</th><th>Thu</th><th>Fri</th><th>Sat</th></tr></thead>
                <tbody>{weeks}</tbody>
            </table>
        </div>",
        base_url = base_url,
        cal_id = html_escape(&calendar.id),
        slug = html_escape(&link.slug),
        month = month_name(month),
        year = year,
        prev = html_escape(&prev_month),
        next = html_escape(&next_month),
        view = view_str,
        css_query = css_query,
        view_selector = view_selector,
        weeks = weeks_html,
        hash = HASH,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_year_view(
    calendar: &CalendarConfig,
    link: &ViewLink,
    events: &[CalendarEvent],
    bookings: &[Booking],
    current_date: &str,
    base_url: &str,
    css_query: &str,
    view_str: &str,
    view_selector: &str,
) -> String {
    let (year, _, _) = parse_date(current_date).unwrap_or((2024, 1, 1));

    let mut months_html = String::new();
    for m in 1..=12 {
        let month_date = format!("{:04}-{:02}-01", year, m);
        let month_start = start_of_month(&month_date);
        let first_dow = day_of_week(&month_start).unwrap_or(0);
        let mut current = add_days(&month_start, -(first_dow as i32));

        let mut mini_weeks = String::new();
        for _ in 0..6 {
            let mut week = String::from("<tr>");
            for _ in 0..7 {
                let (_, cur_month, cur_day) = parse_date(&current).unwrap_or((0, 0, 0));
                let is_current_month = cur_month == m;

                let has_event = link.show_events && events.iter().any(|e| e.start_time.starts_with(&current));
                let has_booking = link.show_bookings && bookings.iter().any(|b| b.slot_date == current);

                let class = if !is_current_month {
                    "other"
                } else if has_event || has_booking {
                    "has-item"
                } else {
                    ""
                };

                week.push_str(&format!("<td class=\"{class}\">{day}</td>", class = class, day = if is_current_month { cur_day.to_string() } else { String::new() }));
                current = add_days(&current, 1);
            }
            week.push_str("</tr>");
            mini_weeks.push_str(&week);
        }

        months_html.push_str(&format!(
            "<div class=\"mini-month\">
                <h4>{month}</h4>
                <table><thead><tr><th>S</th><th>M</th><th>T</th><th>W</th><th>T</th><th>F</th><th>S</th></tr></thead><tbody>{weeks}</tbody></table>
            </div>",
            month = month_name(m),
            weeks = mini_weeks,
        ));
    }

    let prev_year = format!("{:04}-01-01", year - 1);
    let next_year = format!("{:04}-01-01", year + 1);

    format!(
        "<style>
            .year-grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 1rem; }}
            .mini-month {{ border: 1px solid {hash}ddd; border-radius: var(--cal-border-radius); padding: 0.5rem; }}
            .mini-month h4 {{ margin: 0 0 0.5rem 0; text-align: center; }}
            .mini-month table {{ width: 100%; font-size: 0.75rem; border-collapse: collapse; }}
            .mini-month th, .mini-month td {{ text-align: center; padding: 2px; }}
            .mini-month .other {{ color: {hash}ccc; }}
            .mini-month .has-item {{ background: var(--cal-primary); color: white; border-radius: 50%; }}
            .nav {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; gap: 0.5rem; }}
            .nav select {{ padding: 0.5rem; border-radius: var(--cal-border-radius); border: 1px solid {hash}ddd; }}
        </style>
        <div id=\"calendar-view\">
            <div class=\"nav\">
                <button class=\"btn\" hx-get=\"{base_url}/view/{cal_id}/{slug}?date={prev}&view={view}{css_query}\" hx-target=\"{hash}calendar-view\" hx-swap=\"outerHTML\">&larr; {prev_year}</button>
                <div style=\"display: flex; align-items: center; gap: 0.5rem;\">
                    {view_selector}
                    <h2 style=\"margin: 0;\">{year}</h2>
                </div>
                <button class=\"btn\" hx-get=\"{base_url}/view/{cal_id}/{slug}?date={next}&view={view}{css_query}\" hx-target=\"{hash}calendar-view\" hx-swap=\"outerHTML\">{next_year} &rarr;</button>
            </div>
            <div class=\"year-grid\">{months}</div>
        </div>",
        base_url = base_url,
        cal_id = html_escape(&calendar.id),
        slug = html_escape(&link.slug),
        year = year,
        prev = html_escape(&prev_year),
        next = html_escape(&next_year),
        prev_year = year - 1,
        next_year = year + 1,
        view = view_str,
        css_query = css_query,
        view_selector = view_selector,
        months = months_html,
        hash = HASH,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_list_view(
    calendar: &CalendarConfig,
    link: &ViewLink,
    events: &[CalendarEvent],
    bookings: &[Booking],
    current_date: &str,
    base_url: &str,
    css_query: &str,
    view_str: &str,
    view_selector: &str,
) -> String {
    let (year, month, _) = parse_date(current_date).unwrap_or((2024, 1, 1));
    let month_start = start_of_month(current_date);
    let month_end = end_of_month(current_date);

    // Collect all items in date order
    let mut items: Vec<(String, String, String)> = Vec::new(); // (date, time, html)

    if link.show_events {
        for event in events {
            if event.start_time >= month_start && event.start_time <= format!("{}T23:59:59", month_end) {
                let time = event.start_time.get(11..16).unwrap_or("").to_string();
                let html = if link.show_event_details {
                    format!(
                        "<div class=\"list-item event\"><strong>{}</strong>{}</div>",
                        html_escape(&event.title),
                        event.description.as_ref().map(|d| format!("<br><small>{}</small>", html_escape(d))).unwrap_or_default()
                    )
                } else {
                    "<div class=\"list-item event\">Event</div>".to_string()
                };
                items.push((event.start_time.clone(), time, html));
            }
        }
    }

    if link.show_bookings {
        for booking in bookings {
            if booking.slot_date >= month_start && booking.slot_date <= month_end {
                let html = if link.show_booking_details {
                    format!(
                        "<div class=\"list-item booking\"><strong>{}</strong> - {}</div>",
                        format_time(&booking.slot_time),
                        html_escape(&booking.name)
                    )
                } else {
                    format!("<div class=\"list-item booking\">{} - Booking</div>", format_time(&booking.slot_time))
                };
                items.push((booking.slot_date.clone(), booking.slot_time.clone(), html));
            }
        }
    }

    items.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    // Group by date
    let mut list_html = String::new();
    let mut last_date = String::new();
    for (date, _, html) in &items {
        if date != &last_date {
            if !last_date.is_empty() {
                list_html.push_str("</div>");
            }
            let (y, m, d) = parse_date(date).unwrap_or((0, 0, 0));
            let dow = day_of_week(date).unwrap_or(0);
            list_html.push_str(&format!(
                "<div class=\"list-date\"><h3>{}, {} {}, {}</h3>",
                day_name(dow), month_name(m), d, y
            ));
            last_date = date.clone();
        }
        list_html.push_str(html);
    }
    if !last_date.is_empty() {
        list_html.push_str("</div>");
    }

    if list_html.is_empty() {
        list_html = "<p style=\"text-align: center; color: #666;\">No events or bookings this month.</p>".to_string();
    }

    let prev_month = add_days(&month_start, -1);
    let next_month = add_days(&month_end, 1);

    format!(
        "<style>
            .list-date {{ margin-bottom: 1rem; }}
            .list-date h3 {{ margin: 0 0 0.5rem 0; padding-bottom: 0.25rem; border-bottom: 1px solid {hash}ddd; }}
            .list-item {{ padding: 0.5rem; margin-bottom: 0.25rem; border-radius: var(--cal-border-radius); }}
            .list-item.event {{ background: var(--cal-primary); color: white; }}
            .list-item.booking {{ background: {hash}28a745; color: white; }}
            .nav {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; gap: 0.5rem; }}
            .nav select {{ padding: 0.5rem; border-radius: var(--cal-border-radius); border: 1px solid {hash}ddd; }}
        </style>
        <div id=\"calendar-view\">
            <div class=\"nav\">
                <button class=\"btn\" hx-get=\"{base_url}/view/{cal_id}/{slug}?date={prev}&view={view}{css_query}\" hx-target=\"{hash}calendar-view\" hx-swap=\"outerHTML\">&larr; Previous</button>
                <div style=\"display: flex; align-items: center; gap: 0.5rem;\">
                    {view_selector}
                    <h2 style=\"margin: 0;\">{month} {year}</h2>
                </div>
                <button class=\"btn\" hx-get=\"{base_url}/view/{cal_id}/{slug}?date={next}&view={view}{css_query}\" hx-target=\"{hash}calendar-view\" hx-swap=\"outerHTML\">Next &rarr;</button>
            </div>
            {list}
        </div>",
        base_url = base_url,
        cal_id = html_escape(&calendar.id),
        slug = html_escape(&link.slug),
        month = month_name(month),
        year = year,
        prev = html_escape(&prev_month),
        next = html_escape(&next_month),
        view = view_str,
        css_query = css_query,
        view_selector = view_selector,
        list = list_html,
        hash = HASH,
    )
}

fn render_day_items(link: &ViewLink, events: &[&CalendarEvent], bookings: &[&Booking], detailed: bool) -> String {
    let mut html = String::new();

    if link.show_events {
        if link.show_event_details || detailed {
            for e in events {
                html.push_str(&format!("<div class=\"event\">{}</div>", html_escape(&e.title)));
            }
        } else if !events.is_empty() {
            html.push_str(&format!("<div class=\"busy\">{} event{}</div>", events.len(), if events.len() == 1 { "" } else { "s" }));
        }
    }

    if link.show_bookings {
        if link.show_booking_details || detailed {
            for b in bookings {
                html.push_str(&format!("<div class=\"booking\">{} - {}</div>", format_time(&b.slot_time), html_escape(&b.name)));
            }
        } else if !bookings.is_empty() {
            html.push_str(&format!("<div class=\"busy\">{} booking{}</div>", bookings.len(), if bookings.len() == 1 { "" } else { "s" }));
        }
    }

    html
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
