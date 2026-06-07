//! Base HTML wrappers: new cream/paper design system

use std::sync::OnceLock;

use base64::Engine;
use sha2::{Digest, Sha384};

use crate::helpers::html_escape;
use crate::i18n::t;
use crate::locale::Locale;

// CSS is served as a static asset by Cloudflare (see `[assets]` in
// wrangler.toml). The bytes are still compiled in so we can derive an
// SRI hash without a build script; the binary doesn't ship them to the
// browser. Replace with build.rs codegen once we have more than a few
// modules.
const BASE_CSS_BYTES: &[u8] = include_bytes!("../../public/css/base.css");

fn base_css_sri() -> &'static str {
    static CACHED: OnceLock<String> = OnceLock::new();
    CACHED.get_or_init(|| {
        let digest = Sha384::digest(BASE_CSS_BYTES);
        let b64 = base64::engine::general_purpose::STANDARD.encode(digest);
        format!("sha384-{b64}")
    })
}

/// Logo inline SVG for use in templates. Always rendered next to a textual
/// brand name, so it's marked decorative for assistive tech.
pub const LOGO_INLINE: &str = r##"<svg width="30" height="30" viewBox="0 0 100 100" style="display:block" aria-hidden="true" focusable="false"><circle cx="50" cy="50" r="48" fill="var(--accent)"/><path d="M28 40a22 22 0 0 1 44 0v20a22 22 0 0 1-44 0" stroke="#FBF7EE" stroke-width="6" fill="none" stroke-linecap="round"/><circle cx="50" cy="52" r="5.5" fill="#FBF7EE"/></svg>"##;

pub fn brand_mark() -> String {
    format!(
        r#"<a href="/" class="brand link-reset">{}<span class="serif">Concierge</span></a>"#,
        LOGO_INLINE
    )
}

/// Shared header for public marketing pages (home, /features, /pricing).
/// `active` is the slug of the current page so the matching nav item
/// lights up: pass "" to highlight nothing.
///
/// "Open source" gets the `nav-ext` class so it's hidden up to 760px;
/// it's duplicated in the footer and shedding it is what lets the row
/// fit a phone-sized viewport. "Docs" lives only in the footer (it's
/// architecture/dev docs, not user-facing help).
pub fn public_nav_html(active: &str, locale: &Locale) -> String {
    let item = |slug: &str, label: &str, href: &str| -> String {
        let (cls, aria_current) = if slug == active {
            ("btn ghost sm is-active", r#" aria-current="page""#)
        } else {
            ("btn ghost sm", "")
        };
        format!(r#"<a href="{href}" class="{cls}"{aria_current}>{label}</a>"#)
    };
    let features = item("features", &t(locale, "nav-features"), "/features");
    let pricing = item("pricing", &t(locale, "nav-pricing"), "/pricing");
    let github_label = t(locale, "nav-open-source");
    let github = format!(
        r#"<a href="https://github.com/ananthb/concierge" class="btn ghost sm nav-ext" target="_blank" rel="noopener">{github_label}</a>"#,
    );
    let signin_label = t(locale, "nav-sign-in");
    let signin = format!(r#"<a href="/auth/login" class="btn primary sm">{signin_label}</a>"#,);
    format!(
        r#"<header class="site-header">
  {brand}
  <nav class="site-nav row gap-8 ml-auto" aria-label="Primary">
    {features}{pricing}{github}{signin}
  </nav>
</header>"#,
        brand = brand_mark(),
        features = features,
        pricing = pricing,
        github = github,
        signin = signin,
    )
}

#[cfg(test)]
mod footer_tests {
    fn count(haystack: &str, needle: &str) -> usize {
        haystack.matches(needle).count()
    }

    #[test]
    fn welcome_has_one_footer() {
        let l = crate::locale::Locale::default_inr();
        let s = crate::templates::onboarding::welcome_html("", &l, true, 3, 30, None);
        assert_eq!(count(&s, r#"<footer class="site-footer">"#), 1, "welcome");
    }

    #[test]
    fn welcome_substitutes_demo_chat_limits() {
        let l = crate::locale::Locale::default_inr();
        let s = crate::templates::onboarding::welcome_html("", &l, true, 5, 45, None);
        assert!(
            !s.contains("__TURN_LIMIT__") && !s.contains("__CTA_TIMEOUT_MS__"),
            "demo chat JS placeholders must be substituted"
        );
        assert!(
            s.contains("const TURN_LIMIT = 5;"),
            "user-turn limit must be threaded into hero chat JS"
        );
        assert!(
            s.contains("const CTA_TIMEOUT_MS = 45000;"),
            "idle timeout must be converted to ms and threaded into hero chat JS"
        );
    }

    /// Verify every FTL key used by the page resolves: `t()` falls back to
    /// the key string on miss, so a passing assertion guarantees the FTL
    /// bundle has every key the template references.
    fn assert_keys_resolved(html: &str, keys: &[&str], page: &str) {
        for key in keys {
            assert!(
                !html.contains(&format!(">{key}<"))
                    && !html.contains(&format!("=\"{key}\""))
                    && !html.contains(&format!(">{key} "))
                    && !html.contains(&format!(" {key}<")),
                "{page}: FTL key {key:?} appears unresolved in rendered HTML"
            );
        }
    }

    #[test]
    fn welcome_resolves_all_keys() {
        let l = crate::locale::Locale::default_inr();
        let s = crate::templates::onboarding::welcome_html("", &l, true, 3, 30, None);
        assert_keys_resolved(
            &s,
            &[
                "welcome-eyebrow",
                "welcome-headline",
                "welcome-headline-2",
                "welcome-headline-3",
                "welcome-headline-4",
                "welcome-headline-5",
                "welcome-lead",
                "welcome-cta-primary",
                "welcome-cta-secondary",
                "demo-chat-hint",
                "demo-chat-title",
                "demo-chat-subtitle",
                "demo-chat-subtitle-concierge",
                "demo-chat-persona-label",
                "demo-chat-roleplay-prefix",
                "demo-chat-roleplay-suffix",
                "demo-chat-channels-note",
                "demo-chat-business-hours",
                "demo-chat-business-city",
                "demo-chat-business-type",
                "demo-chat-business-goal",
                "demo-chat-handoff-chip",
                "demo-chat-view-prompt",
                "demo-chat-hide-prompt",
                "demo-chat-prompt-heading",
                "demo-chat-envelope-note",
                "demo-chat-placeholder",
                "demo-chat-placeholder-customer-prefix",
                "demo-chat-placeholder-customer-suffix",
                "demo-chat-send",
                "demo-chat-close",
                "demo-chat-thinking",
                "demo-chat-error",
                "demo-chat-rate-limited",
            ],
            "welcome",
        );
    }

    #[test]
    fn features_has_one_footer() {
        let l = crate::locale::Locale::default_inr();
        let cfg = crate::storage::Pricing::default();
        let s = crate::templates::features::features_html(&l, &cfg);
        assert_eq!(count(&s, r#"<footer class="site-footer">"#), 1, "features");
        // Also catch any stray <footer> tag with a different class.
        assert_eq!(count(&s, "<footer"), 1, "features any-footer");
    }

    #[test]
    fn pricing_has_one_footer() {
        let l = crate::locale::Locale::default_inr();
        let cfg = crate::storage::Pricing::default();
        let s = crate::templates::onboarding::pricing_html("INR", &l, &cfg);
        assert_eq!(count(&s, r#"<footer class="site-footer">"#), 1, "pricing");
    }

    #[test]
    fn terms_has_one_footer() {
        let l = crate::locale::Locale::default_inr();
        let s = crate::legal::terms_of_service_html(&l);
        assert_eq!(count(&s, r#"<footer class="site-footer">"#), 1, "terms");
    }

    #[test]
    fn privacy_has_one_footer() {
        let l = crate::locale::Locale::default_inr();
        let s = crate::legal::privacy_policy_html(&l);
        assert_eq!(count(&s, r#"<footer class="site-footer">"#), 1, "privacy");
    }

    #[test]
    fn footer_resolves_keys_in_both_locales() {
        for l in [
            crate::locale::Locale::default_inr(),
            crate::locale::Locale::default_usd(),
        ] {
            let s = super::footer(&l);
            assert!(s.contains("Features"), "footer-features in {}", l.langid);
            assert!(
                s.contains("Privacy Policy"),
                "footer-privacy in {}",
                l.langid
            );
        }
    }

    #[test]
    fn html_lang_matches_locale() {
        let inr = crate::locale::Locale::default_inr();
        let usd = crate::locale::Locale::default_usd();
        let s_inr = super::base_html("t", "<p>x</p>", &inr);
        let s_usd = super::base_html("t", "<p>x</p>", &usd);
        assert!(s_inr.contains(r#"<html lang="en-IN">"#));
        assert!(s_usd.contains(r#"<html lang="en-US">"#));
    }
}

/// Shared footer for all pages.
pub fn footer(locale: &Locale) -> String {
    format!(
        r##"<footer class="site-footer">
  <a href="/features" class="muted">{features}</a> &middot;
  <a href="/pricing" class="muted">{pricing}</a> &middot;
  <a href="https://ananthb.github.io/concierge/" class="muted" target="_blank" rel="noopener">{docs}</a> &middot;
  <a href="https://github.com/ananthb/concierge" class="muted">{open_source}</a> &middot;
  <a href="https://www.gnu.org/licenses/agpl-3.0.html" class="muted">{licence}</a> &middot;
  <a href="/terms" class="muted">{terms}</a> &middot;
  <a href="/privacy" class="muted">{privacy}</a>
</footer>"##,
        features = t(locale, "footer-features"),
        pricing = t(locale, "footer-pricing"),
        docs = t(locale, "footer-docs"),
        open_source = t(locale, "footer-open-source"),
        licence = t(locale, "footer-licence"),
        terms = t(locale, "footer-terms"),
        privacy = t(locale, "footer-privacy"),
    )
}

/// OpenGraph / meta description tags for a page. `og_type` stays static
/// (it's an enumerated technical value, not user-facing copy); description
/// and og_title come from translated strings.
pub struct PageMeta {
    pub description: String,
    pub og_title: String,
    pub og_type: &'static str, // "website", "article", etc.
}

impl PageMeta {
    /// Default meta for pages that don't set their own description.
    pub fn default_for(locale: &Locale) -> Self {
        Self {
            description: t(locale, "meta-default-description"),
            og_title: "Concierge".to_string(),
            og_type: "website",
        }
    }
}

/// Base HTML wrapper for all pages.
pub fn base_html(title: &str, content: &str, locale: &Locale) -> String {
    base_html_with_meta(title, content, &PageMeta::default_for(locale), locale)
}

/// Base HTML wrapper with custom meta tags.
pub fn base_html_with_meta(title: &str, content: &str, meta: &PageMeta, locale: &Locale) -> String {
    let lang = locale.langid.to_string();
    let skip_link = t(locale, "app-nav-skip-link");
    let copy_default = t(locale, "js-copy-button-default");
    let copy_copied = t(locale, "js-copy-button-copied");
    let htmx_error = t(locale, "js-htmx-error-toast");
    format!(
        r##"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="{description}">
<meta property="og:title" content="{og_title}">
<meta property="og:description" content="{description}">
<meta property="og:type" content="{og_type}">
<meta property="og:image" content="/logo-192.png">
<meta property="og:image:width" content="192">
<meta property="og:image:height" content="192">
<meta property="og:site_name" content="Concierge">
<meta name="twitter:card" content="summary">
<meta name="twitter:title" content="{og_title}">
<meta name="twitter:description" content="{description}">
<link rel="icon" href="/logo.svg" type="image/svg+xml">
<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32.png">
<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16.png">
<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">
<link rel="manifest" href="/site.webmanifest">
<meta name="theme-color" content="#E86A2C">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
<script src="https://unpkg.com/htmx.org@2.0.8/dist/htmx.min.js" integrity="sha384-/TgkGk7p307TH7EXJDuUlgG3Ce1UVolAOFopFekQkkXihi5u/6OCvVKyz1W+idaz" crossorigin="anonymous"></script>
<script src="https://unpkg.com/htmx-ext-json-enc@2.0.3/json-enc.js" crossorigin="anonymous"></script>
<script src="https://unpkg.com/htmx-ext-sse@2.2.2/sse.js" crossorigin="anonymous"></script>
<script src="https://unpkg.com/@alpinejs/focus@3.14.3/dist/cdn.min.js" defer></script>
<script src="https://unpkg.com/alpinejs@3.14.3/dist/cdn.min.js" defer></script>
<link rel="stylesheet" href="/css/base.css" integrity="{css_sri}" nonce="__CSP_NONCE__">
</head>
<body data-i18n-copy-default="{copy_default}" data-i18n-copy-copied="{copy_copied}" data-i18n-htmx-error="{htmx_error}">
<a href="#main" class="skip-link">{skip_link}</a>
<div class="app-root"><main id="main" class="app-main">{content}</main>{footer}</div>
<script type="module" nonce="__CSP_NONCE__">
// Copy-to-clipboard via delegated click on `<button class="copy-btn"
// data-copy-url="...">`. We used to wire this with inline `onclick=`,
// but that requires `'unsafe-inline'` in script-src; the delegated
// listener works under a strict nonce-only CSP.
document.addEventListener('click', async (event) => {{
  const btn = event.target.closest('.copy-btn');
  if (!btn) return;
  // `data-copy-url` is the legacy attribute for URL-shaped values;
  // `data-copy-text` accepts any string. Either works.
  const text = btn.dataset.copyText || btn.dataset.copyUrl;
  if (!text) return;
  const copied = document.body.dataset.i18nCopyCopied || 'Copied!';
  const def = btn.dataset.copyLabel || document.body.dataset.i18nCopyDefault || 'Copy';
  await navigator.clipboard.writeText(text);
  const prev = btn.textContent;
  btn.textContent = copied;
  const region = document.getElementById('toast-region');
  if (region) {{
    region.insertAdjacentHTML('afterbegin', `<div class="success">${{copied}}</div>`);
  }} else {{
    const toast = document.getElementById('toast');
    if (toast) toast.innerHTML = `<div class="success">${{copied}}</div>`;
  }}
  setTimeout(() => {{ btn.textContent = btn.dataset.copyLabel ? def : prev; }}, 2000);
}});

document.addEventListener('htmx:responseError', () => {{
  const msg = document.body.dataset.i18nHtmxError || 'Something went wrong. Please try again.';
  // Prefer the global /manage toast region; fall back to legacy #toast.
  const region = document.getElementById('toast-region');
  if (region) {{
    region.insertAdjacentHTML('afterbegin', `<div class="error">${{msg}}</div>`);
    return;
  }}
  const toast = document.getElementById('toast');
  if (toast) toast.innerHTML = `<div class="error">${{msg}}</div>`;
}});

// Send CSRF token with all HTMX requests.
document.addEventListener('htmx:configRequest', (e) => {{
  const csrf = document.cookie
    .split(';')
    .map((c) => c.trim())
    .find((c) => c.startsWith('csrf='));
  if (csrf) e.detail.headers['X-CSRF-Token'] = csrf.substring(5);
}});
</script>
</body>
</html>"##,
        lang = html_escape(&lang),
        title = html_escape(title),
        description = html_escape(&meta.description),
        og_title = html_escape(&meta.og_title),
        og_type = meta.og_type,
        skip_link = html_escape(&skip_link),
        copy_default = html_escape(&copy_default),
        copy_copied = html_escape(&copy_copied),
        htmx_error = html_escape(&htmx_error),
        content = content,
        css_sri = base_css_sri(),
        footer = footer(locale),
    )
}

/// Branded "we're temporarily offline" page. Used when essentials are
/// missing: see `handlers::health::essentials_missing`.
pub fn maintenance_html(locale: &Locale) -> String {
    let body = format!(
        r##"<header class="site-header">{brand}</header>
<section class="page narrow ta-center">
  <h1 class="display" style="margin-top:2rem">{headline}</h1>
  <p class="lead" style="margin:0 auto 1.5rem;max-width:520px">{body_text}</p>
  <p class="muted fs-13">{tail}</p>
</section>"##,
        brand = brand_mark(),
        headline = t(locale, "maintenance-headline"),
        body_text = t(locale, "maintenance-body"),
        tail = t(locale, "maintenance-tail"),
    );
    base_html(&t(locale, "maintenance-title"), &body, locale)
}

/// Wrap content in the app shell (top nav + main area). The shell does
/// NOT wrap `content` in `<main>` because callers may already have one;
/// the surrounding `base_html` provides the `<main>` landmark.
pub fn app_shell(content: &str, active_nav: &str, base_url: &str, locale: &Locale) -> String {
    // Each entry: (active_key, FTL key, href).
    // active_key matches the `active_nav` arg (kept as English for stable
    // cross-locale routing; callers don't have to translate it too).
    // Account-level entries (Settings, Sign out) live in the avatar
    // dropdown on the right — they're per-user, not per-feature.
    let nav_items: [(&str, &str, &str); 4] = [
        ("Overview", "app-nav-overview", "/dashboard"),
        ("Approvals", "app-nav-approvals", "/dashboard/approvals"),
        ("Channels", "app-nav-channels", "/dashboard/channels"),
        ("Billing", "app-nav-billing", "/dashboard/billing"),
    ];

    let nav: String = nav_items
        .iter()
        .map(|(slug, key, href)| {
            let class = if *slug == active_nav { " active" } else { "" };
            let label = t(locale, key);
            format!(r#"<a class="{class}" href="{base_url}{href}">{label}</a>"#)
        })
        .collect();

    let nav_aria = t(locale, "app-nav-aria-label");
    let status = t(locale, "app-nav-status-live");
    let menu_aria = t(locale, "app-account-menu-aria");
    let menu_settings = t(locale, "app-account-menu-settings");
    let menu_signout = t(locale, "app-account-menu-signout");

    format!(
        r##"<div class="app">
  <header class="app-top">
    {brand}
    <nav class="app-nav" aria-label="{nav_aria}">{nav}</nav>
    <div class="row gap-12">
      <span class="chip ok">{status}</span>
      <div class="acct-menu" x-data="{{ open: false }}" @keydown.escape.window="open=false" @click.outside="open=false">
        <button type="button" class="avatar" @click="open=!open" :aria-expanded="open" aria-haspopup="menu" aria-label="{menu_aria}">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true"><circle cx="12" cy="8" r="3.4" stroke="currentColor" stroke-width="1.6"/><path d="M5 19c1.6-3 4.2-4.5 7-4.5s5.4 1.5 7 4.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
        </button>
        <div class="acct-pop" x-show="open" x-cloak x-transition.opacity role="menu" aria-label="{menu_aria}">
          <a href="{base_url}/dashboard/settings" role="menuitem">{menu_settings}</a>
          <a href="{base_url}/auth/logout" role="menuitem">{menu_signout}</a>
        </div>
      </div>
    </div>
  </header>
  {content}
</div>"##,
        brand = brand_mark(),
        nav = nav,
        nav_aria = html_escape(&nav_aria),
        status = html_escape(&status),
        menu_aria = html_escape(&menu_aria),
        menu_settings = html_escape(&menu_settings),
        menu_signout = html_escape(&menu_signout),
        base_url = base_url,
        content = content,
    )
}

/// Standard empty state. `cta` is `(href, label)`.
pub fn empty_state(headline: &str, subtext: &str, cta: Option<(&str, &str)>) -> String {
    let cta_html = match cta {
        Some((href, label)) => format!(
            r#"<div class="empty-cta"><a class="btn sm" href="{href}">{label}</a></div>"#,
            href = href,
            label = html_escape(label),
        ),
        None => String::new(),
    };
    format!(
        r##"<div class="empty-state">
  <p class="empty-headline">{headline}</p>
  <p class="empty-sub">{subtext}</p>
  {cta}
</div>"##,
        headline = html_escape(headline),
        subtext = html_escape(subtext),
        cta = cta_html,
    )
}
