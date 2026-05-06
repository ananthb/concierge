//! Admin templates for the email feature: address list and per-address
//! edit (auto-reply config + notification recipients).

use crate::helpers::html_escape;
use crate::i18n::t;
use crate::locale::Locale;
use crate::types::*;

use super::base::{app_shell, base_html};
use super::HASH;

/// Top-level email dashboard.
pub fn email_dashboard_html(
    addrs: &[EmailAddress],
    tenant: &Tenant,
    base_domain: &str,
    base_url: &str,
    locale: &Locale,
) -> String {
    let used = addrs.len() as u32;
    let quota = tenant.email_address_quota();
    let at_limit = used >= quota;

    let mode_static = t(locale, "admin-email-mode-static");
    let mode_ai = t(locale, "admin-email-mode-ai");
    let on_label = t(locale, "admin-email-on");
    let off_label = t(locale, "admin-email-off");
    let recip_suffix = t(locale, "admin-email-recipients-suffix");
    let edit_label = t(locale, "admin-email-row-edit");
    let delete_label = t(locale, "admin-email-row-delete");
    let delete_confirm = html_escape(&t(locale, "admin-email-delete-confirm"));
    let address_rows: String = addrs
        .iter()
        .map(|a| {
            let full = format!("{}@{}", a.local_part, base_domain);
            let mode_label = if a.auto_reply.default_is_canned() {
                &mode_static
            } else {
                &mode_ai
            };
            let on_off = if a.auto_reply.enabled {
                format!(r#"<span class="chip ok">{on_label}</span>"#)
            } else {
                format!(r#"<span class="chip">{off_label}</span>"#)
            };
            format!(
                r#"<tr>
                    <td><a href="{base_url}/dashboard/email/addresses/{label}" class="link-reset"><strong>{full}</strong></a></td>
                    <td>{mode_label} {on_off}</td>
                    <td>{count} {recip_suffix}</td>
                    <td class="ta-right">
                        <a href="{base_url}/dashboard/email/addresses/{label}" class="btn ghost sm">{edit_label}</a>
                        <button class="btn ghost sm" hx-delete="{base_url}/dashboard/email/addresses/{label}" hx-confirm="{delete_confirm}">{delete_label}</button>
                    </td>
                </tr>"#,
                base_url = base_url,
                label = html_escape(&a.local_part),
                full = html_escape(&full),
                mode_label = mode_label,
                on_off = on_off,
                count = a.notification_recipients.len(),
                recip_suffix = recip_suffix,
                edit_label = edit_label,
                delete_label = delete_label,
                delete_confirm = delete_confirm,
            )
        })
        .collect();

    let address_table = if addrs.is_empty() {
        format!(r#"<p class="muted">{}</p>"#, t(locale, "admin-email-empty"),)
    } else {
        format!(
            r#"<div class="card p-0" style="overflow:hidden">
                <table>
                    <caption class="sr-only">{caption}</caption>
                    <thead><tr><th scope="col">{th_address}</th><th scope="col">{th_auto}</th><th scope="col">{th_notify}</th><th scope="col"><span class="sr-only">{th_actions}</span></th></tr></thead>
                    <tbody>{address_rows}</tbody>
                </table>
            </div>"#,
            address_rows = address_rows,
            caption = t(locale, "admin-email-h1"),
            th_address = t(locale, "admin-email-th-address"),
            th_auto = t(locale, "admin-email-th-autoreply"),
            th_notify = t(locale, "admin-email-th-notify"),
            th_actions = t(locale, "admin-email-th-actions"),
        )
    };

    let add_form = if at_limit {
        format!(
            r#"<div class="card p-18 mt-16 card-warn">
                <p class="m-0"><strong>{warn_prefix} {quota} {warn_suffix}</strong> <a href="{base_url}/dashboard/billing">{warn_link}</a> {warn_tail}</p>
            </div>"#,
            base_url = base_url,
            quota = quota,
            warn_prefix = t(locale, "admin-email-quota-warn-prefix"),
            warn_suffix = t(locale, "admin-email-quota-warn-suffix"),
            warn_link = t(locale, "admin-email-quota-warn-link"),
            warn_tail = t(locale, "admin-email-quota-warn-tail"),
        )
    } else {
        format!(
            r##"<div class="card p-18 mt-16">
                <h2 class="display-sm m-0 mb-8">{add_h2}</h2>
                <p class="muted mb-12">{used} {prefix} {quota} {suffix}</p>
                <form hx-post="{base_url}/dashboard/email/addresses" hx-ext="json-enc" hx-target="{HASH}toast" hx-swap="innerHTML">
                    <div class="row gap-8 wrap">
                        <label for="email-local-part" class="sr-only">{add_h2}</label>
                        <input id="email-local-part" class="input" name="local_part" placeholder="{ph}" required aria-required="true" style="max-width:240px">
                        <span class="muted">@{base_domain}</span>
                        <button class="btn primary" type="submit">{add_cta}</button>
                    </div>
                </form>
                <div id="toast" class="mt-8" role="status" aria-live="polite" aria-atomic="true"></div>
            </div>"##,
            HASH = HASH,
            base_url = base_url,
            base_domain = html_escape(base_domain),
            used = used,
            quota = quota,
            add_h2 = t(locale, "admin-email-add-h2"),
            prefix = t(locale, "admin-email-add-lead-prefix"),
            suffix = t(locale, "admin-email-add-lead-suffix"),
            ph = t(locale, "admin-email-add-placeholder"),
            add_cta = t(locale, "admin-email-add-cta"),
        )
    };

    let body = format!(
        r#"<div class="page-pad">
            <h1 class="display-sm mb-4">{h1}</h1>
            <p class="muted mb-16">{lead_prefix} <code>@{base_domain}</code> {lead_suffix}</p>
            {address_table}
            {add_form}
        </div>"#,
        base_domain = html_escape(base_domain),
        address_table = address_table,
        add_form = add_form,
        h1 = t(locale, "admin-email-h1"),
        lead_prefix = t(locale, "admin-email-lead-prefix"),
        lead_suffix = t(locale, "admin-email-lead-suffix"),
    );

    let page = app_shell(&body, "Email", base_url, locale);
    base_html(&t(locale, "admin-email-title"), &page, locale)
}

/// Per-address edit page.
pub fn email_address_html(
    addr: &EmailAddress,
    base_domain: &str,
    base_url: &str,
    locale: &Locale,
) -> String {
    let full = format!("{}@{}", addr.local_part, base_domain);

    let static_selected = addr.auto_reply.default_is_canned();
    let ai_selected = !static_selected;
    let enabled_attr = if addr.auto_reply.enabled {
        "checked"
    } else {
        ""
    };

    let auto_reply_form = format!(
        r##"<p class="muted fs-13 mb-12">{rules_prefix} <a href="{base_url}/dashboard/rules/email/{label}">{rules_link}</a>.</p>
        <form hx-put="{base_url}/dashboard/email/addresses/{label}/auto-reply" hx-ext="json-enc" hx-target="{HASH}auto-toast" hx-swap="innerHTML">
            <div class="form-group">
                <label class="row gap-8">
                    <span class="toggle">
                        <input type="checkbox" name="enabled" {enabled_attr}>
                        <span></span>
                    </span>
                    <span>{toggle_label}</span>
                </label>
            </div>
            <div class="form-group">
                <label for="email-mode">{mode_label}</label>
                <select id="email-mode" class="select" name="mode">
                    <option value="canned" {static_sel}>{mode_canned}</option>
                    <option value="prompt" {ai_sel}>{mode_prompt}</option>
                </select>
            </div>
            <div class="form-group">
                <label for="email-prompt">{prompt_label}</label>
                <textarea id="email-prompt" class="textarea" name="prompt" rows="6" placeholder="{prompt_ph}">{prompt}</textarea>
            </div>
            <div class="form-group">
                <label for="email-wait">{wait_prefix} ({wait}s)</label>
                <input id="email-wait" class="input" name="wait_seconds" type="number" min="0" max="120" value="{wait}">
                <p class="muted fs-12 mt-4">{wait_help}</p>
            </div>
            <div class="row gap-8">
                <button class="btn primary" type="submit">{save}</button>
            </div>
            <div id="auto-toast" class="mt-8" role="status" aria-live="polite" aria-atomic="true"></div>
        </form>"##,
        HASH = HASH,
        base_url = base_url,
        label = html_escape(&addr.local_part),
        enabled_attr = enabled_attr,
        static_sel = if static_selected { "selected" } else { "" },
        ai_sel = if ai_selected { "selected" } else { "" },
        prompt = html_escape(addr.auto_reply.default_text()),
        wait = addr.auto_reply.wait_seconds,
        rules_prefix = t(locale, "admin-email-edit-rules-prefix"),
        rules_link = t(locale, "admin-email-edit-rules-link"),
        toggle_label = t(locale, "admin-email-edit-toggle-label"),
        mode_label = t(locale, "admin-email-edit-mode-label"),
        mode_canned = t(locale, "admin-email-edit-mode-canned"),
        mode_prompt = t(locale, "admin-email-edit-mode-prompt"),
        prompt_label = t(locale, "admin-email-edit-prompt-label"),
        prompt_ph = html_escape(&t(locale, "admin-email-edit-prompt-placeholder")),
        wait_prefix = t(locale, "admin-email-edit-wait-prefix"),
        wait_help = t(locale, "admin-email-edit-wait-help"),
        save = t(locale, "admin-email-edit-save"),
    );

    let cc_label = t(locale, "admin-email-recipients-cc");
    let bcc_label = t(locale, "admin-email-recipients-bcc");
    let owner_label = t(locale, "admin-email-recipients-status-owner");
    let verified_label = t(locale, "admin-email-recipients-status-verified");
    let pending_label = t(locale, "admin-email-recipients-status-pending");
    let remove_label = t(locale, "admin-email-recipients-remove");
    let remove_confirm = html_escape(&t(locale, "admin-email-recipients-remove-confirm"));
    let recipient_rows: String = addr
        .notification_recipients
        .iter()
        .map(|r| {
            let kind_chip = match r.kind {
                RecipientKind::Cc => format!(r#"<span class="chip">{cc_label}</span>"#),
                RecipientKind::Bcc => format!(r#"<span class="chip">{bcc_label}</span>"#),
            };
            let status_chip = if r.is_owner {
                format!(r#"<span class="chip ok">{owner_label}</span>"#)
            } else if matches!(r.status, RecipientStatus::Verified) {
                format!(r#"<span class="chip ok">{verified_label}</span>"#)
            } else {
                format!(r#"<span class="chip warn">{pending_label}</span>"#)
            };
            let delete_btn = if r.is_owner {
                String::new()
            } else {
                format!(
                    r#"<button class="btn ghost sm" hx-delete="{base_url}/dashboard/email/addresses/{label}/recipients/{id}" hx-confirm="{confirm}" hx-target="closest tr" hx-swap="outerHTML">{remove}</button>"#,
                    base_url = base_url,
                    label = html_escape(&addr.local_part),
                    id = html_escape(&r.id),
                    confirm = remove_confirm,
                    remove = remove_label,
                )
            };
            format!(
                r#"<tr>
                    <td><strong>{address}</strong></td>
                    <td>{kind_chip}</td>
                    <td>{status_chip}</td>
                    <td class="ta-right">{delete_btn}</td>
                </tr>"#,
                address = html_escape(&r.address),
                kind_chip = kind_chip,
                status_chip = status_chip,
                delete_btn = delete_btn,
            )
        })
        .collect();

    let recipients_section = format!(
        r##"<div class="card p-0" style="overflow:hidden">
            <table>
                <caption class="sr-only">{caption}</caption>
                <thead><tr><th scope="col">{th_addr}</th><th scope="col">{th_kind}</th><th scope="col">{th_status}</th><th scope="col"><span class="sr-only">{th_actions}</span></th></tr></thead>
                <tbody>{recipient_rows}</tbody>
            </table>
        </div>
        <form class="card p-18 mt-12" hx-post="{base_url}/dashboard/email/addresses/{label}/recipients" hx-ext="json-enc" hx-target="{HASH}rec-toast" hx-swap="innerHTML">
            <div class="row gap-8 wrap">
                <label for="recipient-address" class="sr-only">{th_addr}</label>
                <input id="recipient-address" class="input" name="address" type="email" placeholder="{ph}" required aria-required="true" style="flex:1;max-width:340px">
                <label for="recipient-kind" class="sr-only">{th_kind}</label>
                <select id="recipient-kind" class="select" name="kind" style="max-width:120px">
                    <option value="cc">{cc_label}</option>
                    <option value="bcc">{bcc_label}</option>
                </select>
                <button class="btn primary" type="submit">{cta}</button>
            </div>
            <p class="muted fs-12 mt-8">{lead}</p>
            <div id="rec-toast" class="mt-8" role="status" aria-live="polite" aria-atomic="true"></div>
        </form>"##,
        HASH = HASH,
        base_url = base_url,
        label = html_escape(&addr.local_part),
        recipient_rows = recipient_rows,
        caption = t(locale, "admin-email-recipients-h2"),
        th_addr = t(locale, "admin-email-recipients-th-address"),
        th_kind = t(locale, "admin-email-recipients-th-kind"),
        th_status = t(locale, "admin-email-recipients-th-status"),
        th_actions = t(locale, "admin-email-th-actions"),
        ph = html_escape(&t(locale, "admin-email-recipients-add-placeholder")),
        cc_label = cc_label,
        bcc_label = bcc_label,
        cta = t(locale, "admin-email-recipients-add-cta"),
        lead = t(locale, "admin-email-recipients-lead"),
    );

    let body = format!(
        r#"<div class="page-pad">
            <p><a href="{base_url}/dashboard/email" class="btn ghost sm">{back}</a></p>
            <h1 class="display-sm mt-8 mb-4">{full}</h1>
            <p class="muted mb-24">{lead}</p>
            <h2 class="display-sm mb-8">{auto_h2}</h2>
            <div class="card p-22 mb-24">{auto_reply_form}</div>
            <h2 class="display-sm mb-8">{rec_h2}</h2>
            {recipients_section}
        </div>"#,
        base_url = base_url,
        full = html_escape(&full),
        auto_reply_form = auto_reply_form,
        recipients_section = recipients_section,
        back = t(locale, "admin-email-edit-back"),
        lead = t(locale, "admin-email-edit-h1"),
        auto_h2 = t(locale, "admin-email-edit-rules-link"),
        rec_h2 = t(locale, "admin-email-recipients-h2"),
    );

    let page = app_shell(&body, "Email", base_url, locale);
    base_html(
        &format!("{full}{}", t(locale, "admin-email-edit-title-suffix")),
        &page,
        locale,
    )
}

/// Public verification page rendered when a recipient clicks the link
/// from the verification email. The handler decides which variant to
/// show (success / already / expired).
pub fn email_verify_result_html(message: &str, locale: &Locale) -> String {
    let body = format!(
        r#"<div class="page-pad ta-center">
            <h1 class="display-md mb-12">{h1}</h1>
            <p class="lead">{message}</p>
            <p class="mt-16"><a class="btn ghost sm" href="/">{back}</a></p>
        </div>"#,
        message = html_escape(message),
        h1 = t(locale, "admin-email-verify-h1"),
        back = t(locale, "admin-email-verify-back"),
    );
    base_html(&t(locale, "admin-email-verify-title"), &body, locale)
}
