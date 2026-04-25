//! Admin templates for the email feature: address list and per-address
//! edit (auto-reply config + notification recipients).

use crate::helpers::html_escape;
use crate::types::*;

use super::base::{app_shell, base_html};
use super::HASH;

/// Top-level email dashboard.
pub fn email_dashboard_html(
    addrs: &[EmailAddress],
    tenant: &Tenant,
    base_domain: &str,
    base_url: &str,
) -> String {
    let used = addrs.len() as u32;
    let quota = tenant.email_address_quota();
    let at_limit = used >= quota;

    let address_rows: String = addrs
        .iter()
        .map(|a| {
            let full = format!("{}@{}", a.local_part, base_domain);
            let mode_label = match a.auto_reply.mode {
                AutoReplyMode::Static => "Static",
                AutoReplyMode::Ai => "AI",
            };
            let on_off = if a.auto_reply.enabled {
                r#"<span class="chip ok">on</span>"#
            } else {
                r#"<span class="chip">off</span>"#
            };
            format!(
                r#"<tr>
                    <td><a href="{base_url}/admin/email/addresses/{label}" class="link-reset"><strong>{full}</strong></a></td>
                    <td>{mode_label} {on_off}</td>
                    <td>{count} recipient(s)</td>
                    <td class="ta-right">
                        <a href="{base_url}/admin/email/addresses/{label}" class="btn ghost sm">Edit</a>
                        <button class="btn ghost sm" hx-delete="{base_url}/admin/email/addresses/{label}" hx-confirm="Delete this address? Inbound mail will be rejected.">Delete</button>
                    </td>
                </tr>"#,
                base_url = base_url,
                label = html_escape(&a.local_part),
                full = html_escape(&full),
                mode_label = mode_label,
                on_off = on_off,
                count = a.notification_recipients.len(),
            )
        })
        .collect();

    let address_table = if addrs.is_empty() {
        r#"<p class="muted">No email addresses yet. Pick a name below: you can use it like <code>name@domain</code> from the moment you save.</p>"#.to_string()
    } else {
        format!(
            r#"<div class="card p-0" style="overflow:hidden">
                <table>
                    <thead><tr><th>Address</th><th>Auto-reply</th><th>Notify</th><th></th></tr></thead>
                    <tbody>{address_rows}</tbody>
                </table>
            </div>"#,
            address_rows = address_rows,
        )
    };

    let add_form = if at_limit {
        format!(
            r#"<div class="card p-18 mt-16 card-warn">
                <p class="m-0"><strong>You've used your {quota} address slot(s).</strong> Buy a 5-pack from <a href="{base_url}/admin/billing">Billing</a> to add more.</p>
            </div>"#,
            base_url = base_url,
            quota = quota,
        )
    } else {
        format!(
            r##"<div class="card p-18 mt-16">
                <h2 class="display-sm m-0 mb-8">Add an address</h2>
                <p class="muted mb-12">{used} of {quota} addresses used. Pick a memorable local-part: it can use a-z, 0-9, dot, dash, underscore.</p>
                <form hx-post="{base_url}/admin/email/addresses" hx-ext="json-enc" hx-target="{HASH}toast" hx-swap="innerHTML">
                    <div class="row gap-8 wrap">
                        <input class="input" name="local_part" placeholder="support" required style="max-width:240px">
                        <span class="muted">@{base_domain}</span>
                        <button class="btn primary" type="submit">Add</button>
                    </div>
                </form>
                <div id="toast" class="mt-8"></div>
            </div>"##,
            HASH = HASH,
            base_url = base_url,
            base_domain = html_escape(base_domain),
            used = used,
            quota = quota,
        )
    };

    let body = format!(
        r#"<div class="page-pad">
            <h1 class="display-sm mb-4">Email</h1>
            <p class="muted mb-16">Each address you add at <code>@{base_domain}</code> can auto-reply to incoming mail. Replies go to the sender; you and your team get a copy via Cc/Bcc.</p>
            {address_table}
            {add_form}
        </div>"#,
        base_domain = html_escape(base_domain),
        address_table = address_table,
        add_form = add_form,
    );

    let page = app_shell(&body, "Email", base_url);
    base_html("Email: Concierge", &page)
}

/// Per-address edit page.
pub fn email_address_html(addr: &EmailAddress, base_domain: &str, base_url: &str) -> String {
    let full = format!("{}@{}", addr.local_part, base_domain);

    let static_selected = matches!(addr.auto_reply.mode, AutoReplyMode::Static);
    let ai_selected = matches!(addr.auto_reply.mode, AutoReplyMode::Ai);
    let enabled_attr = if addr.auto_reply.enabled {
        "checked"
    } else {
        ""
    };

    let auto_reply_form = format!(
        r##"<form hx-put="{base_url}/admin/email/addresses/{label}/auto-reply" hx-ext="json-enc" hx-target="{HASH}auto-toast" hx-swap="innerHTML">
            <div class="form-group">
                <label class="row gap-8">
                    <span class="toggle">
                        <input type="checkbox" name="enabled" {enabled_attr}>
                        <span></span>
                    </span>
                    <span>Reply automatically to inbound mail</span>
                </label>
            </div>
            <div class="form-group">
                <label>Mode</label>
                <select class="select" name="mode">
                    <option value="static" {static_sel}>Static: same canned reply every time</option>
                    <option value="ai" {ai_sel}>AI: generate a reply for each message (uses 1 credit)</option>
                </select>
            </div>
            <div class="form-group">
                <label>Reply text / AI prompt</label>
                <textarea class="textarea" name="prompt" rows="6" placeholder="In static mode this exact text is sent. In AI mode, this is the system prompt for the model.">{prompt}</textarea>
            </div>
            <div class="form-group">
                <label>Wait before replying ({wait}s)</label>
                <input class="input" name="wait_seconds" type="number" min="0" max="120" value="{wait}">
                <p class="muted fs-12 mt-4">Lets clusters of forwarded messages collapse into one reply. 0 = reply immediately.</p>
            </div>
            <div class="row gap-8">
                <button class="btn primary" type="submit">Save</button>
            </div>
            <div id="auto-toast" class="mt-8"></div>
        </form>"##,
        HASH = HASH,
        base_url = base_url,
        label = html_escape(&addr.local_part),
        enabled_attr = enabled_attr,
        static_sel = if static_selected { "selected" } else { "" },
        ai_sel = if ai_selected { "selected" } else { "" },
        prompt = html_escape(&addr.auto_reply.prompt),
        wait = addr.auto_reply.wait_seconds,
    );

    let recipient_rows: String = addr
        .notification_recipients
        .iter()
        .map(|r| {
            let kind_chip = match r.kind {
                RecipientKind::Cc => r#"<span class="chip">Cc</span>"#,
                RecipientKind::Bcc => r#"<span class="chip">Bcc</span>"#,
            };
            let status_chip = if r.is_owner {
                r#"<span class="chip ok">Owner</span>"#.to_string()
            } else if matches!(r.status, RecipientStatus::Verified) {
                r#"<span class="chip ok">Verified</span>"#.to_string()
            } else {
                r#"<span class="chip warn">Pending</span>"#.to_string()
            };
            let delete_btn = if r.is_owner {
                String::new()
            } else {
                format!(
                    r#"<button class="btn ghost sm" hx-delete="{base_url}/admin/email/addresses/{label}/recipients/{id}" hx-confirm="Remove this recipient?" hx-target="closest tr" hx-swap="outerHTML">Remove</button>"#,
                    base_url = base_url,
                    label = html_escape(&addr.local_part),
                    id = html_escape(&r.id),
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
                <thead><tr><th>Address</th><th>Kind</th><th>Status</th><th></th></tr></thead>
                <tbody>{recipient_rows}</tbody>
            </table>
        </div>
        <form class="card p-18 mt-12" hx-post="{base_url}/admin/email/addresses/{label}/recipients" hx-ext="json-enc" hx-target="{HASH}rec-toast" hx-swap="innerHTML">
            <div class="row gap-8 wrap">
                <input class="input" name="address" type="email" placeholder="team@example.com" required style="flex:1;max-width:340px">
                <select class="select" name="kind" style="max-width:120px">
                    <option value="cc">Cc</option>
                    <option value="bcc">Bcc</option>
                </select>
                <button class="btn primary" type="submit">Add &amp; verify</button>
            </div>
            <p class="muted fs-12 mt-8">We'll send a one-click verification link to confirm ownership before notifications start arriving.</p>
            <div id="rec-toast" class="mt-8"></div>
        </form>"##,
        HASH = HASH,
        base_url = base_url,
        label = html_escape(&addr.local_part),
        recipient_rows = recipient_rows,
    );

    let body = format!(
        r#"<div class="page-pad">
            <p><a href="{base_url}/admin/email" class="btn ghost sm">&larr; Email</a></p>
            <h1 class="display-sm mt-8 mb-4">{full}</h1>
            <p class="muted mb-24">Replies are sent from this address. Cc/Bcc recipients listed below get a copy of every reply.</p>
            <h2 class="display-sm mb-8">Auto-reply</h2>
            <div class="card p-22 mb-24">{auto_reply_form}</div>
            <h2 class="display-sm mb-8">Notify on every reply</h2>
            {recipients_section}
        </div>"#,
        base_url = base_url,
        full = html_escape(&full),
        auto_reply_form = auto_reply_form,
        recipients_section = recipients_section,
    );

    let page = app_shell(&body, "Email", base_url);
    base_html(&format!("{full}: Concierge"), &page)
}

/// Public verification page rendered when a recipient clicks the link
/// from the verification email. The handler decides which variant to
/// show (success / already / expired).
pub fn email_verify_result_html(message: &str) -> String {
    let body = format!(
        r#"<div class="page-pad ta-center">
            <h1 class="display-md mb-12">Email verification</h1>
            <p class="lead">{message}</p>
            <p class="mt-16"><a class="btn ghost sm" href="/">Back to Concierge</a></p>
        </div>"#,
        message = html_escape(message),
    );
    base_html("Email verification: Concierge", &body)
}
