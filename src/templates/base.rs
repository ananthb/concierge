//! Base HTML wrappers and CSS handling

use crate::helpers::html_escape;
use crate::types::CalendarStyle;

use super::HASH;

/// Generate timezone select options
pub fn timezone_options(selected: &str) -> String {
    const TIMEZONES: &[(&str, &str)] = &[
        // UTC
        ("UTC", "UTC"),
        // Americas
        ("America/Adak", "America/Adak (HST)"),
        ("America/Anchorage", "America/Anchorage (AKST)"),
        ("America/Boise", "America/Boise (MST)"),
        ("America/Chicago", "America/Chicago (CST)"),
        ("America/Denver", "America/Denver (MST)"),
        ("America/Detroit", "America/Detroit (EST)"),
        ("America/Edmonton", "America/Edmonton (MST)"),
        ("America/Halifax", "America/Halifax (AST)"),
        ("America/Indiana/Indianapolis", "America/Indianapolis (EST)"),
        ("America/Los_Angeles", "America/Los Angeles (PST)"),
        ("America/New_York", "America/New York (EST)"),
        ("America/Phoenix", "America/Phoenix (MST)"),
        ("America/Regina", "America/Regina (CST)"),
        ("America/St_Johns", "America/St Johns (NST)"),
        ("America/Toronto", "America/Toronto (EST)"),
        ("America/Vancouver", "America/Vancouver (PST)"),
        ("America/Winnipeg", "America/Winnipeg (CST)"),
        (
            "America/Argentina/Buenos_Aires",
            "America/Buenos Aires (ART)",
        ),
        ("America/Bogota", "America/Bogota (COT)"),
        ("America/Caracas", "America/Caracas (VET)"),
        ("America/Guatemala", "America/Guatemala (CST)"),
        ("America/Havana", "America/Havana (CST)"),
        ("America/Lima", "America/Lima (PET)"),
        ("America/Mexico_City", "America/Mexico City (CST)"),
        ("America/Montevideo", "America/Montevideo (UYT)"),
        ("America/Panama", "America/Panama (EST)"),
        ("America/Santiago", "America/Santiago (CLT)"),
        ("America/Sao_Paulo", "America/Sao Paulo (BRT)"),
        // Europe
        ("Europe/Amsterdam", "Europe/Amsterdam (CET)"),
        ("Europe/Athens", "Europe/Athens (EET)"),
        ("Europe/Belgrade", "Europe/Belgrade (CET)"),
        ("Europe/Berlin", "Europe/Berlin (CET)"),
        ("Europe/Brussels", "Europe/Brussels (CET)"),
        ("Europe/Bucharest", "Europe/Bucharest (EET)"),
        ("Europe/Budapest", "Europe/Budapest (CET)"),
        ("Europe/Copenhagen", "Europe/Copenhagen (CET)"),
        ("Europe/Dublin", "Europe/Dublin (GMT)"),
        ("Europe/Helsinki", "Europe/Helsinki (EET)"),
        ("Europe/Istanbul", "Europe/Istanbul (TRT)"),
        ("Europe/Kyiv", "Europe/Kyiv (EET)"),
        ("Europe/Lisbon", "Europe/Lisbon (WET)"),
        ("Europe/London", "Europe/London (GMT)"),
        ("Europe/Madrid", "Europe/Madrid (CET)"),
        ("Europe/Moscow", "Europe/Moscow (MSK)"),
        ("Europe/Oslo", "Europe/Oslo (CET)"),
        ("Europe/Paris", "Europe/Paris (CET)"),
        ("Europe/Prague", "Europe/Prague (CET)"),
        ("Europe/Rome", "Europe/Rome (CET)"),
        ("Europe/Stockholm", "Europe/Stockholm (CET)"),
        ("Europe/Vienna", "Europe/Vienna (CET)"),
        ("Europe/Warsaw", "Europe/Warsaw (CET)"),
        ("Europe/Zurich", "Europe/Zurich (CET)"),
        // Asia
        ("Asia/Almaty", "Asia/Almaty (ALMT)"),
        ("Asia/Amman", "Asia/Amman (EET)"),
        ("Asia/Baghdad", "Asia/Baghdad (AST)"),
        ("Asia/Baku", "Asia/Baku (AZT)"),
        ("Asia/Bangkok", "Asia/Bangkok (ICT)"),
        ("Asia/Beirut", "Asia/Beirut (EET)"),
        ("Asia/Colombo", "Asia/Colombo (IST)"),
        ("Asia/Damascus", "Asia/Damascus (EET)"),
        ("Asia/Dhaka", "Asia/Dhaka (BST)"),
        ("Asia/Dubai", "Asia/Dubai (GST)"),
        ("Asia/Ho_Chi_Minh", "Asia/Ho Chi Minh (ICT)"),
        ("Asia/Hong_Kong", "Asia/Hong Kong (HKT)"),
        ("Asia/Jakarta", "Asia/Jakarta (WIB)"),
        ("Asia/Jerusalem", "Asia/Jerusalem (IST)"),
        ("Asia/Kabul", "Asia/Kabul (AFT)"),
        ("Asia/Karachi", "Asia/Karachi (PKT)"),
        ("Asia/Kathmandu", "Asia/Kathmandu (NPT)"),
        ("Asia/Kolkata", "Asia/Kolkata (IST)"),
        ("Asia/Kuala_Lumpur", "Asia/Kuala Lumpur (MYT)"),
        ("Asia/Kuwait", "Asia/Kuwait (AST)"),
        ("Asia/Manila", "Asia/Manila (PHT)"),
        ("Asia/Muscat", "Asia/Muscat (GST)"),
        ("Asia/Riyadh", "Asia/Riyadh (AST)"),
        ("Asia/Seoul", "Asia/Seoul (KST)"),
        ("Asia/Shanghai", "Asia/Shanghai (CST)"),
        ("Asia/Singapore", "Asia/Singapore (SGT)"),
        ("Asia/Taipei", "Asia/Taipei (CST)"),
        ("Asia/Tashkent", "Asia/Tashkent (UZT)"),
        ("Asia/Tehran", "Asia/Tehran (IRST)"),
        ("Asia/Tokyo", "Asia/Tokyo (JST)"),
        ("Asia/Yangon", "Asia/Yangon (MMT)"),
        // Africa
        ("Africa/Abidjan", "Africa/Abidjan (GMT)"),
        ("Africa/Accra", "Africa/Accra (GMT)"),
        ("Africa/Addis_Ababa", "Africa/Addis Ababa (EAT)"),
        ("Africa/Algiers", "Africa/Algiers (CET)"),
        ("Africa/Cairo", "Africa/Cairo (EET)"),
        ("Africa/Casablanca", "Africa/Casablanca (WET)"),
        ("Africa/Johannesburg", "Africa/Johannesburg (SAST)"),
        ("Africa/Lagos", "Africa/Lagos (WAT)"),
        ("Africa/Nairobi", "Africa/Nairobi (EAT)"),
        ("Africa/Tunis", "Africa/Tunis (CET)"),
        // Australia & Pacific
        ("Australia/Adelaide", "Australia/Adelaide (ACST)"),
        ("Australia/Brisbane", "Australia/Brisbane (AEST)"),
        ("Australia/Darwin", "Australia/Darwin (ACST)"),
        ("Australia/Hobart", "Australia/Hobart (AEST)"),
        ("Australia/Melbourne", "Australia/Melbourne (AEST)"),
        ("Australia/Perth", "Australia/Perth (AWST)"),
        ("Australia/Sydney", "Australia/Sydney (AEST)"),
        ("Pacific/Auckland", "Pacific/Auckland (NZST)"),
        ("Pacific/Fiji", "Pacific/Fiji (FJT)"),
        ("Pacific/Guam", "Pacific/Guam (ChST)"),
        ("Pacific/Honolulu", "Pacific/Honolulu (HST)"),
        ("Pacific/Majuro", "Pacific/Majuro (MHT)"),
        ("Pacific/Midway", "Pacific/Midway (SST)"),
        ("Pacific/Noumea", "Pacific/Noumea (NCT)"),
        ("Pacific/Pago_Pago", "Pacific/Pago Pago (SST)"),
        ("Pacific/Port_Moresby", "Pacific/Port Moresby (PGT)"),
        ("Pacific/Tahiti", "Pacific/Tahiti (TAHT)"),
        ("Pacific/Tongatapu", "Pacific/Tongatapu (TOT)"),
        // Atlantic
        ("Atlantic/Azores", "Atlantic/Azores (AZOT)"),
        ("Atlantic/Bermuda", "Atlantic/Bermuda (AST)"),
        ("Atlantic/Canary", "Atlantic/Canary (WET)"),
        ("Atlantic/Cape_Verde", "Atlantic/Cape Verde (CVT)"),
        ("Atlantic/Reykjavik", "Atlantic/Reykjavik (GMT)"),
        // Indian
        ("Indian/Maldives", "Indian/Maldives (MVT)"),
        ("Indian/Mauritius", "Indian/Mauritius (MUT)"),
    ];

    TIMEZONES
        .iter()
        .map(|(value, label)| {
            let sel = if *value == selected { " selected" } else { "" };
            format!("<option value=\"{}\"{}>{}</option>", value, sel, label)
        })
        .collect::<Vec<_>>()
        .join("\n                        ")
}

/// Base HTML wrapper for admin pages
pub fn base_html(title: &str, content: &str, style: &CalendarStyle) -> String {
    format!(
        "<!DOCTYPE html>
<html lang=\"en\">
<head>
    <meta charset=\"UTF-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
    <title>{title}</title>
    <link rel=\"icon\" type=\"image/svg+xml\" href=\"/logo.svg\">
    <script src=\"https://unpkg.com/htmx.org@1.9.10\"></script>
    <style>
        :root {{
            --cal-primary: {primary};
            --cal-text: {text};
            --cal-bg: {bg};
            --cal-border-radius: {radius};
            --cal-font: {font};
            --bg-card: white;
            --bg-muted: {hash}f8f9fa;
            --border: {hash}ddd;
            --text-muted: {hash}666;
            --success-bg: {hash}d4edda;
            --success-text: {hash}155724;
            --error-bg: {hash}f8d7da;
            --error-text: {hash}721c24;
            --warning-bg: {hash}fff3cd;
            --warning-text: {hash}856404;
            --warning-border: {hash}ffc107;
            --code-bg: {hash}e9ecef;
        }}
        @media (prefers-color-scheme: dark) {{
            :root {{
                --cal-bg: {hash}1a1a1a;
                --cal-text: {hash}e0e0e0;
                --cal-primary: {hash}3b9eff;
                --bg-card: {hash}2d2d2d;
                --bg-muted: {hash}3a3a3a;
                --border: {hash}444;
                --text-muted: {hash}999;
                --success-bg: {hash}1e3a2f;
                --success-text: {hash}6fcf97;
                --error-bg: {hash}3a1e1e;
                --error-text: {hash}f5a5a5;
                --warning-bg: {hash}3a351e;
                --warning-text: {hash}f5d56f;
                --warning-border: {hash}7a6a20;
                --code-bg: {hash}3a3a3a;
            }}
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: var(--cal-font);
            color: var(--cal-text);
            background: var(--cal-bg);
            line-height: 1.5;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; padding: 1rem; }}
        a {{ color: var(--cal-primary); }}
        .btn {{
            display: inline-block;
            padding: 0.5rem 1rem;
            background: var(--cal-primary);
            color: white;
            border: none;
            border-radius: var(--cal-border-radius);
            cursor: pointer;
            text-decoration: none;
            font-size: 0.875rem;
            font-family: inherit;
            line-height: 1.2;
            vertical-align: middle;
            box-sizing: border-box;
        }}
        .btn:hover {{ opacity: 0.9; }}
        .btn-secondary {{
            background: {hash}6c757d;
        }}
        .btn-danger {{
            background: {hash}dc3545;
        }}
        .btn-sm {{
            padding: 0.25rem 0.5rem;
            font-size: 0.75rem;
        }}
        .form-group {{
            margin-bottom: 1rem;
        }}
        .form-group label {{
            display: block;
            margin-bottom: 0.25rem;
            font-weight: 500;
        }}
        .form-group input,
        .form-group select,
        .form-group textarea {{
            width: 100%;
            padding: 0.5rem;
            border: 1px solid var(--border);
            border-radius: var(--cal-border-radius);
            font-size: 1rem;
            background: var(--bg-card);
            color: var(--cal-text);
        }}
        .form-group input:focus,
        .form-group select:focus,
        .form-group textarea:focus {{
            outline: none;
            border-color: var(--cal-primary);
        }}
        .card {{
            background: var(--bg-card);
            border: 1px solid var(--border);
            border-radius: var(--cal-border-radius);
            padding: 1rem;
            margin-bottom: 1rem;
        }}
        .success {{
            background: var(--success-bg);
            color: var(--success-text);
            padding: 1rem;
            border-radius: var(--cal-border-radius);
            margin-bottom: 1rem;
        }}
        .error {{
            background: var(--error-bg);
            color: var(--error-text);
            padding: 1rem;
            border-radius: var(--cal-border-radius);
            margin-bottom: 1rem;
        }}
        .htmx-indicator {{
            display: none;
        }}
        .htmx-request .htmx-indicator {{
            display: inline;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
        }}
        th, td {{
            padding: 0.5rem;
            text-align: left;
            border-bottom: 1px solid var(--border);
        }}
        th {{
            background: var(--bg-muted);
        }}
        code {{
            background: var(--code-bg);
            padding: 0.2rem 0.4rem;
            border-radius: 3px;
        }}
        small, .text-muted {{
            color: var(--text-muted);
        }}
        .url-cell {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        .url-cell code {{
            flex: 1;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }}
        .btn-copy {{
            padding: 0.25rem 0.5rem;
            font-size: 0.7rem;
            background: {hash}6c757d;
            white-space: nowrap;
        }}
        .btn-copy.copied {{
            background: {hash}28a745;
        }}
    </style>
</head>
<body>
    <div class=\"container\">
        {content}
    </div>
    <script>
        function copyUrl(btn, url) {{
            navigator.clipboard.writeText(url).then(function() {{
                btn.textContent = 'Copied!';
                btn.classList.add('copied');
                setTimeout(function() {{
                    btn.textContent = 'Copy';
                    btn.classList.remove('copied');
                }}, 2000);
            }});
        }}
    </script>
</body>
</html>",
        title = html_escape(title),
        content = content,
        primary = html_escape(&style.primary_color),
        text = html_escape(&style.text_color),
        bg = html_escape(&style.background_color),
        radius = html_escape(&style.border_radius),
        font = html_escape(&style.font_family),
        hash = HASH,
    )
}

/// CSS options for public templates
#[derive(Default)]
pub struct CssOptions<'a> {
    pub inline_css: Option<&'a str>,
    pub css_url: Option<&'a str>,
}


/// Available responder channels based on configured secrets
#[derive(Default)]
pub struct AvailableChannels {
    pub twilio_sms: bool,
    pub twilio_whatsapp: bool,
    pub twilio_email: bool,
    pub resend_email: bool,
}

/// Base HTML wrapper with custom CSS support for public pages
pub fn base_html_with_css(
    title: &str,
    content: &str,
    style: &CalendarStyle,
    css: &CssOptions,
) -> String {
    let css_link = css
        .css_url
        .map(|url| format!("<link rel=\"stylesheet\" href=\"{}\">", html_escape(url)))
        .unwrap_or_default();
    let query_css = css.inline_css.unwrap_or_default();

    format!(
        "<!DOCTYPE html>
<html lang=\"en\">
<head>
    <meta charset=\"UTF-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
    <title>{title}</title>
    <link rel=\"icon\" type=\"image/svg+xml\" href=\"/logo.svg\">
    {css_link}
    <script src=\"https://unpkg.com/htmx.org@1.9.10\"></script>
    <style>
        :root {{
            --cal-primary: {primary};
            --cal-text: {text};
            --cal-bg: {bg};
            --cal-border-radius: {radius};
            --cal-font: {font};
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: var(--cal-font);
            color: var(--cal-text);
            background: var(--cal-bg);
            line-height: 1.5;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; padding: 1rem; }}
        .btn {{
            display: inline-block;
            padding: 0.5rem 1rem;
            background: var(--cal-primary);
            color: white;
            border: none;
            border-radius: var(--cal-border-radius);
            cursor: pointer;
            text-decoration: none;
            font-size: 0.875rem;
            font-family: inherit;
            line-height: 1.2;
            vertical-align: middle;
            box-sizing: border-box;
        }}
        .btn:hover {{ opacity: 0.9; }}
        .btn-secondary {{
            background: {hash}6c757d;
        }}
        .btn-danger {{
            background: {hash}dc3545;
        }}
        .form-group {{
            margin-bottom: 1rem;
        }}
        .form-group label {{
            display: block;
            margin-bottom: 0.25rem;
            font-weight: 500;
        }}
        .form-group input,
        .form-group select,
        .form-group textarea {{
            width: 100%;
            padding: 0.5rem;
            border: 1px solid {hash}ddd;
            border-radius: var(--cal-border-radius);
            font-size: 1rem;
        }}
        .form-group input:focus,
        .form-group select:focus,
        .form-group textarea:focus {{
            outline: none;
            border-color: var(--cal-primary);
        }}
        .card {{
            background: white;
            border: 1px solid {hash}ddd;
            border-radius: var(--cal-border-radius);
            padding: 1rem;
            margin-bottom: 1rem;
        }}
        .success {{
            background: {hash}d4edda;
            color: {hash}155724;
            padding: 1rem;
            border-radius: var(--cal-border-radius);
            margin-bottom: 1rem;
        }}
        .error {{
            background: {hash}f8d7da;
            color: {hash}721c24;
            padding: 1rem;
            border-radius: var(--cal-border-radius);
            margin-bottom: 1rem;
        }}
        .htmx-indicator {{
            display: none;
        }}
        .htmx-request .htmx-indicator {{
            display: inline;
        }}
        {custom_css}
        {query_css}
    </style>
</head>
<body>
    <div class=\"container\">
        {content}
    </div>
</body>
</html>",
        title = html_escape(title),
        content = content,
        primary = html_escape(&style.primary_color),
        text = html_escape(&style.text_color),
        bg = html_escape(&style.background_color),
        radius = html_escape(&style.border_radius),
        font = html_escape(&style.font_family),
        custom_css = style.custom_css,
        css_link = css_link,
        query_css = query_css,
        hash = HASH,
    )
}

/// Fragment HTML for HTMX requests - returns just the content
/// Styles are already on the page from the initial full page load
fn fragment_html_with_css(content: &str, _style: &CalendarStyle, _css: &CssOptions) -> String {
    content.to_string()
}

/// Wrap content with full page or fragment based on HTMX request
pub fn wrap_html(
    content: &str,
    title: &str,
    style: &CalendarStyle,
    css: &CssOptions,
    is_htmx: bool,
) -> String {
    if is_htmx {
        fragment_html_with_css(content, style, css)
    } else {
        base_html_with_css(title, content, style, css)
    }
}
