use worker::*;

/// Check if request is an HTMX request
pub fn is_htmx_request(req: &Request) -> bool {
    req.headers()
        .get("HX-Request")
        .ok()
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false)
}

/// Generate a unique ID
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Generate a URL-friendly slug
pub fn generate_slug() -> String {
    let adjectives = [
        "swift", "bright", "calm", "bold", "warm", "cool", "soft", "keen", "quick", "light",
        "fresh", "clear", "smart", "sharp", "neat", "fine",
    ];
    let nouns = [
        "fox", "owl", "bear", "wolf", "hawk", "deer", "swan", "dove", "lynx", "crow", "hare",
        "seal", "wren", "lark", "moth", "newt",
    ];

    let adj_idx = (js_sys::Math::random() * adjectives.len() as f64) as usize;
    let noun_idx = (js_sys::Math::random() * nouns.len() as f64) as usize;
    let suffix: String = (0..3)
        .map(|_| {
            let chars = "abcdefghijklmnopqrstuvwxyz0123456789";
            let idx = (js_sys::Math::random() * chars.len() as f64) as usize;
            chars.chars().nth(idx).unwrap_or('x')
        })
        .collect();

    format!("{}-{}-{}", adjectives[adj_idx], nouns[noun_idx], suffix)
}

/// Generate a secure token
pub fn generate_token() -> String {
    (0..32)
        .map(|_| {
            let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            let idx = (js_sys::Math::random() * chars.len() as f64) as usize;
            chars.chars().nth(idx).unwrap_or('x')
        })
        .collect()
}

/// Get current ISO timestamp
pub fn now_iso() -> String {
    js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_else(|| String::from("1970-01-01T00:00:00.000Z"))
}

/// Get today's date in YYYY-MM-DD format
pub fn today_date() -> String {
    let date = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date()
    )
}

/// HTML escape for XSS prevention
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// URL encode for query parameters
pub fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            ' ' => result.push_str("%20"),
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}

/// Normalize an origin for comparison (lowercase, no trailing slash)
fn normalize_origin(origin: &str) -> String {
    origin.to_lowercase().trim_end_matches('/').to_string()
}

/// Check if origin is allowed
pub fn is_origin_allowed(origin: &str, allowed_origins: &[String]) -> bool {
    if allowed_origins.is_empty() {
        return true; // Allow all if no specific origins configured
    }
    let normalized_origin = normalize_origin(origin);
    allowed_origins
        .iter()
        .any(|allowed| normalize_origin(allowed) == normalized_origin)
}

/// Add CORS headers to response
pub fn with_cors(
    mut response: Response,
    origin: Option<&str>,
    allowed_origins: &[String],
) -> Response {
    if let Some(origin) = origin {
        if is_origin_allowed(origin, allowed_origins) {
            let headers = response.headers_mut();
            let _ = headers.set("Access-Control-Allow-Origin", origin);
            let _ = headers.set(
                "Access-Control-Allow-Methods",
                "GET, POST, PUT, DELETE, OPTIONS",
            );
            let _ = headers.set(
                "Access-Control-Allow-Headers",
                "Content-Type, HX-Request, HX-Target, HX-Current-URL, HX-Trigger",
            );
            let _ = headers.set("Access-Control-Max-Age", "86400");
            let _ = headers.set("Vary", "Origin");
        }
    }

    response
}

/// Handle CORS preflight
pub fn cors_preflight(origin: Option<&str>, allowed_origins: &[String]) -> Result<Response> {
    let mut response = Response::empty()?.with_status(204);

    if let Some(origin) = origin {
        if is_origin_allowed(origin, allowed_origins) {
            let headers = response.headers_mut();
            let _ = headers.set("Access-Control-Allow-Origin", origin);
            let _ = headers.set(
                "Access-Control-Allow-Methods",
                "GET, POST, PUT, DELETE, OPTIONS",
            );
            let _ = headers.set(
                "Access-Control-Allow-Headers",
                "Content-Type, HX-Request, HX-Target, HX-Current-URL, HX-Trigger",
            );
            let _ = headers.set("Access-Control-Max-Age", "86400");
            let _ = headers.set("Vary", "Origin");
        }
    }

    Ok(response)
}

/// Parse date string to components
pub fn parse_date(date: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year = parts[0].parse().ok()?;
    let month = parts[1].parse().ok()?;
    let day = parts[2].parse().ok()?;
    Some((year, month, day))
}

/// Get day of week (0=Sunday, 6=Saturday) for a date
pub fn day_of_week(date: &str) -> Option<u32> {
    let (year, month, day) = parse_date(date)?;
    // Zeller's formula for Gregorian calendar - use i32 for all calculations
    let m: i32 = if month < 3 {
        month as i32 + 12
    } else {
        month as i32
    };
    let y: i32 = if month < 3 { year - 1 } else { year };
    let q: i32 = day as i32;
    let k: i32 = y % 100;
    let j: i32 = y / 100;

    let h = (q + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    // Convert from Zeller (0=Saturday) to standard (0=Sunday)
    Some(((h + 6) % 7) as u32)
}

/// Add days to a date string
pub fn add_days(date: &str, days: i32) -> String {
    let (year, month, day) = parse_date(date).unwrap_or((1970, 1, 1));
    let js_date = js_sys::Date::new_0();
    js_date.set_full_year(year as u32);
    js_date.set_month(month - 1);
    js_date.set_date(day);
    js_date.set_date((js_date.get_date() as i32 + days) as u32);

    format!(
        "{:04}-{:02}-{:02}",
        js_date.get_full_year(),
        js_date.get_month() + 1,
        js_date.get_date()
    )
}

/// Get start of week (Monday) for a date
pub fn start_of_week(date: &str) -> String {
    let dow = day_of_week(date).unwrap_or(0);
    // Convert to Monday=0 based
    let days_since_monday = if dow == 0 { 6 } else { dow - 1 };
    add_days(date, -(days_since_monday as i32))
}

/// Get start of month for a date
pub fn start_of_month(date: &str) -> String {
    let (year, month, _) = parse_date(date).unwrap_or((1970, 1, 1));
    format!("{:04}-{:02}-01", year, month)
}

/// Get end of month for a date
pub fn end_of_month(date: &str) -> String {
    let (year, month, _) = parse_date(date).unwrap_or((1970, 1, 1));
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };
    add_days(&format!("{:04}-{:02}-01", next_year, next_month), -1)
}

/// Format time from HH:MM to human readable
pub fn format_time(time: &str) -> String {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() < 2 {
        return time.to_string();
    }
    let hour: u32 = parts[0].parse().unwrap_or(0);
    let minute: u32 = parts[1].parse().unwrap_or(0);
    let period = if hour >= 12 { "PM" } else { "AM" };
    let display_hour = if hour == 0 {
        12
    } else if hour > 12 {
        hour - 12
    } else {
        hour
    };
    format!("{}:{:02} {}", display_hour, minute, period)
}

/// Add minutes to time string HH:MM
pub fn add_minutes(time: &str, minutes: i32) -> String {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() < 2 {
        return time.to_string();
    }
    let hour: i32 = parts[0].parse().unwrap_or(0);
    let min: i32 = parts[1].parse().unwrap_or(0);

    let total_mins = hour * 60 + min + minutes;
    let new_hour = (total_mins / 60) % 24;
    let new_min = total_mins % 60;

    format!("{:02}:{:02}", new_hour, new_min)
}

/// Parse time string to minutes since midnight
pub fn time_to_minutes(time: &str) -> i32 {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() < 2 {
        return 0;
    }
    let hour: i32 = parts[0].parse().unwrap_or(0);
    let min: i32 = parts[1].parse().unwrap_or(0);
    hour * 60 + min
}

/// Get month name
pub fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

/// Get day name
pub fn day_name(day: u32) -> &'static str {
    match day {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}

/// Get short day name
#[allow(dead_code)]
pub fn day_name_short(day: u32) -> &'static str {
    match day {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        6 => "Sat",
        _ => "?",
    }
}

/// Add CORS headers to a response
pub fn add_cors_headers(
    mut response: Response,
    origin: Option<&str>,
    allowed_origins: &[String],
) -> Response {
    if let Some(origin) = origin {
        if is_origin_allowed(origin, allowed_origins) {
            let headers = response.headers_mut();
            let _ = headers.set("Access-Control-Allow-Origin", origin);
            let _ = headers.set("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
            let _ = headers.set(
                "Access-Control-Allow-Headers",
                "Content-Type, HX-Request, HX-Target, HX-Current-URL, HX-Trigger",
            );
            let _ = headers.set("Access-Control-Allow-Credentials", "true");
            let _ = headers.set("Vary", "Origin");
        }
    }
    response
}

/// Interpolate template variables like {{field_name}} with values
pub fn interpolate_template(
    template: &str,
    fields: &serde_json::Map<String, serde_json::Value>,
) -> String {
    let mut result = template.to_string();
    for (key, value) in fields {
        let placeholder = format!("{{{{{}}}}}", key);
        let replacement = match value {
            serde_json::Value::String(s) => s.clone(),
            _ => value.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}
