//! Admin dashboard and calendar management templates

use std::collections::HashMap;

use crate::helpers::*;
use crate::types::*;

use super::base::{base_html, timezone_options};
use super::HASH;

pub fn admin_dashboard_html(
    forms: &[FormConfig],
    calendars: &[CalendarConfig],
    response_counts: &HashMap<String, i64>,
    base_url: &str,
) -> String {
    // Split forms into active and archived
    let active_forms: Vec<_> = forms.iter().filter(|f| !f.archived).collect();
    let archived_forms: Vec<_> = forms.iter().filter(|f| f.archived).collect();

    // Split calendars into active and archived
    let active_calendars: Vec<_> = calendars.iter().filter(|c| !c.archived).collect();
    let archived_calendars: Vec<_> = calendars.iter().filter(|c| c.archived).collect();

    // Active forms section
    let form_rows: String = active_forms
        .iter()
        .map(|f| {
            let count = response_counts.get(&f.slug).unwrap_or(&0);
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/forms/{slug}\">{name}</a></td>
                    <td><code>/f/{slug}</code></td>
                    <td>{count}</td>
                    <td>
                        <button onclick=\"copyLink('/f/{slug}', this)\" class=\"btn btn-sm\">Copy Link</button>
                        <a href=\"{base_url}/admin/forms/{slug}/responses\" class=\"btn btn-sm\">Responses</a>
                        <a href=\"{base_url}/admin/forms/{slug}\" class=\"btn btn-sm\">Edit</a>
                        <button class=\"btn btn-sm btn-secondary\"
                                hx-post=\"{base_url}/admin/forms/{slug}/archive\"
                                hx-confirm=\"Archive this form? It will become read-only.\"
                                hx-target=\"closest tr\"
                                hx-swap=\"outerHTML\">Archive</button>
                    </td>
                </tr>",
                base_url = base_url,
                name = html_escape(&f.name),
                slug = html_escape(&f.slug),
                count = count
            )
        })
        .collect();

    // Archived forms section
    let archived_form_rows: String = archived_forms
        .iter()
        .map(|f| {
            let count = response_counts.get(&f.slug).unwrap_or(&0);
            format!(
                "<tr>
                    <td>{name} <span style=\"color:#666;font-size:0.85em;\">(archived)</span></td>
                    <td><code>/f/{slug}</code></td>
                    <td>{count}</td>
                    <td>
                        <a href=\"{base_url}/admin/forms/{slug}/responses\" class=\"btn btn-sm\">Responses</a>
                        <a href=\"{base_url}/admin/forms/{slug}\" class=\"btn btn-sm\">View</a>
                        <button class=\"btn btn-sm\"
                                hx-post=\"{base_url}/admin/forms/{slug}/unarchive\"
                                hx-target=\"closest tr\"
                                hx-swap=\"outerHTML\">Unarchive</button>
                    </td>
                </tr>",
                base_url = base_url,
                name = html_escape(&f.name),
                slug = html_escape(&f.slug),
                count = count
            )
        })
        .collect();

    // Active calendars section
    let calendar_rows: String = active_calendars
        .iter()
        .map(|cal| {
            let bc = cal.booking_links.len();
            let vc = cal.view_links.len();
            format!(
                "<tr>
                    <td><a href=\"{base_url}/admin/calendars/{id}\">{name}</a></td>
                    <td>{bc} booking {bc_s}</td>
                    <td>{vc} view {vc_s}</td>
                    <td>{updated}</td>
                    <td>
                        <a href=\"{base_url}/admin/calendars/{id}\" class=\"btn btn-sm\">Edit</a>
                        <button class=\"btn btn-sm btn-secondary\"
                                hx-post=\"{base_url}/admin/calendars/{id}/archive\"
                                hx-confirm=\"Archive this calendar? It will become read-only.\"
                                hx-target=\"closest tr\"
                                hx-swap=\"outerHTML\">Archive</button>
                    </td>
                </tr>",
                base_url = base_url,
                id = html_escape(&cal.id),
                name = html_escape(&cal.name),
                bc = bc,
                bc_s = if bc == 1 { "link" } else { "links" },
                vc = vc,
                vc_s = if vc == 1 { "link" } else { "links" },
                updated = html_escape(cal.updated_at.split('T').next().unwrap_or("")),
            )
        })
        .collect();

    // Archived calendars section
    let archived_calendar_rows: String = archived_calendars
        .iter()
        .map(|cal| {
            let bc = cal.booking_links.len();
            let vc = cal.view_links.len();
            format!(
                "<tr>
                    <td>{name} <span style=\"color:#666;font-size:0.85em;\">(archived)</span></td>
                    <td>{bc} booking {bc_s}</td>
                    <td>{vc} view {vc_s}</td>
                    <td>{updated}</td>
                    <td>
                        <a href=\"{base_url}/admin/calendars/{id}\" class=\"btn btn-sm\">View</a>
                        <button class=\"btn btn-sm\"
                                hx-post=\"{base_url}/admin/calendars/{id}/unarchive\"
                                hx-target=\"closest tr\"
                                hx-swap=\"outerHTML\">Unarchive</button>
                    </td>
                </tr>",
                base_url = base_url,
                id = html_escape(&cal.id),
                name = html_escape(&cal.name),
                bc = bc,
                bc_s = if bc == 1 { "link" } else { "links" },
                vc = vc,
                vc_s = if vc == 1 { "link" } else { "links" },
                updated = html_escape(cal.updated_at.split('T').next().unwrap_or("")),
            )
        })
        .collect();

    // Build archived sections HTML
    let archived_forms_section = if archived_forms.is_empty() {
        String::new()
    } else {
        format!(
            "<details style=\"margin-top: 1rem;\">
                <summary style=\"cursor: pointer; color: #666;\">Archived Forms ({count})</summary>
                <table style=\"margin-top: 0.5rem;\">
                    <thead>
                        <tr>
                            <th>Name</th>
                            <th>URL</th>
                            <th>Responses</th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody>{rows}</tbody>
                </table>
            </details>",
            count = archived_forms.len(),
            rows = archived_form_rows
        )
    };

    let archived_calendars_section = if archived_calendars.is_empty() {
        String::new()
    } else {
        format!(
            "<details style=\"margin-top: 1rem;\">
                <summary style=\"cursor: pointer; color: #666;\">Archived Calendars ({count})</summary>
                <table style=\"margin-top: 0.5rem;\">
                    <thead>
                        <tr>
                            <th>Name</th>
                            <th>Booking Links</th>
                            <th>View Links</th>
                            <th>Updated</th>
                            <th>Actions</th>
                        </tr>
                    </thead>
                    <tbody>{rows}</tbody>
                </table>
            </details>",
            count = archived_calendars.len(),
            rows = archived_calendar_rows
        )
    };

    let content = format!(
        "<h1 style=\"display: flex; align-items: center; gap: 0.5rem;\">
            <img src=\"/logo.svg\" alt=\"\" style=\"width: 32px; height: 32px;\">
            Concierge Admin
        </h1>

        <h2 style=\"margin-top: 2rem;\">Forms</h2>
        <p style=\"margin: 1rem 0;\">
            <a href=\"{base_url}/admin/forms/new\" class=\"btn\">+ Create Form</a>
        </p>
        <table id=\"form-list\">
            <thead>
                <tr>
                    <th>Name</th>
                    <th>URL</th>
                    <th>Responses</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody>
                {form_rows}
            </tbody>
        </table>
        {archived_forms_section}

        <h2 style=\"margin-top: 2rem;\">Calendars</h2>
        <p style=\"margin: 1rem 0;\">
            <button class=\"btn\" hx-post=\"{base_url}/admin/calendars\" hx-target=\"{hash}calendar-list tbody\" hx-swap=\"beforeend\">
                + New Calendar
            </button>
        </p>
        <table id=\"calendar-list\">
            <thead>
                <tr>
                    <th>Name</th>
                    <th>Booking Links</th>
                    <th>View Links</th>
                    <th>Updated</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody>
                {calendar_rows}
            </tbody>
        </table>
        {archived_calendars_section}
        <script>
        function copyLink(path, btn) {{
            const url = window.location.origin + path;
            navigator.clipboard.writeText(url).then(() => {{
                const orig = btn.textContent;
                btn.textContent = 'Copied!';
                btn.style.background = '#28a745';
                setTimeout(() => {{
                    btn.textContent = orig;
                    btn.style.background = '';
                }}, 1500);
            }});
        }}
        </script>",
        base_url = base_url,
        form_rows = if form_rows.is_empty() {
            "<tr><td colspan=\"4\" style=\"text-align:center;color:#666;\">No forms yet.</td></tr>".to_string()
        } else {
            form_rows
        },
        calendar_rows = if calendar_rows.is_empty() {
            "<tr><td colspan=\"5\" style=\"text-align:center;color:#666;\">No calendars yet.</td></tr>".to_string()
        } else {
            calendar_rows
        },
        archived_forms_section = archived_forms_section,
        archived_calendars_section = archived_calendars_section,
        hash = HASH,
    );

    base_html("Concierge Admin", &content, &CalendarStyle::default())
}

pub fn admin_calendar_html(calendar: &CalendarConfig, time_slots: &[TimeSlot], base_url: &str) -> String {
    // Build time slots summary
    let time_slots_summary = if time_slots.is_empty() {
        "<p style=\"color: #666;\">No time slots configured. Bookings won't be available until you configure availability.</p>".to_string()
    } else {
        let days = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
        let mut summary_html = String::from("<table style=\"width: 100%; margin-top: 0.5rem;\"><thead><tr><th>Day</th><th>Hours</th><th>Duration</th><th>Capacity</th></tr></thead><tbody>");

        // Group slots by day of week
        for (dow, day_name) in days.iter().enumerate() {
            let day_slots: Vec<_> = time_slots.iter()
                .filter(|s| s.day_of_week == Some(dow as i32))
                .collect();

            if !day_slots.is_empty() {
                for slot in day_slots {
                    summary_html.push_str(&format!(
                        "<tr><td>{}</td><td>{} - {}</td><td>{} min</td><td>{}</td></tr>",
                        day_name,
                        format_time(&slot.start_time),
                        format_time(&slot.end_time),
                        slot.slot_duration,
                        slot.max_bookings
                    ));
                }
            }
        }

        // Add specific date slots
        let date_slots: Vec<_> = time_slots.iter()
            .filter(|s| s.specific_date.is_some())
            .collect();
        for slot in date_slots {
            if let Some(date) = &slot.specific_date {
                summary_html.push_str(&format!(
                    "<tr><td>{}</td><td>{} - {}</td><td>{} min</td><td>{}</td></tr>",
                    html_escape(date),
                    format_time(&slot.start_time),
                    format_time(&slot.end_time),
                    slot.slot_duration,
                    slot.max_bookings
                ));
            }
        }

        summary_html.push_str("</tbody></table>");
        summary_html
    };

    let booking_links_html: String = calendar
        .booking_links
        .iter()
        .map(|link| {
            let url = format!("{}/book/{}/{}", base_url, calendar.id, link.slug);
            format!(
                "<tr>
                    <td>{name}</td>
                    <td><div class=\"url-cell\"><code>{url}</code><button class=\"btn btn-copy\" onclick=\"copyUrl(this, '{url}')\">Copy</button></div></td>
                    <td>{duration} min</td>
                    <td>{status}</td>
                    <td>
                        <a href=\"{base_url}/admin/calendars/{cal_id}/booking/{link_id}\" class=\"btn btn-sm\">Edit</a>
                        <button class=\"btn btn-sm btn-danger\"
                                hx-delete=\"{base_url}/admin/calendars/{cal_id}/booking/{link_id}\"
                                hx-target=\"closest tr\"
                                hx-swap=\"outerHTML\">Delete</button>
                    </td>
                </tr>",
                base_url = base_url,
                cal_id = html_escape(&calendar.id),
                link_id = html_escape(&link.id),
                url = html_escape(&url),
                name = html_escape(&link.name),
                duration = link.duration,
                status = if link.enabled { "Enabled" } else { "Disabled" },
            )
        })
        .collect();

    let view_links_html: String = calendar
        .view_links
        .iter()
        .map(|link| {
            let url = format!("{}/view/{}/{}", base_url, calendar.id, link.slug);
            format!(
                "<tr>
                    <td>{name}</td>
                    <td><div class=\"url-cell\"><code>{url}</code><button class=\"btn btn-copy\" onclick=\"copyUrl(this, '{url}')\">Copy</button></div></td>
                    <td>{view_type:?}</td>
                    <td>{status}</td>
                    <td>
                        <a href=\"{base_url}/admin/calendars/{cal_id}/view/{link_id}\" class=\"btn btn-sm\">Edit</a>
                        <button class=\"btn btn-sm btn-danger\"
                                hx-delete=\"{base_url}/admin/calendars/{cal_id}/view/{link_id}\"
                                hx-target=\"closest tr\"
                                hx-swap=\"outerHTML\">Delete</button>
                    </td>
                </tr>",
                base_url = base_url,
                cal_id = html_escape(&calendar.id),
                link_id = html_escape(&link.id),
                url = html_escape(&url),
                name = html_escape(&link.name),
                view_type = link.view_type,
                status = if link.enabled { "Enabled" } else { "Disabled" },
            )
        })
        .collect();

    let feed_links_html: String = calendar
        .feed_links
        .iter()
        .map(|link| {
            let url = format!("{}/feed/{}/{}?token={}", base_url, calendar.id, link.slug, link.token);
            format!(
                "<tr>
                    <td>{name}</td>
                    <td><div class=\"url-cell\"><code>{url}</code><button class=\"btn btn-copy\" onclick=\"copyUrl(this, '{url}')\">Copy</button></div></td>
                    <td>{status}</td>
                    <td>
                        <button class=\"btn btn-sm btn-danger\"
                                hx-delete=\"{base_url}/admin/calendars/{cal_id}/feed/{link_id}\"
                                hx-target=\"closest tr\"
                                hx-swap=\"outerHTML\">Delete</button>
                    </td>
                </tr>",
                base_url = base_url,
                cal_id = html_escape(&calendar.id),
                link_id = html_escape(&link.id),
                url = html_escape(&url),
                name = html_escape(&link.name),
                status = if link.enabled { "Enabled" } else { "Disabled" },
            )
        })
        .collect();

    let instagram_sources_html: String = calendar
        .instagram_sources
        .iter()
        .map(|source| {
            let last_synced = source
                .last_synced_at
                .as_ref()
                .map(|s| s.split('T').next().unwrap_or("Never"))
                .unwrap_or("Never");
            format!(
                "<tr>
                    <td>@{username}</td>
                    <td>{last_synced}</td>
                    <td>{status}</td>
                    <td>
                        <button class=\"btn btn-sm btn-danger\"
                                hx-delete=\"{base_url}/instagram/disconnect/{cal_id}/{source_id}\"
                                hx-confirm=\"Disconnect this Instagram account?\"
                                hx-target=\"closest tr\"
                                hx-swap=\"outerHTML\">Disconnect</button>
                    </td>
                </tr>",
                base_url = base_url,
                cal_id = html_escape(&calendar.id),
                source_id = html_escape(&source.id),
                username = html_escape(&source.instagram_username),
                last_synced = last_synced,
                status = if source.enabled {
                    "Enabled"
                } else {
                    "Disabled"
                },
            )
        })
        .collect();

    let content = format!(
        "<style>
            .tabs {{ display: flex; gap: 0.5rem; margin-bottom: 1.5rem; flex-wrap: wrap; }}
            .tab {{ padding: 0.5rem 1rem; background: #e9ecef; border-radius: 4px; cursor: pointer; border: none; font-size: 1rem; text-decoration: none; color: inherit; }}
            .tab.active {{ background: #0070f3; color: white; }}
            .tab-content {{ display: none; }}
            .tab-content.active {{ display: block; }}
            .form-actions {{ display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 1rem; }}
        </style>

        <p><a href=\"{base_url}/admin\">&larr; Back to Dashboard</a></p>
        <h1>{name}</h1>
        {archived_notice}

        <div class=\"tabs\">
            <a href=\"#settings\" class=\"tab active\" onclick=\"showTab('settings', this)\">Settings</a>
            <a href=\"#events\" class=\"tab\" onclick=\"showTab('events', this)\">Events</a>
            <a href=\"#bookings\" class=\"tab\" onclick=\"showTab('bookings', this)\">Bookings</a>
        </div>

        <div id=\"tab-settings\" class=\"tab-content active\">
            <div class=\"card\">
                <h2>Calendar Settings</h2>
                <form hx-put=\"{base_url}/admin/calendars/{id}\" hx-swap=\"none\" hx-on::before-request=\"this.querySelector('button[type=submit]').disabled=true;this.querySelector('button[type=submit]').textContent='Saving...'\" hx-on::after-request=\"this.querySelector('button[type=submit]').disabled=false;this.querySelector('button[type=submit]').textContent='Save Settings'\">
                    <div class=\"form-group\">
                        <label>Name</label>
                        <input type=\"text\" name=\"name\" value=\"{name}\" required>
                    </div>
                    <div class=\"form-group\">
                        <label>Description</label>
                        <textarea name=\"description\" rows=\"2\">{description}</textarea>
                    </div>
                    <div class=\"form-group\">
                        <label>Timezone</label>
                        <select name=\"timezone\">
                            {timezone_options}
                        </select>
                    </div>
                    <div class=\"form-group\">
                        <label>Allowed Domains (for embedding)</label>
                        <textarea name=\"allowed_origins\" rows=\"3\" placeholder=\"https://example.com&#10;https://another-site.com&#10;(leave empty to allow all)\">{allowed_origins}</textarea>
                        <small style=\"color: #666;\">One domain per line. Leave empty to allow embedding from any domain.</small>
                    </div>
                    <div class=\"form-group\">
                        <label>Custom CSS</label>
                        <textarea name=\"custom_css\" rows=\"5\" style=\"font-family: monospace; font-size: 0.9rem;\" placeholder=\"/* Custom styles */&#10;.booking-container {{ max-width: 500px; }}\">{custom_css}</textarea>
                        <small style=\"color: #666;\">CSS variables: <code>--cal-primary</code>, <code>--cal-text</code>, <code>--cal-bg</code>, <code>--cal-border-radius</code>, <code>--cal-font</code></small>
                    </div>
                    <div class=\"form-actions\">
                        <button type=\"submit\" class=\"btn\">Save Settings</button>
                    </div>
                </form>
            </div>

            <div class=\"card\">
                <h2>View Links</h2>
                <p style=\"margin-bottom: 1rem; color: #666;\">Public calendar views that can be embedded or shared.</p>
                <button class=\"btn\" hx-post=\"{base_url}/admin/calendars/{id}/view\"
                        hx-target=\"{hash}view-links tbody\" hx-swap=\"beforeend\">+ Add View Link</button>
                <table id=\"view-links\" style=\"margin-top: 1rem;\">
                    <thead><tr><th>Name</th><th>URL</th><th>Type</th><th>Status</th><th>Actions</th></tr></thead>
                    <tbody>{view_links_html}</tbody>
                </table>
            </div>

            <div class=\"card\">
                <h2>Feed Links (iCal)</h2>
                <p style=\"margin-bottom: 1rem; color: #666;\">Subscribe to this calendar from other apps.</p>
                <button class=\"btn\" hx-post=\"{base_url}/admin/calendars/{id}/feed\"
                        hx-target=\"{hash}feed-links tbody\" hx-swap=\"beforeend\"{readonly_disabled}>+ Add Feed Link</button>
                <table id=\"feed-links\" style=\"margin-top: 1rem;\">
                    <thead><tr><th>Name</th><th>URL</th><th>Status</th><th>Actions</th></tr></thead>
                    <tbody>{feed_links_html}</tbody>
                </table>
            </div>

            <div class=\"card\" style=\"border-color: #dc3545;\">
                <h2 style=\"color: #dc3545;\">Danger Zone</h2>
                <p style=\"margin-bottom: 1rem; color: #666;\">Permanently delete this calendar and all its data.</p>
                <button class=\"btn btn-danger\"
                        hx-delete=\"{base_url}/admin/calendars/{id}\"
                        hx-confirm=\"Are you sure you want to permanently delete this calendar? This action cannot be undone.\"
                        hx-on::after-request=\"if(event.detail.successful) window.location.href='/admin'\">Delete Calendar</button>
            </div>
        </div>

        <div id=\"tab-events\" class=\"tab-content\">
            <div class=\"card\">
                <h2>Manage Events</h2>
                <p><a href=\"{base_url}/admin/calendars/{id}/events\" class=\"btn\">Open Event Editor</a></p>
            </div>

            <div class=\"card\">
                <h2>Instagram Sources</h2>
                <p style=\"margin-bottom: 1rem; color: #666;\">Connect Instagram accounts to automatically import events from posts using AI.</p>
                <a href=\"{base_url}/instagram/auth/{id}\" class=\"btn\">Connect Instagram Account</a>
                <table id=\"instagram-sources\" style=\"margin-top: 1rem;\">
                    <thead><tr><th>Account</th><th>Last Synced</th><th>Status</th><th>Actions</th></tr></thead>
                    <tbody>{instagram_sources_html}</tbody>
                </table>
            </div>
        </div>

        <div id=\"tab-bookings\" class=\"tab-content\">
            <div class=\"card\">
                <h2>Time Slots</h2>
                {time_slots_summary}
                <p style=\"margin-top: 1rem;\"><a href=\"{base_url}/admin/calendars/{id}/slots\" class=\"btn\">Configure Available Slots</a></p>
            </div>

            <div class=\"card\">
                <h2>Booking Links</h2>
                <p style=\"margin-bottom: 1rem; color: #666;\">Public booking pages that customers can use to schedule appointments.</p>
                <button class=\"btn\" hx-post=\"{base_url}/admin/calendars/{id}/booking\"
                        hx-target=\"{hash}booking-links tbody\" hx-swap=\"beforeend\">+ Add Booking Link</button>
                <table id=\"booking-links\" style=\"margin-top: 1rem;\">
                    <thead><tr><th>Name</th><th>URL</th><th>Duration</th><th>Status</th><th>Actions</th></tr></thead>
                    <tbody>{booking_links_html}</tbody>
                </table>
            </div>

            <div class=\"card\">
                <h2>All Bookings</h2>
                <p><a href=\"{base_url}/admin/calendars/{id}/bookings\" class=\"btn\">View All Bookings</a></p>
            </div>
        </div>

        <script>
            function showTab(name, el) {{
                document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
                document.querySelectorAll('.tab-content').forEach(t => t.classList.remove('active'));
                document.getElementById('tab-' + name).classList.add('active');
                if (el) el.classList.add('active');
            }}
            // Handle initial hash on page load
            (function() {{
                const hash = window.location.hash.slice(1);
                if (hash && document.getElementById('tab-' + hash)) {{
                    showTab(hash, document.querySelector('a[href=\"#' + hash + '\"]'));
                }}
            }})();
            // Handle browser back/forward
            window.addEventListener('hashchange', function() {{
                const hash = window.location.hash.slice(1);
                if (hash && document.getElementById('tab-' + hash)) {{
                    showTab(hash, document.querySelector('a[href=\"#' + hash + '\"]'));
                }}
            }});
        </script>",
        base_url = base_url,
        id = html_escape(&calendar.id),
        name = html_escape(&calendar.name),
        description = html_escape(calendar.description.as_deref().unwrap_or("")),
        timezone_options = timezone_options(&calendar.timezone),
        allowed_origins = html_escape(&calendar.allowed_origins.join("\n")),
        custom_css = html_escape(&calendar.style.custom_css),
        booking_links_html = booking_links_html,
        view_links_html = view_links_html,
        feed_links_html = feed_links_html,
        instagram_sources_html = instagram_sources_html,
        time_slots_summary = time_slots_summary,
        archived_notice = if calendar.archived {
            "<div class=\"card\" style=\"background: #fff3cd; border-color: #ffc107; margin-bottom: 1rem;\">
                <p style=\"margin: 0; color: #856404;\"><strong>This calendar is archived.</strong> It is read-only. Unarchive from the dashboard to make changes.</p>
            </div>"
        } else { "" },
        readonly_disabled = if calendar.archived { " disabled" } else { "" },
        hash = HASH,
    );

    let title = if calendar.archived {
        format!("{} (Archived)", calendar.name)
    } else {
        format!("Edit: {}", calendar.name)
    };
    base_html(&title, &content, &calendar.style)
}

pub fn admin_events_html(
    calendar: &CalendarConfig,
    events: &[CalendarEvent],
    base_url: &str,
) -> String {
    let event_rows: String = events
        .iter()
        .map(|event| {
            format!(
                "<tr>
                    <td>{title}</td>
                    <td>{start}</td>
                    <td>{end}</td>
                    <td>
                        <button class=\"btn btn-sm\"
                                hx-get=\"{base_url}/admin/calendars/{cal_id}/events/{event_id}/edit\"
                                hx-target=\"{hash}event-form\"
                                hx-swap=\"innerHTML\">Edit</button>
                        <button class=\"btn btn-sm btn-danger\"
                                hx-delete=\"{base_url}/admin/calendars/{cal_id}/events/{event_id}\"
                                hx-target=\"closest tr\"
                                hx-swap=\"outerHTML\">Delete</button>
                    </td>
                </tr>",
                base_url = base_url,
                cal_id = html_escape(&calendar.id),
                event_id = html_escape(&event.id),
                title = html_escape(&event.title),
                start = html_escape(&event.start_time),
                end = html_escape(&event.end_time),
                hash = HASH,
            )
        })
        .collect();

    let content = format!(
        "<p><a href=\"{base_url}/admin/calendars/{id}\">&larr; Back to {name}</a></p>
        <h1>Events: {name}</h1>

        <div class=\"card\" id=\"event-form\">
            <h2>Add Event</h2>
            <form hx-post=\"{base_url}/admin/calendars/{id}/events\" hx-target=\"{hash}event-list tbody\" hx-swap=\"beforeend\" hx-on::after-request=\"this.reset()\">
                <div class=\"form-group\">
                    <label>Title</label>
                    <input type=\"text\" name=\"title\" required>
                </div>
                <div class=\"form-group\">
                    <label>Start</label>
                    <input type=\"datetime-local\" name=\"start_time\" required>
                </div>
                <div class=\"form-group\">
                    <label>End</label>
                    <input type=\"datetime-local\" name=\"end_time\" required>
                </div>
                <div class=\"form-group\">
                    <label>Description</label>
                    <textarea name=\"description\" rows=\"2\"></textarea>
                </div>
                <button type=\"submit\" class=\"btn\">Add Event</button>
            </form>
        </div>

        <div class=\"card\">
            <h2>Upcoming Events</h2>
            <table id=\"event-list\">
                <thead><tr><th>Title</th><th>Start</th><th>End</th><th>Actions</th></tr></thead>
                <tbody>{event_rows}</tbody>
            </table>
        </div>",
        base_url = base_url,
        id = html_escape(&calendar.id),
        name = html_escape(&calendar.name),
        event_rows = event_rows,
        hash = HASH,
    );

    base_html(
        &format!("Events: {}", calendar.name),
        &content,
        &calendar.style,
    )
}

pub fn admin_slots_html(calendar: &CalendarConfig, slots: &[TimeSlot], base_url: &str) -> String {
    let slot_rows: String = slots
        .iter()
        .map(|slot| {
            let day_display = slot
                .day_of_week
                .map(|d| day_name(d as u32).to_string())
                .or_else(|| slot.specific_date.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            format!(
                "<tr>
                    <td>{day}</td>
                    <td>{start} - {end}</td>
                    <td>{duration} min</td>
                    <td>{max}</td>
                    <td>
                        <button class=\"btn btn-sm btn-danger\"
                                hx-delete=\"{base_url}/admin/calendars/{cal_id}/slots/{slot_id}\"
                                hx-target=\"closest tr\"
                                hx-swap=\"outerHTML\">Delete</button>
                    </td>
                </tr>",
                base_url = base_url,
                cal_id = html_escape(&calendar.id),
                slot_id = html_escape(&slot.id),
                day = html_escape(&day_display),
                start = html_escape(&slot.start_time),
                end = html_escape(&slot.end_time),
                duration = slot.slot_duration,
                max = slot.max_bookings,
            )
        })
        .collect();

    let content = format!(
        "<p><a href=\"{base_url}/admin/calendars/{id}\">&larr; Back to {name}</a></p>
        <h1>Time Slots: {name}</h1>

        <div class=\"card\">
            <h2>Add Recurring Slot</h2>
            <form hx-post=\"{base_url}/admin/calendars/{id}/slots\" hx-target=\"{hash}slot-list tbody\" hx-swap=\"beforeend\" hx-on::after-request=\"this.reset()\">
                <div class=\"form-group\">
                    <label>Day of Week</label>
                    <select name=\"day_of_week\">
                        <option value=\"1\">Monday</option>
                        <option value=\"2\">Tuesday</option>
                        <option value=\"3\">Wednesday</option>
                        <option value=\"4\">Thursday</option>
                        <option value=\"5\">Friday</option>
                        <option value=\"6\">Saturday</option>
                        <option value=\"0\">Sunday</option>
                    </select>
                </div>
                <div class=\"form-group\">
                    <label>Start Time</label>
                    <input type=\"time\" name=\"start_time\" value=\"09:00\" required>
                </div>
                <div class=\"form-group\">
                    <label>End Time</label>
                    <input type=\"time\" name=\"end_time\" value=\"17:00\" required>
                </div>
                <div class=\"form-group\">
                    <label>Slot Duration (minutes)</label>
                    <input type=\"number\" name=\"slot_duration\" value=\"30\" min=\"5\" max=\"480\" required>
                </div>
                <div class=\"form-group\">
                    <label>Max Bookings per Slot</label>
                    <input type=\"number\" name=\"max_bookings\" value=\"1\" min=\"1\" required>
                </div>
                <button type=\"submit\" class=\"btn\">Add Slot</button>
            </form>
        </div>

        <div class=\"card\">
            <h2>Configured Slots</h2>
            <table id=\"slot-list\">
                <thead><tr><th>Day</th><th>Time Range</th><th>Slot Duration</th><th>Max Bookings</th><th>Actions</th></tr></thead>
                <tbody>{slot_rows}</tbody>
            </table>
        </div>",
        base_url = base_url,
        id = html_escape(&calendar.id),
        name = html_escape(&calendar.name),
        slot_rows = slot_rows,
        hash = HASH,
    );

    base_html(
        &format!("Slots: {}", calendar.name),
        &content,
        &calendar.style,
    )
}

pub fn admin_bookings_html(
    calendar: &CalendarConfig,
    bookings: &[Booking],
    base_url: &str,
) -> String {
    let booking_rows: String = bookings
        .iter()
        .map(|booking| {
            format!(
                "<tr>
                    <td>{date}</td>
                    <td>{time}</td>
                    <td>{name}</td>
                    <td>{email}</td>
                    <td>{status:?}</td>
                    <td>
                        <button class=\"btn btn-sm btn-danger\"
                                hx-post=\"{base_url}/admin/calendars/{cal_id}/bookings/{booking_id}/cancel\"
                                hx-target=\"closest tr\"
                                hx-swap=\"outerHTML\">Cancel</button>
                    </td>
                </tr>",
                base_url = base_url,
                cal_id = html_escape(&calendar.id),
                booking_id = html_escape(&booking.id),
                date = html_escape(&booking.slot_date),
                time = html_escape(&booking.slot_time),
                name = html_escape(&booking.name),
                email = html_escape(&booking.email),
                status = booking.status,
            )
        })
        .collect();

    let content = format!(
        "<p><a href=\"{base_url}/admin/calendars/{id}\">&larr; Back to {name}</a></p>
        <h1>Bookings: {name}</h1>

        <div class=\"card\">
            <table>
                <thead><tr><th>Date</th><th>Time</th><th>Name</th><th>Email</th><th>Status</th><th>Actions</th></tr></thead>
                <tbody>{booking_rows}</tbody>
            </table>
        </div>",
        base_url = base_url,
        id = html_escape(&calendar.id),
        name = html_escape(&calendar.name),
        booking_rows = booking_rows,
    );

    base_html(
        &format!("Bookings: {}", calendar.name),
        &content,
        &calendar.style,
    )
}

pub fn admin_booking_link_html(
    calendar: &CalendarConfig,
    link: &BookingLink,
    base_url: &str,
    channels: &super::base::AvailableChannels,
) -> String {
    let responders_json = serde_json::to_string(&link.responders).unwrap_or_else(|_| "[]".into());
    let admin_responders_json =
        serde_json::to_string(&link.admin_responders).unwrap_or_else(|_| "[]".into());
    let js_escape = |s: &str| {
        s.replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n")
    };

    // Build channel options based on what's available
    let mut channel_options = Vec::new();
    if channels.twilio_sms {
        channel_options.push(("twilio_sms", "Twilio SMS"));
    }
    if channels.twilio_whatsapp {
        channel_options.push(("twilio_whatsapp", "Twilio WhatsApp"));
    }
    if channels.twilio_email {
        channel_options.push(("twilio_email", "Twilio Email"));
    }
    if channels.resend_email {
        channel_options.push(("resend_email", "Resend Email"));
    }

    let has_channels = !channel_options.is_empty();
    let default_channel = channel_options
        .first()
        .map(|(c, _)| *c)
        .unwrap_or("resend_email");
    let is_default_email = default_channel.contains("email");

    // Build JS channel options string
    let js_channel_options: String = channel_options
        .iter()
        .map(|(value, label)| {
            format!(
                "<option value=\"{}\" ${{r.channel==='{}'?'selected':''}}>{}</option>",
                value, value, label
            )
        })
        .collect::<Vec<_>>()
        .join("\\n                                ");

    let responders_section = if has_channels {
        r#"<h3 style="margin-top: 1.5rem; margin-bottom: 1rem;">Customer Notifications</h3>
                <p style="color: #666; margin-bottom: 1rem;">Send automatic confirmation messages to customers when bookings are confirmed. Use {{{{name}}}}, {{{{email}}}}, {{{{date}}}}, {{{{time}}}} placeholders.</p>
                <div id="responders-list"></div>
                <button type="button" class="btn btn-secondary" onclick="addResponder()" style="margin-bottom: 1rem;">+ Add Responder</button>
                <input type="hidden" name="responders_json" id="responders-json">"#.to_string()
    } else {
        String::from(
            r#"<input type="hidden" name="responders_json" id="responders-json" value="[]">"#,
        )
    };

    let admin_responders_section = if has_channels {
        r#"<h3 style="margin-top: 1.5rem; margin-bottom: 1rem;">Admin Notifications</h3>
                <p style="color: #666; margin-bottom: 1rem;">Receive notifications when bookings need approval (when auto-accept is disabled). Use {{{{name}}}}, {{{{email}}}}, {{{{date}}}}, {{{{time}}}}, {{{{approve_url}}}} placeholders.</p>
                <div id="admin-responders-list"></div>
                <button type="button" class="btn btn-secondary" onclick="addAdminResponder()" style="margin-bottom: 1rem;">+ Add Admin Responder</button>
                <input type="hidden" name="admin_responders_json" id="admin-responders-json">"#.to_string()
    } else {
        String::from(
            r#"<input type="hidden" name="admin_responders_json" id="admin-responders-json" value="[]">"#,
        )
    };

    let responders_script = if has_channels {
        format!(
            r#"<script>
            let responders = {responders_json};
            let adminResponders = {admin_responders_json};

            function renderResponderList(list, containerId, isAdmin) {{
                const container = document.getElementById(containerId);
                container.innerHTML = list.map((r, i) => {{
                    const isEmail = r.channel === 'twilio_email' || r.channel === 'resend_email';
                    const isSms = r.channel === 'twilio_sms' || r.channel === 'twilio_whatsapp';
                    const listName = isAdmin ? 'adminResponders' : 'responders';
                    const renderFn = isAdmin ? 'renderAdminResponders' : 'renderResponders';
                    const removeFn = isAdmin ? 'removeAdminResponder' : 'removeResponder';
                    const targetPlaceholder = isEmail ? 'admin@example.com' : '+1234567890';
                    const targetLabel = isEmail ? 'Email' : 'Phone';
                    return `<div class="card" style="margin-bottom:1rem;padding:1rem;background:#f8f9fa;">
                        <div style="display:flex;gap:0.5rem;margin-bottom:0.5rem;align-items:center;flex-wrap:wrap;">
                            <input type="text" value="${{r.name||''}}" onchange="${{listName}}[${{i}}].name=this.value" placeholder="Responder Name" style="flex:1;min-width:150px;">
                            <select onchange="${{listName}}[${{i}}].channel=this.value;${{renderFn}}();">
                                {js_channel_options}
                            </select>
                            <label style="white-space:nowrap;"><input type="checkbox" ${{r.enabled?'checked':''}} onchange="${{listName}}[${{i}}].enabled=this.checked" style="width:auto;"> Enabled</label>
                            <button type="button" class="btn btn-sm btn-danger" onclick="${{removeFn}}(${{i}})">Delete</button>
                        </div>
                        ${{isAdmin ? `<div class="form-group" style="margin-bottom:0.5rem;">
                            <input type="${{isEmail ? 'email' : 'tel'}}" value="${{r.target_field||''}}" onchange="${{listName}}[${{i}}].target_field=this.value" placeholder="${{targetPlaceholder}}" style="width:100%;">
                            <small style="color:#666;">Your ${{targetLabel.toLowerCase()}} address for notifications</small>
                        </div>` : ''}}
                        ${{isEmail ? `<div class="form-group" style="margin-bottom:0.5rem;">
                            <input type="text" value="${{r.subject||''}}" onchange="${{listName}}[${{i}}].subject=this.value" placeholder="Email Subject">
                        </div>` : ''}}
                        <div class="form-group" style="margin-bottom:0;">
                            <textarea rows="3" onchange="${{listName}}[${{i}}].body=this.value" placeholder="${{isAdmin ? 'Message body. Use {{{{{{{{name}}}}}}}}, {{{{{{{{email}}}}}}}}, {{{{{{{{date}}}}}}}}, {{{{{{{{time}}}}}}}}, {{{{{{{{approve_url}}}}}}}} placeholders.' : 'Message body. Use {{{{{{{{name}}}}}}}}, {{{{{{{{email}}}}}}}}, {{{{{{{{date}}}}}}}}, {{{{{{{{time}}}}}}}} placeholders.'}}">${{r.body||''}}</textarea>
                        </div>
                    </div>`;
                }}).join('');
            }}

            function renderResponders() {{
                renderResponderList(responders, 'responders-list', false);
            }}

            function renderAdminResponders() {{
                renderResponderList(adminResponders, 'admin-responders-list', true);
            }}

            function addResponder() {{
                responders.push({{
                    id: 'resp_' + Date.now(),
                    name: 'Booking Confirmation',
                    channel: '{default_channel}',
                    target_field: 'email',
                    subject: {default_subject},
                    body: 'Hi {{{{{{{{name}}}}}}}},\\n\\nYour booking for {{{{{{{{date}}}}}}}} at {{{{{{{{time}}}}}}}} has been confirmed.\\n\\nThank you!',
                    enabled: true,
                    use_ai: false
                }});
                renderResponders();
            }}

            function addAdminResponder() {{
                adminResponders.push({{
                    id: 'admin_' + Date.now(),
                    name: 'Approval Request',
                    channel: '{default_channel}',
                    target_field: '',
                    subject: {admin_default_subject},
                    body: 'New booking request from {{{{{{{{name}}}}}}}} ({{{{{{{{email}}}}}}}}) for {{{{{{{{date}}}}}}}} at {{{{{{{{time}}}}}}}}.\\n\\nApprove: {{{{{{{{approve_url}}}}}}}}',
                    enabled: true,
                    use_ai: false
                }});
                renderAdminResponders();
            }}

            function removeResponder(i) {{
                responders.splice(i, 1);
                renderResponders();
            }}

            function removeAdminResponder(i) {{
                adminResponders.splice(i, 1);
                renderAdminResponders();
            }}

            function updateRespondersField() {{
                document.getElementById('responders-json').value = JSON.stringify(responders);
                document.getElementById('admin-responders-json').value = JSON.stringify(adminResponders);
            }}

            renderResponders();
            renderAdminResponders();
        </script>"#,
            responders_json = js_escape(&responders_json),
            admin_responders_json = js_escape(&admin_responders_json),
            js_channel_options = js_channel_options,
            default_channel = default_channel,
            default_subject = if is_default_email {
                "'Your booking is confirmed'"
            } else {
                "''"
            },
            admin_default_subject = if is_default_email {
                "'New booking requires approval'"
            } else {
                "''"
            },
        )
    } else {
        String::from("<script>function updateRespondersField() { document.getElementById('responders-json').value = '[]'; document.getElementById('admin-responders-json').value = '[]'; }</script>")
    };

    let content = format!(
        "<p><a href=\"{base_url}/admin/calendars/{cal_id}\">&larr; Back to {cal_name}</a></p>
        <h1>Edit Booking Link: {name}</h1>

        <div class=\"card\">
            <p><strong>URL:</strong> <code>{base_url}/book/{cal_id}/{slug}</code></p>
        </div>

        <div class=\"card\">
            <form id=\"booking-link-form\" hx-put=\"{base_url}/admin/calendars/{cal_id}/booking/{link_id}\" hx-swap=\"none\" hx-on::before-request=\"updateRespondersField();this.querySelector('button[type=submit]').disabled=true;this.querySelector('button[type=submit]').textContent='Saving...'\" hx-on::after-request=\"this.querySelector('button[type=submit]').disabled=false;this.querySelector('button[type=submit]').textContent='Save Changes'\">
                <div class=\"form-group\">
                    <label>Name</label>
                    <input type=\"text\" name=\"name\" value=\"{name}\" required>
                </div>
                <div class=\"form-group\">
                    <label>Description</label>
                    <textarea name=\"description\" rows=\"2\">{description}</textarea>
                </div>
                <div class=\"form-group\">
                    <label>Duration (minutes)</label>
                    <input type=\"number\" name=\"duration\" value=\"{duration}\" min=\"5\" max=\"480\" required>
                </div>
                <div class=\"form-group\">
                    <label>Minimum Notice (hours)</label>
                    <input type=\"number\" name=\"min_notice\" value=\"{min_notice}\" min=\"0\" required>
                </div>
                <div class=\"form-group\">
                    <label>Maximum Advance Booking (days)</label>
                    <input type=\"number\" name=\"max_advance\" value=\"{max_advance}\" min=\"1\" required>
                </div>
                <div class=\"form-group\">
                    <label>Confirmation Message</label>
                    <textarea name=\"confirmation_message\" rows=\"2\">{confirmation}</textarea>
                </div>

                <h3 style=\"margin-top: 1.5rem; margin-bottom: 1rem;\">Display Settings</h3>
                <div class=\"form-group\">
                    <label style=\"display: flex; align-items: center; gap: 0.5rem; cursor: pointer;\">
                        <input type=\"checkbox\" name=\"hide_title\" {hide_title_checked} style=\"width: auto;\">
                        Hide title when embedded
                    </label>
                    <small style=\"color: #666;\">Hide the booking link name when embedding in another page</small>
                </div>

                <h3 style=\"margin-top: 1.5rem; margin-bottom: 1rem;\">Approval Settings</h3>
                <div class=\"form-group\">
                    <label style=\"display: flex; align-items: center; gap: 0.5rem; cursor: pointer;\">
                        <input type=\"checkbox\" name=\"auto_accept\" {auto_accept_checked} style=\"width: auto;\" onchange=\"document.getElementById('admin-responders-section').style.display = this.checked ? 'none' : 'block'\">
                        Auto-accept bookings
                    </label>
                    <small style=\"color: #666;\">When unchecked, bookings require manual approval</small>
                </div>
                <div id=\"admin-responders-section\" style=\"{admin_responders_display}\">
                    {admin_responders_section}
                </div>

                {responders_section}

                <div class=\"form-group\">
                    <label style=\"display: flex; align-items: center; gap: 0.5rem; cursor: pointer;\">
                        <input type=\"checkbox\" name=\"enabled\" {enabled_checked} style=\"width: auto;\">
                        Enabled
                    </label>
                </div>
                <div class=\"form-actions\">
                    <button type=\"submit\" class=\"btn\">Save Changes</button>
                </div>
            </form>
        </div>

        {responders_script}",
        base_url = base_url,
        cal_id = html_escape(&calendar.id),
        cal_name = html_escape(&calendar.name),
        link_id = html_escape(&link.id),
        slug = html_escape(&link.slug),
        name = html_escape(&link.name),
        description = html_escape(link.description.as_deref().unwrap_or("")),
        duration = link.duration,
        min_notice = link.min_notice,
        max_advance = link.max_advance,
        confirmation = html_escape(&link.confirmation_message),
        hide_title_checked = if link.hide_title { "checked" } else { "" },
        auto_accept_checked = if link.auto_accept { "checked" } else { "" },
        admin_responders_section = admin_responders_section,
        admin_responders_display = if link.auto_accept { "display: none;" } else { "" },
        enabled_checked = if link.enabled { "checked" } else { "" },
        responders_section = responders_section,
        responders_script = responders_script,
    );

    base_html(&format!("Edit: {}", link.name), &content, &calendar.style)
}

pub fn admin_view_link_html(calendar: &CalendarConfig, link: &ViewLink, base_url: &str) -> String {
    let content = format!(
        "<p><a href=\"{base_url}/admin/calendars/{cal_id}\">&larr; Back to {cal_name}</a></p>
        <h1>Edit View Link: {name}</h1>

        <div class=\"card\">
            <p><strong>URL:</strong> <code>{base_url}/view/{cal_id}/{slug}</code></p>
        </div>

        <div class=\"card\">
            <form hx-put=\"{base_url}/admin/calendars/{cal_id}/view/{link_id}\" hx-swap=\"none\" hx-on::before-request=\"this.querySelector('button[type=submit]').disabled=true;this.querySelector('button[type=submit]').textContent='Saving...'\" hx-on::after-request=\"this.querySelector('button[type=submit]').disabled=false;this.querySelector('button[type=submit]').textContent='Save Changes'\">
                <div class=\"form-group\">
                    <label>Name</label>
                    <input type=\"text\" name=\"name\" value=\"{name}\" required>
                </div>
                <div class=\"form-group\">
                    <label>View Type</label>
                    <select name=\"view_type\">
                        <option value=\"week\" {week_sel}>Week</option>
                        <option value=\"month\" {month_sel}>Month</option>
                        <option value=\"year\" {year_sel}>Year</option>
                        <option value=\"endless\" {endless_sel}>Endless</option>
                    </select>
                </div>
                <h3 style=\"margin-top: 1.5rem; margin-bottom: 1rem;\">Events</h3>
                <div class=\"form-group\">
                    <label>
                        <input type=\"checkbox\" name=\"show_events\" {events_checked}>
                        Show Events
                    </label>
                </div>
                <div class=\"form-group\">
                    <label>
                        <input type=\"checkbox\" name=\"show_event_details\" {event_details_checked}>
                        Show Event Details (titles, times)
                    </label>
                    <small style=\"color: #666; display: block; margin-top: 0.25rem;\">When unchecked, shows \"busy\" instead of details</small>
                </div>

                <h3 style=\"margin-top: 1.5rem; margin-bottom: 1rem;\">Bookings</h3>
                <div class=\"form-group\">
                    <label>
                        <input type=\"checkbox\" name=\"show_bookings\" {bookings_checked}>
                        Show Bookings
                    </label>
                </div>
                <div class=\"form-group\">
                    <label>
                        <input type=\"checkbox\" name=\"show_booking_details\" {booking_details_checked}>
                        Show Booking Details (names, times)
                    </label>
                    <small style=\"color: #666; display: block; margin-top: 0.25rem;\">When unchecked, shows \"busy\" instead of details</small>
                </div>
                <div class=\"form-group\">
                    <label>
                        <input type=\"checkbox\" name=\"enabled\" {enabled_checked}>
                        Enabled
                    </label>
                </div>
                <div class=\"form-actions\">
                    <button type=\"submit\" class=\"btn\">Save Changes</button>
                </div>
            </form>
        </div>",
        base_url = base_url,
        cal_id = html_escape(&calendar.id),
        cal_name = html_escape(&calendar.name),
        link_id = html_escape(&link.id),
        slug = html_escape(&link.slug),
        name = html_escape(&link.name),
        week_sel = if matches!(link.view_type, ViewType::Week) { "selected" } else { "" },
        month_sel = if matches!(link.view_type, ViewType::Month) { "selected" } else { "" },
        year_sel = if matches!(link.view_type, ViewType::Year) { "selected" } else { "" },
        endless_sel = if matches!(link.view_type, ViewType::Endless) { "selected" } else { "" },
        events_checked = if link.show_events { "checked" } else { "" },
        event_details_checked = if link.show_event_details { "checked" } else { "" },
        bookings_checked = if link.show_bookings { "checked" } else { "" },
        booking_details_checked = if link.show_booking_details { "checked" } else { "" },
        enabled_checked = if link.enabled { "checked" } else { "" },
    );

    base_html(&format!("Edit: {}", link.name), &content, &calendar.style)
}
