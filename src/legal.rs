//! Legal pages: Terms of Service and Privacy Policy

use crate::i18n::t;
use crate::locale::Locale;
use crate::templates::base::{base_html_with_meta, brand_mark, PageMeta};

pub fn terms_of_service_html(locale: &Locale) -> String {
    let content = format!(
        r##"<header class="site-header">
  {brand}
  <div style="margin-left:auto"><a href="/" class="btn ghost sm">{home}</a></div>
</header>
<article class="legal">
  <h1>{h1}</h1>
  <p class="muted">{effective}</p>

  <h2>{h2_service}</h2>
  <p>{p_service}</p>

  <h2>{h2_accounts}</h2>
  <p>{p_accounts}</p>

  <h2>{h2_acceptable}</h2>
  <p>{p_acceptable}</p>

  <h2>{h2_data}</h2>
  <p>{p_data}</p>

  <h2>{h2_warranty}</h2>
  <p>{p_warranty}</p>

  <h2>{h2_ai}</h2>
  <p>{p_ai_1}</p>
  <p>{p_ai_2}</p>

  <h2>{h2_liability}</h2>
  <p>{p_liability}</p>

  <h2>{h2_changes}</h2>
  <p>{p_changes}</p>

  <h2>{h2_contact}</h2>
  <p>{p_contact}</p>
</article>"##,
        brand = brand_mark(),
        home = t(locale, "legal-back-home"),
        h1 = t(locale, "terms-h1"),
        effective = t(locale, "terms-effective"),
        h2_service = t(locale, "terms-h2-service"),
        p_service = t(locale, "terms-p-service"),
        h2_accounts = t(locale, "terms-h2-accounts"),
        p_accounts = t(locale, "terms-p-accounts"),
        h2_acceptable = t(locale, "terms-h2-acceptable"),
        p_acceptable = t(locale, "terms-p-acceptable"),
        h2_data = t(locale, "terms-h2-data"),
        p_data = t(locale, "terms-p-data"),
        h2_warranty = t(locale, "terms-h2-warranty"),
        p_warranty = t(locale, "terms-p-warranty"),
        h2_ai = t(locale, "terms-h2-ai"),
        p_ai_1 = t(locale, "terms-p-ai-1"),
        p_ai_2 = t(locale, "terms-p-ai-2"),
        h2_liability = t(locale, "terms-h2-liability"),
        p_liability = t(locale, "terms-p-liability"),
        h2_changes = t(locale, "terms-h2-changes"),
        p_changes = t(locale, "terms-p-changes"),
        h2_contact = t(locale, "terms-h2-contact"),
        p_contact = t(locale, "terms-p-contact"),
    );

    let title = t(locale, "terms-title");
    base_html_with_meta(
        &title,
        &content,
        &PageMeta {
            description: t(locale, "terms-meta-description"),
            og_title: title.clone(),
            og_type: "article",
        },
        locale,
    )
}

pub fn privacy_policy_html(locale: &Locale) -> String {
    let content = format!(
        r##"<header class="site-header">
  {brand}
  <div style="margin-left:auto"><a href="/" class="btn ghost sm">{home}</a></div>
</header>
<article class="legal">
  <h1>{h1}</h1>
  <p class="muted">{effective}</p>

  <h2>{h2_collect}</h2>
  <ul>
    <li>{li_account}</li>
    <li>{li_connected}</li>
    <li>{li_logs}</li>
    <li>{li_leads}</li>
    <li>{li_persona}</li>
  </ul>

  <h2>{h2_use}</h2>
  <p>{p_use}</p>

  <h2>{h2_storage}</h2>
  <p>{p_storage}</p>

  <h2>{h2_ai}</h2>
  <p>{p_ai}</p>

  <h2>{h2_third}</h2>
  <p>{p_third}</p>

  <h2>{h2_google_limited}</h2>
  <p>{p_google_limited_prefix}</p>
  <ul>
    <li>{li_google_limited_1}</li>
    <li>{li_google_limited_2}</li>
    <li>{li_google_limited_3}</li>
    <li>{li_google_limited_4}</li>
    <li>{li_google_limited_5}</li>
    <li>{li_google_limited_6}</li>
  </ul>

  <h2>{h2_retention}</h2>
  <p>{p_retention}</p>

  <h2>{h2_deletion}</h2>
  <p>{p_deletion_prefix}</p>
  <ul>
    <li>{li_deletion_1}</li>
    <li>{li_deletion_2}</li>
  </ul>
  <p>{p_deletion_suffix}</p>

  <h2>{h2_contact}</h2>
  <p>{p_contact}</p>
</article>"##,
        brand = brand_mark(),
        home = t(locale, "legal-back-home"),
        h1 = t(locale, "privacy-h1"),
        effective = t(locale, "privacy-effective"),
        h2_collect = t(locale, "privacy-h2-collect"),
        li_account = t(locale, "privacy-li-account"),
        li_connected = t(locale, "privacy-li-connected"),
        li_logs = t(locale, "privacy-li-logs"),
        li_leads = t(locale, "privacy-li-leads"),
        li_persona = t(locale, "privacy-li-persona"),
        h2_use = t(locale, "privacy-h2-use"),
        p_use = t(locale, "privacy-p-use"),
        h2_storage = t(locale, "privacy-h2-storage"),
        p_storage = t(locale, "privacy-p-storage"),
        h2_ai = t(locale, "privacy-h2-ai"),
        p_ai = t(locale, "privacy-p-ai"),
        h2_third = t(locale, "privacy-h2-third"),
        p_third = t(locale, "privacy-p-third"),
        h2_google_limited = t(locale, "privacy-h2-google-limited"),
        p_google_limited_prefix = t(locale, "privacy-p-google-limited-prefix"),
        li_google_limited_1 = t(locale, "privacy-li-google-limited-1"),
        li_google_limited_2 = t(locale, "privacy-li-google-limited-2"),
        li_google_limited_3 = t(locale, "privacy-li-google-limited-3"),
        li_google_limited_4 = t(locale, "privacy-li-google-limited-4"),
        li_google_limited_5 = t(locale, "privacy-li-google-limited-5"),
        li_google_limited_6 = t(locale, "privacy-li-google-limited-6"),
        h2_retention = t(locale, "privacy-h2-retention"),
        p_retention = t(locale, "privacy-p-retention"),
        h2_deletion = t(locale, "privacy-h2-deletion"),
        p_deletion_prefix = t(locale, "privacy-p-deletion-prefix"),
        li_deletion_1 = t(locale, "privacy-li-deletion-1"),
        li_deletion_2 = t(locale, "privacy-li-deletion-2"),
        p_deletion_suffix = t(locale, "privacy-p-deletion-suffix"),
        h2_contact = t(locale, "privacy-h2-contact"),
        p_contact = t(locale, "privacy-p-contact"),
    );

    let title = t(locale, "privacy-title");
    base_html_with_meta(
        &title,
        &content,
        &PageMeta {
            description: t(locale, "privacy-meta-description"),
            og_title: title.clone(),
            og_type: "article",
        },
        locale,
    )
}
