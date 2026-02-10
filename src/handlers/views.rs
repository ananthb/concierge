//! Public view handlers - calendar views and iCal feeds

use worker::*;

use super::{get_base_url, get_origin};
use crate::helpers::*;
use crate::storage::*;
use crate::templates::*;
use crate::types::*;

/// Handle public view routes (/view/*)
pub async fn handle_view(req: Request, env: Env, path: &str, method: Method) -> Result<Response> {
    let base_url = get_base_url(&req);
    let origin = get_origin(&req);
    let kv = env.kv("CALENDARS_KV")?;
    let db = env.d1("DB")?;

    let path_parts: Vec<&str> = path
        .strip_prefix("/view/")
        .unwrap_or("")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let calendar_id = path_parts.first().copied().unwrap_or("");
    let slug = path_parts.get(1).copied().unwrap_or("");

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
        return Response::error("View link required", 400);
    }

    let calendar = match get_calendar(&kv, calendar_id).await? {
        Some(c) => c,
        None => return Response::error("Calendar not found", 404),
    };

    if method != Method::Get {
        return Response::error("Method not allowed", 405);
    }

    let link = match calendar
        .view_links
        .iter()
        .find(|l| l.slug == slug && l.enabled)
    {
        Some(l) => l.clone(),
        None => return Response::error("View link not found", 404),
    };

    let url = req.url()?;
    let query_pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let date = query_pairs
        .get("date")
        .map(|s| s.to_string())
        .unwrap_or_else(today_date);
    let inline_css = query_pairs.get("css").map(|s| s.to_string());
    let css_url = query_pairs.get("css_url").map(|s| s.to_string());
    let css_options = CssOptions {
        inline_css: inline_css.as_deref(),
        css_url: css_url.as_deref(),
    };

    let view_type = query_pairs
        .get("view")
        .map(|v| match v.as_ref() {
            "week" => ViewType::Week,
            "month" => ViewType::Month,
            "year" => ViewType::Year,
            "endless" => ViewType::Endless,
            _ => link.view_type.clone(),
        })
        .unwrap_or_else(|| link.view_type.clone());

    let hide_title = query_pairs
        .get("notitle")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);

    let (start_date, end_date) = match &view_type {
        ViewType::Week => {
            let start = start_of_week(&date);
            let end = add_days(&start, 6);
            (start, end)
        }
        ViewType::Month => {
            let start = start_of_month(&date);
            let end = end_of_month(&date);
            (start, end)
        }
        ViewType::Year => {
            let (year, _, _) = parse_date(&date).unwrap_or((2024, 1, 1));
            (format!("{}-01-01", year), format!("{}-12-31", year))
        }
        ViewType::Endless => {
            let start = start_of_month(&date);
            let end = add_days(&start, 365);
            (start, end)
        }
    };

    let (start_date, end_date) = if let Some(range) = &link.date_range {
        (
            if start_date < range.start {
                range.start.clone()
            } else {
                start_date
            },
            if end_date > range.end {
                range.end.clone()
            } else {
                end_date
            },
        )
    } else {
        (start_date, end_date)
    };

    let events = get_events(&db, calendar_id, &start_date, &end_date).await?;
    let bookings = get_bookings(&db, calendar_id, &start_date, &end_date).await?;

    let is_htmx = is_htmx_request(&req);
    let html = calendar_view_html(
        &calendar,
        &link,
        &view_type,
        &events,
        &bookings,
        &date,
        &base_url,
        &css_options,
        is_htmx,
        hide_title,
    );
    let response = Response::from_html(html)?;
    Ok(with_cors(
        response,
        origin.as_deref(),
        &calendar.allowed_origins,
    ))
}

/// Handle iCal feed routes (/feed/*)
pub async fn handle_feed(req: Request, env: Env, path: &str, method: Method) -> Result<Response> {
    if method != Method::Get {
        return Response::error("Method not allowed", 405);
    }

    let kv = env.kv("CALENDARS_KV")?;
    let db = env.d1("DB")?;

    let path_parts: Vec<&str> = path
        .strip_prefix("/feed/")
        .unwrap_or("")
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let calendar_id = match path_parts.first() {
        Some(id) => *id,
        None => return Response::error("Calendar ID required", 400),
    };
    let slug = match path_parts.get(1) {
        Some(s) => *s,
        None => return Response::error("Feed link required", 400),
    };

    let calendar = match get_calendar(&kv, calendar_id).await? {
        Some(c) => c,
        None => return Response::error("Calendar not found", 404),
    };

    let link = match calendar
        .feed_links
        .iter()
        .find(|l| l.slug == slug && l.enabled)
    {
        Some(l) => l.clone(),
        None => return Response::error("Feed link not found", 404),
    };

    let url = req.url()?;
    let token = url
        .query_pairs()
        .find(|(k, _)| k == "token")
        .map(|(_, v)| v.to_string());

    if token.as_ref() != Some(&link.token) {
        return Response::error("Invalid token", 403);
    }

    let today = today_date();
    let start_date = add_days(&today, -365);
    let end_date = add_days(&today, 365);

    let events = get_events(&db, calendar_id, &start_date, &end_date).await?;
    let bookings = if link.include_details {
        get_bookings(&db, calendar_id, &start_date, &end_date).await?
    } else {
        Vec::new()
    };

    let ical = ical_feed(&calendar, &events, &bookings);

    let headers = Headers::new();
    headers.set("Content-Type", "text/calendar; charset=utf-8")?;
    headers.set(
        "Content-Disposition",
        &format!("attachment; filename=\"{}.ics\"", calendar.name),
    )?;

    Ok(Response::ok(ical)?.with_headers(headers))
}
