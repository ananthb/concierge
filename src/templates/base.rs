//! Base HTML wrappers and CSS handling

use crate::helpers::html_escape;

use super::HASH;

/// Minimal style struct for base HTML rendering
pub struct BaseStyle {
    pub primary_color: String,
    pub text_color: String,
    pub background_color: String,
    pub border_radius: String,
    pub font_family: String,
}

impl Default for BaseStyle {
    fn default() -> Self {
        Self {
            primary_color: String::from("#F38020"),
            text_color: String::from("#333333"),
            background_color: String::from("#ffffff"),
            border_radius: String::from("4px"),
            font_family: String::from("system-ui, -apple-system, sans-serif"),
        }
    }
}

/// Base HTML wrapper for admin pages
pub fn base_html(title: &str, content: &str, style: &BaseStyle) -> String {
    format!(
        "<!DOCTYPE html>
<html lang=\"en\">
<head>
    <meta charset=\"UTF-8\">
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
    <title>{title}</title>
    <link rel=\"icon\" type=\"image/svg+xml\" href=\"/logo.svg\">
    <link rel=\"icon\" type=\"image/png\" sizes=\"32x32\" href=\"/favicon-32.png\">
    <link rel=\"icon\" type=\"image/png\" sizes=\"16x16\" href=\"/favicon-16.png\">
    <link rel=\"apple-touch-icon\" sizes=\"180x180\" href=\"/apple-touch-icon.png\">
    <link rel=\"manifest\" href=\"/site.webmanifest\">
    <meta name=\"msapplication-TileColor\" content=\"#1A1A2E\">
    <meta name=\"theme-color\" content=\"#F38020\">
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
            --code-bg: {hash}e9ecef;
        }}
        @media (prefers-color-scheme: dark) {{
            :root {{
                --cal-bg: {hash}1a1a1a;
                --cal-text: {hash}e0e0e0;
                --cal-primary: {hash}F9A825;
                --bg-card: {hash}2d2d2d;
                --bg-muted: {hash}3a3a3a;
                --border: {hash}444;
                --text-muted: {hash}999;
                --success-bg: {hash}1e3a2f;
                --success-text: {hash}6fcf97;
                --error-bg: {hash}3a1e1e;
                --error-text: {hash}f5a5a5;
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
            background: {hash}1A1A2E;
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
        .btn-copy {{
            padding: 0.25rem 0.5rem;
            font-size: 0.7rem;
            background: {hash}1A1A2E;
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
    <footer style=\"text-align: center; padding: 2rem 1rem; color: var(--text-muted); font-size: 0.8rem;\">
        <a href=\"/terms\" style=\"color: var(--text-muted);\">Terms</a> &middot;
        <a href=\"/privacy\" style=\"color: var(--text-muted);\">Privacy</a> &middot;
        <a href=\"https://ananthb.github.io/concierge-worker/\" style=\"color: var(--text-muted);\">Docs</a> &middot;
        <a href=\"https://github.com/ananthb/concierge-worker\" style=\"color: var(--text-muted);\">Source</a>
    </footer>
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
