//! Management panel templates: super-admin UI

use crate::helpers::html_escape;
use crate::locale::Locale;
use crate::types::*;

use super::base::{base_html, brand_mark};
use super::HASH;

fn manage_shell(
    title: &str,
    content: &str,
    active: &str,
    actor_email: &str,
    base_url: &str,
    locale: &Locale,
) -> String {
    let nav_items = [
        ("Dashboard", "/manage"),
        ("Tenants", "/manage/tenants"),
        ("Archetypes", "/manage/archetypes"),
        ("Demo", "/manage/demo"),
        ("Billing", "/manage/billing"),
        ("Audit Log", "/manage/audit"),
    ];

    let nav: String = nav_items
        .iter()
        .map(|(label, href)| {
            let class = if *label == active { " active" } else { "" };
            format!(r#"<a class="{class}" href="{base_url}{href}">{label}</a>"#)
        })
        .collect();

    // Cloudflare Access sign-out endpoint clears the CF_Authorization
    // cookie + JWT, which is the only credential the management panel
    // accepts. After logout the user is bounced back to the team's IdP
    // picker.
    //
    // The confirm dialog at the bottom intercepts every `hx-confirm`
    // attribute on /manage and routes it through a themed `<dialog>`
    // instead of the browser-native `confirm()` modal. The script reads
    // the verb (Delete / Remove / Wipe / …) out of the prompt's first
    // word to pick a danger-styled OK button when appropriate.
    let inner = format!(
        r##"<div class="app">
  <header class="app-top">
    {brand}
    <nav class="app-nav" aria-label="Management sections">{nav}</nav>
    <div class="app-actor">
      <span class="actor-email" title="{email}">{email}</span>
      <a class="signout" href="/cdn-cgi/access/logout" rel="nofollow">Sign out</a>
    </div>
  </header>
  <div id="toast-region" class="toast-region" role="status" aria-live="polite" aria-atomic="false"></div>
  {content}

  <dialog id="manage-confirm" class="manage-confirm" aria-labelledby="manage-confirm-title">
    <div class="confirm-card">
      <p id="manage-confirm-title" class="confirm-eyebrow">Confirm</p>
      <p id="manage-confirm-msg" class="confirm-msg"></p>
      <div class="confirm-actions">
        <button type="button" class="btn ghost sm" data-confirm-cancel>Cancel</button>
        <button type="button" class="btn sm" data-confirm-ok>Confirm</button>
      </div>
    </div>
  </dialog>
  <script type="module" nonce="__CSP_NONCE__">
  const dialog = document.getElementById('manage-confirm');
  const msgEl = document.getElementById('manage-confirm-msg');
  const titleEl = document.getElementById('manage-confirm-title');
  const okBtn = dialog.querySelector('[data-confirm-ok]');
  const cancelBtn = dialog.querySelector('[data-confirm-cancel]');
  let pendingEvt = null;

  // Pick eyebrow + button styling from the first word of the prompt.
  // "Delete" / "Wipe" → destructive; "Remove" → mild; default → neutral.
  function classify(prompt) {{
    const w = (prompt || '').trim().split(/\s+/)[0].toLowerCase();
    if (w === 'delete' || w === 'wipe') return {{ eyebrow: 'Destructive action', danger: true, ok: 'Delete' }};
    if (w === 'remove') return {{ eyebrow: 'Remove', danger: true, ok: 'Remove' }};
    return {{ eyebrow: 'Confirm', danger: false, ok: 'Confirm' }};
  }}

  document.body.addEventListener('htmx:confirm', (evt) => {{
    const prompt = evt.detail.question;
    if (!prompt) return; // no hx-confirm set — let HTMX proceed
    evt.preventDefault();
    pendingEvt = evt;
    const c = classify(prompt);
    titleEl.textContent = c.eyebrow;
    msgEl.textContent = prompt;
    okBtn.textContent = c.ok;
    okBtn.classList.toggle('danger', c.danger);
    okBtn.classList.toggle('primary', !c.danger);
    dialog.showModal();
    okBtn.focus();
  }});

  okBtn.addEventListener('click', () => {{
    dialog.close();
    if (pendingEvt) {{
      pendingEvt.detail.issueRequest(true);
      pendingEvt = null;
    }}
  }});
  cancelBtn.addEventListener('click', () => {{
    dialog.close();
    pendingEvt = null;
  }});
  dialog.addEventListener('cancel', () => {{ pendingEvt = null; }});
  </script>
</div>"##,
        brand = brand_mark(),
        nav = nav,
        email = html_escape(actor_email),
        content = content,
    );

    base_html(title, &inner, locale)
}

/// Standardized page header for /manage pages.
///
/// `back` is `(href, label)` for detail/edit views; `None` on top-level
/// list pages. `right_slot` is raw HTML for the right-aligned action(s)
/// (e.g. the "+ New archetype" button on the archetypes list).
/// `subtitle` is a single-line muted line under the title.
fn manage_header(
    eyebrow: &str,
    title: &str,
    back: Option<(&str, &str)>,
    subtitle: Option<&str>,
    right_slot: &str,
) -> String {
    let back_html = match back {
        Some((href, label)) => format!(
            r#"<a class="back" href="{href}">&larr; {label}</a>"#,
            href = href,
            label = html_escape(label),
        ),
        None => String::new(),
    };
    let subtitle_html = match subtitle {
        Some(s) if !s.is_empty() => format!(r#"<p class="header-subtitle">{}</p>"#, s),
        _ => String::new(),
    };
    format!(
        r##"<div class="manage-header">
  {back}
  <div class="header-row">
    <div class="header-title">
      <div class="eyebrow">{eyebrow}</div>
      <h1 class="display-sm m-0 mt-4">{title}</h1>
      {subtitle}
    </div>
    <div class="header-actions">{right_slot}</div>
  </div>
</div>"##,
        back = back_html,
        eyebrow = html_escape(eyebrow),
        title = title,
        subtitle = subtitle_html,
        right_slot = right_slot,
    )
}

pub fn dashboard_html(
    email: &str,
    tenant_count: usize,
    health: &crate::handlers::health::HealthReport,
    base_url: &str,
    locale: &Locale,
) -> String {
    let health_panel = health_panel_html(health);
    let header = manage_header(
        "Management",
        "Overview",
        None,
        Some(&format!("Signed in as {}", html_escape(email))),
        "",
    );
    // KPI placeholders for MRR and Active are deliberately omitted until
    // their data sources land. The single Tenants tile keeps the
    // dashboard honest about what it actually knows today.
    let content = format!(
        r##"<div class="page-pad">
  {header}
  <div class="card p-18 mb-16" style="max-width:240px">
    <div class="stat-n serif">{tenant_count}</div>
    <div class="mono muted fs-11">Tenants</div>
  </div>

  {health_panel}
</div>"##,
        header = header,
        tenant_count = tenant_count,
        health_panel = health_panel,
    );

    manage_shell(
        "Management · Concierge",
        &content,
        "Dashboard",
        email,
        base_url,
        locale,
    )
}

fn health_panel_html(report: &crate::handlers::health::HealthReport) -> String {
    use crate::handlers::health::Status;
    let overall_chip = match report.overall {
        Status::Ok => r#"<span class="chip ok">All systems normal</span>"#,
        Status::Warn => r#"<span class="chip warn">Degraded</span>"#,
        Status::Error => r#"<span class="chip error">Issues detected</span>"#,
    };
    let rows: String = report
        .checks
        .iter()
        .map(|c| {
            let dot = match c.status {
                Status::Ok => r#"<span class="dot ok"></span>"#,
                Status::Warn => r#"<span class="dot warn"></span>"#,
                Status::Error => r#"<span class="dot error"></span>"#,
            };
            format!(
                r#"<div class="rt-row" style="grid-template-columns:auto 1.2fr 2fr">
  <div>{dot}</div>
  <div class="fw-600">{name}</div>
  <div class="muted fs-13">{detail}</div>
</div>"#,
                dot = dot,
                name = html_escape(&c.name),
                detail = html_escape(&c.detail),
            )
        })
        .collect();
    format!(
        r##"<div class="card" style="padding:0;overflow:hidden">
  <div class="between p-18">
    <div>
      <div class="eyebrow">Connection status</div>
      <p class="muted m-0 mt-4 fs-13">External providers + bindings: refreshed every 60s. Generated {ts}.</p>
    </div>
    {chip}
  </div>
  <div>{rows}</div>
</div>"##,
        chip = overall_chip,
        ts = html_escape(&report.generated_at),
        rows = rows,
    )
}

pub fn tenants_list_html(
    tenants: &[Tenant],
    query: &str,
    actor_email: &str,
    base_url: &str,
    locale: &Locale,
) -> String {
    let table = tenants_table_html(tenants, base_url);
    let header = manage_header("All tenants", "Tenants", None, None, "");

    let content = format!(
        r##"<div class="page-pad">
  {header}
  <div class="row gap-12 mb-12 wrap" style="align-items:center">
    <input
      class="input w-input-lg" type="search" name="q" value="{q}"
      placeholder="Search by email or name…"
      hx-get="{base_url}/manage/tenants"
      hx-trigger="input changed delay:200ms, search"
      hx-target="{hash}tenants-table" hx-swap="outerHTML"
      hx-push-url="true"
      autocomplete="off">
  </div>
  {table}
</div>"##,
        header = header,
        base_url = base_url,
        hash = HASH,
        q = html_escape(query),
        table = table,
    );

    manage_shell(
        "Tenants · Concierge",
        &content,
        "Tenants",
        actor_email,
        base_url,
        locale,
    )
}

/// Render just the `<div id="tenants-table">` portion of the tenants
/// list. Used both for the full page and for the HTMX search swap.
pub fn tenants_table_html(tenants: &[Tenant], base_url: &str) -> String {
    let rows: String = tenants
        .iter()
        .map(|t| {
            format!(
                r##"<div class="rt-row" style="grid-template-columns:1fr 1fr 0.6fr 0.5fr 80px">
  <div><a href="{base_url}/manage/tenants/{id}"><strong>{email}</strong></a></div>
  <div class="muted">{name}</div>
  <div><span class="chip">{plan}</span></div>
  <div class="mono muted fs-11">{created}</div>
  <div>
    <button class="btn ghost sm danger" hx-delete="{base_url}/manage/tenants/{id}" hx-confirm="Delete tenant {email} and ALL their data?" hx-target="closest .rt-row" hx-swap="outerHTML">Delete</button>
  </div>
</div>"##,
                base_url = base_url,
                id = html_escape(&t.id),
                email = html_escape(&t.email),
                name = html_escape(t.name.as_deref().unwrap_or("–")),
                plan = html_escape(t.plan.label()),
                created = html_escape(&t.created_at.get(..10).unwrap_or(&t.created_at)),
            )
        })
        .collect();

    let body = if tenants.is_empty() {
        empty_state(
            "No tenants match",
            "Try a different search, or clear the box to see every tenant.",
            None,
        )
    } else {
        format!(
            r##"<div class="rt-head" style="grid-template-columns:1fr 1fr 0.6fr 0.5fr 80px">
  <div>Email</div><div>Name</div><div>Plan</div><div>Created</div><div></div>
</div>{rows}"##,
            rows = rows,
        )
    };

    let count_line = format!(
        r#"<div class="muted fs-12 mb-8">{n} tenant{s}</div>"#,
        n = tenants.len(),
        s = if tenants.len() == 1 { "" } else { "s" },
    );

    format!(
        r##"<div id="tenants-table">
  {count_line}
  <div class="card" style="padding:0;overflow:hidden">{body}</div>
</div>"##,
        count_line = count_line,
        body = body,
    )
}

/// Single empty-state component used by all /manage list pages.
/// `cta` is `(href, label)` for an optional call-to-action.
fn empty_state(headline: &str, subtext: &str, cta: Option<(&str, &str)>) -> String {
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

pub fn tenant_detail_html(
    tenant: &Tenant,
    wa: &[WhatsAppAccount],
    ig: &[InstagramAccount],
    addrs: &[EmailAddress],
    billing: &TenantBilling,
    actor_email: &str,
    base_url: &str,
    locale: &Locale,
) -> String {
    let wa_list: String = wa
        .iter()
        .map(|a| {
            format!(
                r##"<div class="side-row"><div class="flex-1 fs-13">{name} <span class="mono muted">{phone}</span></div></div>"##,
                name = html_escape(&a.name),
                phone = html_escape(&a.phone_number),
            )
        })
        .collect();

    let ig_list: String = ig
        .iter()
        .map(|a| {
            format!(
                r##"<div class="side-row"><div class="flex-1 fs-13">@{username}</div></div>"##,
                username = html_escape(&a.instagram_username),
            )
        })
        .collect();

    let domain_list: String = addrs
        .iter()
        .map(|a| {
            format!(
                r##"<div class="side-row"><div class="flex-1 fs-13">{local}</div></div>"##,
                local = html_escape(&a.local_part),
            )
        })
        .collect();

    let plan_options: String = crate::types::Plan::ALL
        .iter()
        .map(|p| {
            let sel = if *p == tenant.plan { " selected" } else { "" };
            format!(
                r#"<option value="{val}"{sel}>{label}</option>"#,
                val = p.as_str(),
                label = p.label(),
            )
        })
        .collect();

    let delete_btn = format!(
        r##"<button class="btn ghost sm danger" hx-delete="{base_url}/manage/tenants/{id}" hx-confirm="Delete this tenant and ALL their data?">Delete tenant</button>"##,
        base_url = base_url,
        id = html_escape(&tenant.id),
    );

    let header = manage_header(
        "Tenant",
        &html_escape(&tenant.email),
        Some((&format!("{}/manage/tenants", base_url), "Back to tenants")),
        Some(&format!(
            "{name} · {plan} · joined {created}",
            name = html_escape(tenant.name.as_deref().unwrap_or("–")),
            plan = html_escape(tenant.plan.label()),
            created = html_escape(&tenant.created_at.get(..10).unwrap_or(&tenant.created_at)),
        )),
        &delete_btn,
    );

    let content = format!(
        r##"<div class="page-pad">
  {header}
  <div class="card p-18 mb-16">
    <h3 class="mb-8">Plan</h3>
    <p class="muted mb-12">Currently on <strong>{plan_label}</strong>.</p>
    <form hx-put="{base_url}/manage/tenants/{id}" hx-target="{hash}toast-region" hx-swap="afterbegin">
      <div class="row gap-12">
        <label for="tenant-plan" class="sr-only">Plan</label>
        <select id="tenant-plan" class="select w-input-md" name="plan">
          {plan_options}
        </select>
        <button class="btn sm" type="submit">Save plan</button>
      </div>
    </form>
  </div>
  <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:16px">
    <div class="card p-16">
      <div class="eyebrow">WhatsApp ({wa_count})</div>
      <div class="side-list">{wa_list}</div>
    </div>
    <div class="card p-16">
      <div class="eyebrow">Instagram ({ig_count})</div>
      <div class="side-list">{ig_list}</div>
    </div>
    <div class="card p-16">
      <div class="eyebrow">Email Domains ({domain_count})</div>
      <div class="side-list">{domain_list}</div>
    </div>
  </div>

  <!-- Two grant flows hit different endpoints (/grant-replies and
       /grant-addresses) but the operator picks between them based on
       intent, not visual layout. Combined into one card with a
       segmented toggle so the switch is immediate and the
       balance/quota line tracks the active mode. -->
  <div class="card p-18 mt-16" x-data="{{ kind: 'replies' }}">
    <div class="between mb-12 wrap" style="gap:12px">
      <h3 class="m-0">Grant credits</h3>
      <div class="seg-tabs" role="tablist" aria-label="Grant type">
        <button type="button" role="tab" :aria-selected="kind === 'replies'" :class="kind === 'replies' ? 'active' : ''" @click="kind = 'replies'">Replies</button>
        <button type="button" role="tab" :aria-selected="kind === 'addresses'" :class="kind === 'addresses' ? 'active' : ''" @click="kind = 'addresses'">Addresses</button>
      </div>
    </div>

    <p class="muted mb-12" x-show="kind === 'replies'">Add reply credits to this tenant's balance. Current balance: <strong>{balance}</strong>.</p>
    <p class="muted mb-12" x-show="kind === 'addresses'" x-cloak>Add to this tenant's reply-email quota. Current quota: <strong>{quota}</strong> address(es).</p>

    <form x-show="kind === 'replies'" hx-post="{base_url}/manage/tenants/{id}/grant-replies" hx-target="{hash}toast-region" hx-swap="afterbegin" hx-ext="json-enc">
      <div class="row gap-12 wrap" style="align-items:flex-end">
        <label class="stack">
          <span class="eyebrow lbl">Replies</span>
          <input class="input mono w-input-sm" name="replies" type="number" min="1" required placeholder="e.g. 100">
        </label>
        <label class="stack">
          <span class="eyebrow lbl">Expires in (days)</span>
          <input class="input mono w-input-sm" name="expires_days" type="number" min="1" value="365" required>
        </label>
        <button class="btn sm" type="submit">Grant replies</button>
      </div>
    </form>

    <form x-show="kind === 'addresses'" x-cloak hx-post="{base_url}/manage/tenants/{id}/grant-addresses" hx-target="{hash}toast-region" hx-swap="afterbegin" hx-ext="json-enc">
      <div class="row gap-12 wrap" style="align-items:flex-end">
        <label class="stack">
          <span class="eyebrow lbl">Address slots</span>
          <input class="input mono w-input-sm" name="addresses" type="number" min="1" required placeholder="e.g. 5">
        </label>
        <button class="btn sm" type="submit">Grant addresses</button>
      </div>
    </form>
  </div>
</div>"##,
        header = header,
        base_url = base_url,
        hash = HASH,
        id = html_escape(&tenant.id),
        plan_label = html_escape(tenant.plan.label()),
        plan_options = plan_options,
        wa_count = wa.len(),
        ig_count = ig.len(),
        domain_count = addrs.len(),
        balance = billing.total_remaining(),
        quota = tenant.email_address_quota(),
        wa_list = if wa_list.is_empty() {
            r#"<div class="muted fs-13">None</div>"#.to_string()
        } else {
            wa_list
        },
        ig_list = if ig_list.is_empty() {
            r#"<div class="muted fs-13">None</div>"#.to_string()
        } else {
            ig_list
        },
        domain_list = if domain_list.is_empty() {
            r#"<div class="muted fs-13">None</div>"#.to_string()
        } else {
            domain_list
        },
    );

    manage_shell(
        &format!("{} · Concierge", tenant.email),
        &content,
        "Tenants",
        actor_email,
        base_url,
        locale,
    )
}

pub fn audit_html(
    log: &[serde_json::Value],
    actor_email: &str,
    base_url: &str,
    locale: &Locale,
) -> String {
    let rows: String = log
        .iter()
        .map(|entry| {
            let actor = entry
                .get("actor_email")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let action = entry.get("action").and_then(|v| v.as_str()).unwrap_or("?");
            let resource_type = entry
                .get("resource_type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let resource_id = entry
                .get("resource_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let created = entry
                .get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let action_cell = audit_action_chip(action);
            let resource_cell = audit_resource_cell(base_url, resource_type, resource_id);

            format!(
                r##"<div class="rt-row" style="grid-template-columns:0.8fr 1fr 0.7fr 1.4fr">
  <div class="mono muted fs-11">{created}</div>
  <div class="fs-13">{actor}</div>
  <div>{action_cell}</div>
  <div>{resource_cell}</div>
</div>"##,
                created = html_escape(created.get(..19).unwrap_or(created)),
                actor = html_escape(actor),
                action_cell = action_cell,
                resource_cell = resource_cell,
            )
        })
        .collect();

    let empty = if log.is_empty() {
        empty_state(
            "No audit entries yet",
            "Every management action (plan change, credit grant, archetype edit, …) is recorded here. Take an action and it will appear at the top.",
            None,
        )
    } else {
        String::new()
    };

    let header = manage_header("Audit log", "Management actions", None, None, "");

    let content = format!(
        r##"<div class="page-pad">
  {header}
  <div class="card" style="padding:0;overflow:hidden">
    <div class="rt-head" style="grid-template-columns:0.8fr 1fr 0.7fr 1.4fr">
      <div>Time</div><div>Actor</div><div>Action</div><div>Resource</div>
    </div>
    {rows}{empty}
  </div>
</div>"##,
        header = header,
        rows = rows,
        empty = empty,
    );

    manage_shell(
        "Audit Log · Concierge",
        &content,
        "Audit Log",
        actor_email,
        base_url,
        locale,
    )
}

/// Render a single audit row's action column. Maps the wire-name
/// (snake_case stored in D1) to a human label and color-tags
/// destructive verbs so deletes stand out at a glance.
fn audit_action_chip(action: &str) -> String {
    let (label, kind) = match action {
        "create_archetype" => ("Created archetype", "ok"),
        "edit_archetype" => ("Edited archetype", ""),
        "delete_archetype" => ("Deleted archetype", "error"),
        "grant_replies" => ("Granted replies", "ok"),
        "grant_addresses" => ("Granted addresses", "ok"),
        "update_tenant" => ("Updated tenant", ""),
        "delete_tenant" => ("Deleted tenant", "error"),
        "update_pricing" => ("Updated pricing", ""),
        "delete_pricing_currency" => ("Removed currency", "error"),
        "schedule_grant" => ("Scheduled grant", "ok"),
        "schedule_grant_remove" => ("Removed scheduled grant", "error"),
        "edit_demo_config" => ("Edited demo config", ""),
        other => return format!(r#"<span class="chip mono">{}</span>"#, html_escape(other)),
    };
    let cls = if kind.is_empty() {
        "chip".to_string()
    } else {
        format!("chip {kind}")
    };
    format!(r#"<span class="{cls}">{}</span>"#, html_escape(label))
}

/// Render the resource column. Where the resource has a detail page
/// in /manage we link to it; otherwise we surface the bare ID with a
/// copy button so an operator can paste it into a search or a query.
fn audit_resource_cell(base_url: &str, kind: &str, id: &str) -> String {
    if id.is_empty() {
        return match kind {
            "billing" => r#"<a class="mono fs-12" href="/manage/billing">Billing</a>"#.to_string(),
            "demo_config" => {
                r#"<a class="mono fs-12" href="/manage/demo">Demo config</a>"#.to_string()
            }
            "" => r#"<span class="muted fs-12">—</span>"#.to_string(),
            other => format!(
                r#"<span class="mono muted fs-12">{}</span>"#,
                html_escape(other)
            ),
        };
    }
    let id_esc = html_escape(id);
    let kind_label = match kind {
        "tenant" => "Tenant",
        "archetype" => "Archetype",
        "billing" => "Billing",
        "demo_config" => "Demo",
        other => other,
    };
    let kind_label_esc = html_escape(kind_label);
    let copy_btn = format!(
        r##"<button type="button" class="copy-btn btn ghost sm" data-copy-text="{id_esc}" data-copy-label="Copy" title="Copy {kind_label_esc} ID" aria-label="Copy {kind_label_esc} ID">Copy</button>"##,
        id_esc = id_esc,
        kind_label_esc = kind_label_esc,
    );
    let id_short = id.get(..8).unwrap_or(id);
    let id_short_esc = html_escape(id_short);
    let body = match kind {
        "tenant" => format!(
            r#"<a class="mono fs-12" href="{base_url}/manage/tenants/{id_esc}" title="{id_esc}">{kind_label_esc} · {id_short_esc}…</a>"#,
            base_url = base_url,
            id_esc = id_esc,
            id_short_esc = id_short_esc,
            kind_label_esc = kind_label_esc,
        ),
        "archetype" => format!(
            r#"<a class="mono fs-12" href="{base_url}/manage/archetypes/{id_esc}">{kind_label_esc} · {id_esc}</a>"#,
            base_url = base_url,
            id_esc = id_esc,
            kind_label_esc = kind_label_esc,
        ),
        _ => format!(
            r#"<span class="mono fs-12 muted" title="{id_esc}">{kind_label_esc} · {id_short_esc}…</span>"#,
            id_esc = id_esc,
            id_short_esc = id_short_esc,
            kind_label_esc = kind_label_esc,
        ),
    };
    format!(
        r#"<div class="row gap-8" style="align-items:center">{body}{copy}</div>"#,
        body = body,
        copy = copy_btn,
    )
}

fn scheduled_grants_table(scheduled: &[crate::types::ScheduledGrant], base_url: &str) -> String {
    if scheduled.is_empty() {
        return empty_state(
            "No scheduled grants yet",
            "Add a recurring grant below to automatically credit every tenant on a calendar cadence.",
            None,
        );
    }

    let rows: String = scheduled
        .iter()
        .map(|g| {
            let expiry = if g.expires_in_days <= 0 {
                "never".to_string()
            } else {
                format!("{}d", g.expires_in_days)
            };
            let last_run = g.last_run_at.as_deref().unwrap_or("–");
            let active = if g.active { "active" } else { "off" };
            format!(
                r##"<tr>
  <td>{cadence}</td>
  <td class="ta-right mono">{credits}</td>
  <td class="mono fs-12">{expiry}</td>
  <td class="mono fs-11">{last_run}</td>
  <td class="mono fs-11">{next_run}</td>
  <td><span class="chip">{active}</span></td>
  <td>
    <form hx-delete="{base_url}/manage/billing/schedule/{id}" hx-target="body" hx-swap="innerHTML" hx-confirm="Remove this scheduled grant?">
      <button class="btn ghost sm danger" type="submit">Remove</button>
    </form>
  </td>
</tr>"##,
                cadence = html_escape(g.cadence.label()),
                credits = g.credits,
                expiry = expiry,
                last_run = html_escape(last_run.get(..16).unwrap_or(last_run)),
                next_run = html_escape(g.next_run_at.get(..16).unwrap_or(&g.next_run_at)),
                active = active,
                base_url = base_url,
                id = html_escape(&g.id),
            )
        })
        .collect();

    format!(
        r##"<div class="card no-pad" style="overflow-x:auto">
            <table class="manage-table fs-13">
              <thead>
                <tr>
                  <th>Cadence</th>
                  <th class="ta-right">Credits</th>
                  <th>Expiry</th>
                  <th>Last run</th>
                  <th>Next run</th>
                  <th>Status</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>{rows}</tbody>
            </table>
          </div>"##,
        rows = rows,
    )
}

pub fn billing_overview_html(
    actor_email: &str,
    base_url: &str,
    locale: &Locale,
    cfg: &crate::storage::Pricing,
    scheduled: &[crate::types::ScheduledGrant],
    schedule_form_msg: Option<&str>,
) -> String {
    let pricing_table = pricing_form_table(cfg, base_url);
    let scheduled_rows = scheduled_grants_table(scheduled, base_url);
    let schedule_msg = schedule_form_msg.unwrap_or("");

    let header = manage_header("Billing", "Pricing & grants", None, None, "");
    let content = format!(
        r##"<div class="page-pad">
  {header}

  <!-- Pricing card has two destructive in-page actions (Remove
       currency column, Add currency) that swap the whole body — if
       the operator has typed into a price cell and clicks one, those
       edits vanish. The x-data wrapper here flips `dirty` on any
       nested @input and re-disables the destructive actions until
       Save pricing succeeds. The `htmx:after-request` listener clears
       dirty when the /settings POST returns successfully. -->
  <div class="card p-22 mb-16"
       x-data="{{ dirty: false }}"
       @input.capture="dirty = true"
       @htmx:after-request="if ($event.detail.successful && $event.detail.requestConfig.path.endsWith('/billing/settings')) dirty = false">
    <h3 class="mb-8">Pricing</h3>
    <p class="muted mb-12">One column per supported currency. Each cell is the price in that currency's own unit (no conversion). The unit caption under each row shows whether the value is in minor units or milli-minor units.</p>
    <form hx-post="{base_url}/manage/billing/settings" hx-target="{hash}toast-region" hx-swap="afterbegin" hx-ext="json-enc">
      {pricing_table}

      <div class="row gap-12 wrap mb-12 mt-16">
        <label class="flex-1" style="min-width:240px">
          <div class="eyebrow mb-4">Addresses per reply-email pack</div>
          <input class="input mono" name="email_pack_size" type="number" min="1" required value="{email_pack_size}">
          <div class="muted fs-11 mt-4">currency-independent · tenants receive this many addresses per active pack</div>
        </label>
      </div>

      <div class="row gap-12 mt-12 wrap" style="align-items:center">
        <button class="btn sm" type="submit">Save pricing</button>
        <span class="muted fs-12" x-show="dirty" x-cloak>Unsaved changes — save before removing or adding a currency.</span>
      </div>
    </form>
  </div>

  <div class="card p-22 mb-16">
    <h3 class="mb-8">Recurring credit grants</h3>
    <p class="muted mb-12">Automated grants that fire on a calendar cadence and apply to every tenant. Use these in place of a baked-in monthly free allowance.</p>
    {scheduled_rows}
    <div id="schedule-toast">{schedule_msg}</div>
    <form hx-post="{base_url}/manage/billing/schedule" hx-target="body" hx-swap="innerHTML" hx-ext="json-enc" class="mt-12">
      <div class="row gap-12 wrap mb-12" style="align-items:flex-end">
        <label style="min-width:220px">
          <div class="eyebrow lbl">Cadence</div>
          <select class="select" name="cadence" required>
            <option value="monthly_first">1st of every month</option>
            <option value="weekly_mon">Every Monday</option>
            <option value="weekly_tue">Every Tuesday</option>
            <option value="weekly_wed">Every Wednesday</option>
            <option value="weekly_thu">Every Thursday</option>
            <option value="weekly_fri">Every Friday</option>
            <option value="weekly_sat">Every Saturday</option>
            <option value="weekly_sun">Every Sunday</option>
            <option value="daily">Every day at 00:00 UTC</option>
          </select>
        </label>
        <label class="stack">
          <span class="eyebrow lbl">Credits</span>
          <input class="input mono w-input-xs" name="credits" type="number" min="1" required placeholder="e.g. 50">
        </label>
        <label class="stack">
          <span class="eyebrow lbl">Expires in (days, 0 = never)</span>
          <input class="input mono w-input-sm" name="expires_in_days" type="number" min="0" value="0" required>
        </label>
        <button class="btn sm" type="submit">Add scheduled grant</button>
      </div>
    </form>
  </div>
</div>"##,
        header = header,
        base_url = base_url,
        hash = HASH,
        pricing_table = pricing_table,
        email_pack_size = cfg.email_pack_size,
        scheduled_rows = scheduled_rows,
        schedule_msg = schedule_msg,
    );

    manage_shell(
        "Billing · Concierge",
        &content,
        "Billing",
        actor_email,
        base_url,
        locale,
    )
}

/// Build the per-(concept, currency) input grid plus the "add currency"
/// row. Field names follow `<concept>__<CODE>` so the settings POST can
/// dispatch to `upsert_pricing_amount` cell-by-cell.
fn pricing_form_table(cfg: &crate::storage::Pricing, base_url: &str) -> String {
    use crate::storage::PricingConcept;
    let codes = cfg.currencies();
    if codes.is_empty() {
        return r#"<p class="muted fs-13 m-0">No currencies configured. Use the "Add currency" form below.</p>"#.to_string()
            + &add_currency_form(base_url, &[]);
    }

    // Header row: concept label + one column per currency.
    let header_cells: String = codes
        .iter()
        .map(|c| {
            let info = currency_info(c);
            let remove = format!(
                r##" <button type="button" class="btn ghost sm danger" :disabled="dirty" hx-delete="{base_url}/manage/billing/currency/{code}" hx-confirm="Remove all {code} prices?" hx-target="body" hx-swap="innerHTML" :title="dirty ? 'Save pricing first to remove a currency' : 'Remove {code} from pricing'">Remove</button>"##,
                base_url = base_url,
                code = html_escape(c),
            );
            format!(
                r##"<th class="ta-right">
  <div class="row gap-8" style="justify-content:flex-end;align-items:center">
    <span class="mono">{symbol} {code}</span>{remove}
  </div>
  <div class="muted fs-11">{name}</div>
</th>"##,
                symbol = html_escape(&info.symbol),
                code = html_escape(c),
                name = html_escape(&info.name),
                remove = remove,
            )
        })
        .collect();

    let body_rows: String = PricingConcept::ALL
        .iter()
        .map(|concept| {
            let cells: String = codes
                .iter()
                .map(|code| {
                    let value = cfg.amount(*concept, code).unwrap_or(0);
                    format!(
                        r##"<td><input class="input mono w-input-sm" name="{name}" type="number" min="1" required value="{value}"></td>"##,
                        name = format!("{}__{}", concept.as_wire(), code),
                        value = value,
                    )
                })
                .collect();
            format!(
                r##"<tr>
  <th class="ta-left" style="font-weight:600">
    <div>{label}</div>
    <div class="muted fs-11">{unit}</div>
  </th>
  {cells}
</tr>"##,
                label = html_escape(concept.label()),
                unit = html_escape(concept.unit_caption()),
                cells = cells,
            )
        })
        .collect();

    format!(
        r##"<div class="card no-pad" style="overflow-x:auto">
  <table class="manage-table fs-13" style="width:100%">
    <thead><tr><th></th>{header_cells}</tr></thead>
    <tbody>{body_rows}</tbody>
  </table>
</div>
{add_form}"##,
        header_cells = header_cells,
        body_rows = body_rows,
        add_form = add_currency_form(base_url, &codes),
    )
}

/// Tiny inline form to add a new currency to the pricing table. Submits to
/// the same `/settings` endpoint with default-valued cells so the new
/// column appears immediately on next render.
fn add_currency_form(base_url: &str, existing: &[String]) -> String {
    use crate::storage::PricingConcept;
    let already: std::collections::HashSet<&str> = existing.iter().map(|s| s.as_str()).collect();
    let popular = [
        "INR", "USD", "EUR", "GBP", "JPY", "AUD", "CAD", "AED", "SGD", "ZAR",
    ];
    let options: String = popular
        .iter()
        .filter(|c| !already.contains(*c))
        .map(|c| {
            let info = currency_info(c);
            format!(
                r#"<option value="{code}">{code} · {name} ({symbol})</option>"#,
                code = c,
                name = html_escape(&info.name),
                symbol = html_escape(&info.symbol),
            )
        })
        .collect();

    // Each new-currency POST goes back through /settings with the chosen
    // code and seed values for every concept. Defaults seed to 1 minor /
    // milli-minor; the operator edits to taste afterwards.
    let seed_inputs: String = PricingConcept::ALL
        .iter()
        .map(|concept| {
            let default = if concept.is_milli() { 100 } else { 100 };
            format!(
                r#"<input type="hidden" :name="`{key}__${{currency}}`" :value="{default}">"#,
                key = concept.as_wire(),
                default = default,
            )
        })
        .collect();

    format!(
        r##"<div x-data="{{ currency: '' }}" class="row gap-12 mt-12 wrap" style="align-items:flex-end">
  <label style="min-width:260px">
    <div class="eyebrow mb-4">Add currency</div>
    <select class="input" x-model="currency">
      <option value="">Pick a currency…</option>
      {options}
    </select>
  </label>
  <form hx-post="{base_url}/manage/billing/settings" hx-target="body" hx-swap="innerHTML" hx-ext="json-enc"
        x-show="currency">
    <input type="hidden" name="__currencies" :value="JSON.stringify([currency])">
    {seed_inputs}
    <button class="btn sm" type="submit" :disabled="dirty" :title="dirty ? 'Save pricing first to add a currency' : ''">Add</button>
  </form>
</div>"##,
        base_url = base_url,
        options = options,
        seed_inputs = seed_inputs,
    )
}

/// Lookup symbol + display name for an ISO 4217 code via rusty_money.
/// Returns the code itself for unknown codes.
fn currency_info(code: &str) -> CurrencyDisplay {
    match rusty_money::iso::find(code) {
        Some(c) => CurrencyDisplay {
            symbol: c.symbol.to_string(),
            name: c.name.to_string(),
        },
        None => CurrencyDisplay {
            symbol: code.to_string(),
            name: code.to_string(),
        },
    }
}

struct CurrencyDisplay {
    symbol: String,
    name: String,
}

// =====================================================================
// Archetype catalog
// =====================================================================

pub fn archetypes_list_html(
    rows: &[crate::types::Archetype],
    query: &str,
    actor_email: &str,
    base_url: &str,
    locale: &Locale,
) -> String {
    let new_btn = format!(
        r#"<a class="btn primary sm" href="{base_url}/manage/archetypes/new">+ New archetype</a>"#,
        base_url = base_url,
    );

    let header = manage_header(
        "Archetype catalog",
        "Archetypes",
        None,
        Some("Every save runs through the safety classifier; only Approved rows are visible in the demo and onboarding."),
        &new_btn,
    );

    let table = archetypes_table_html(rows, base_url);

    let content = format!(
        r##"<div class="page-pad">
  {header}
  <div class="row gap-12 mb-12 wrap" style="align-items:center">
    <input
      class="input w-input-lg" type="search" name="q" value="{q}"
      placeholder="Search by slug, label, or description…"
      hx-get="{base_url}/manage/archetypes"
      hx-trigger="input changed delay:200ms, search"
      hx-target="{hash}archetypes-table" hx-swap="outerHTML"
      hx-push-url="true"
      autocomplete="off">
  </div>
  {table}
</div>"##,
        header = header,
        base_url = base_url,
        hash = HASH,
        q = html_escape(query),
        table = table,
    );
    manage_shell(
        "Archetypes · Concierge",
        &content,
        "Archetypes",
        actor_email,
        base_url,
        locale,
    )
}

/// Render just the `<div id="archetypes-table">` portion of the
/// archetypes list. Used both for the full page and the HTMX search
/// swap. The empty-state branches between "no archetypes ever" and
/// "no rows match this query" so the CTA fits the situation.
pub fn archetypes_table_html(rows: &[crate::types::Archetype], base_url: &str) -> String {
    let row_html: String = rows
        .iter()
        .map(|r| {
            let status_chip = persona_status_chip(&r.safety.status);
            let updated = r
                .updated_at
                .as_deref()
                .map(|s| s.get(..10).unwrap_or(s).to_string())
                .unwrap_or_default();
            format!(
                r##"<div class="rt-row" style="grid-template-columns:0.7fr 0.7fr 1.4fr 0.6fr 0.5fr 80px">
  <div><a href="{base_url}/manage/archetypes/{slug}"><strong>{slug}</strong></a></div>
  <div>{label}</div>
  <div class="muted fs-13">{description}</div>
  <div>{status}</div>
  <div class="mono muted fs-11">{updated}</div>
  <div><a class="btn ghost sm" href="{base_url}/manage/archetypes/{slug}">Edit</a></div>
</div>"##,
                base_url = base_url,
                slug = html_escape(&r.slug),
                label = html_escape(&r.label),
                description = html_escape(&r.description),
                status = status_chip,
                updated = html_escape(&updated),
            )
        })
        .collect();

    let body = if rows.is_empty() {
        empty_state(
            "No archetypes match",
            "Try a different search, or clear the box to see every archetype.",
            None,
        )
    } else {
        format!(
            r##"<div class="rt-head" style="grid-template-columns:0.7fr 0.7fr 1.4fr 0.6fr 0.5fr 80px">
  <div>Slug</div><div>Label</div><div>Description</div><div>Safety</div><div>Updated</div><div></div>
</div>{rows}"##,
            rows = row_html,
        )
    };

    let count_line = format!(
        r#"<div class="muted fs-12 mb-8">{n} archetype{s}</div>"#,
        n = rows.len(),
        s = if rows.len() == 1 { "" } else { "s" },
    );

    format!(
        r##"<div id="archetypes-table">
  {count_line}
  <div class="card" style="padding:0;overflow:hidden">{body}</div>
</div>"##,
        count_line = count_line,
        body = body,
    )
}

fn persona_status_chip(status: &PersonaSafetyStatus) -> String {
    match status {
        PersonaSafetyStatus::Approved => r#"<span class="chip ok">approved</span>"#.to_string(),
        PersonaSafetyStatus::Pending => {
            r#"<span class="chip warn">draft / classifying</span>"#.to_string()
        }
        PersonaSafetyStatus::Rejected => r#"<span class="chip warn">rejected</span>"#.to_string(),
    }
}

pub fn archetype_edit_html(
    row: Option<&crate::types::Archetype>,
    actor_email: &str,
    base_url: &str,
    locale: &Locale,
) -> String {
    let is_new = row.is_none();
    let blank_row;
    let row_ref = match row {
        Some(r) => r,
        None => {
            blank_row = crate::types::Archetype {
                slug: String::new(),
                label: String::new(),
                description: String::new(),
                voice_prompt: String::new(),
                greeting: String::new(),
                default_rules_json: "[]".to_string(),
                catch_phrases: vec![],
                off_topics: vec![],
                never: String::new(),
                handoff_conditions: vec![],
                safety: crate::types::PersonaSafety::default(),
                created_at: None,
                updated_at: None,
            };
            &blank_row
        }
    };

    let action = if is_new {
        format!("{base_url}/manage/archetypes/new")
    } else {
        format!("{base_url}/manage/archetypes/{}", row_ref.slug)
    };

    // Header: safety chip + classifier detail go in the right slot so
    // they're always visible in the sticky page header instead of in a
    // standalone card the user has to scroll past.
    let safety_chip = persona_status_chip(&row_ref.safety.status);
    let safety_detail = match (
        &row_ref.safety.status,
        row_ref.safety.checked_at.as_deref(),
        row_ref.safety.vague_reason.as_deref(),
    ) {
        (PersonaSafetyStatus::Approved, Some(at), _) => format!("Approved {at}"),
        (PersonaSafetyStatus::Rejected, _, Some(reason)) => {
            format!("Rejected: {}", html_escape(reason))
        }
        (PersonaSafetyStatus::Rejected, _, None) => "Rejected".to_string(),
        _ => "Awaiting classifier".to_string(),
    };
    let header_right = format!(
        r#"<div class="row gap-8" style="align-items:center">{chip}<span class="fs-12 muted">{detail}</span></div>"#,
        chip = safety_chip,
        detail = safety_detail,
    );

    let delete_button = if is_new {
        String::new()
    } else {
        format!(
            r##"<form hx-post="{action}/delete" hx-target="body" hx-swap="innerHTML" hx-confirm="Delete this archetype? This cannot be undone." style="display:inline">
              <button type="submit" class="btn ghost sm danger">Delete archetype</button>
            </form>"##,
            action = action,
        )
    };

    let slug_field = if is_new {
        r#"<div class="mt-12">
              <label for="archetype-slug" class="eyebrow lbl">Slug (lowercase, _ or -)</label>
              <input id="archetype-slug" class="input w-input-md mono" name="slug" required pattern="[a-z0-9_-]+">
            </div>"#
            .to_string()
    } else {
        format!(
            r#"<input type="hidden" name="slug" value="{}">"#,
            html_escape(&row_ref.slug)
        )
    };

    let title = if is_new {
        "New archetype".to_string()
    } else {
        html_escape(&row_ref.label)
    };
    let subtitle = if is_new {
        Some("Define a new persona's tone and initial rules.".to_string())
    } else {
        Some(format!("Slug: <code>{}</code>", html_escape(&row_ref.slug)))
    };
    let header = manage_header(
        if is_new {
            "New archetype"
        } else {
            "Edit archetype"
        },
        &title,
        Some((
            &format!("{base_url}/manage/archetypes"),
            "Back to archetypes",
        )),
        subtitle.as_deref(),
        &header_right,
    );

    // Three list-of-string fields become Alpine chip inputs. The
    // existing handler still parses newline-joined strings, so we keep
    // that contract via the hidden textarea inside each component.
    let phrases_input = chip_input(
        "catch_phrases",
        "archetype-phrases",
        &row_ref.catch_phrases,
        "Type a phrase and press Enter",
    );
    let off_input = chip_input(
        "off_topics",
        "archetype-off",
        &row_ref.off_topics,
        "Type a topic and press Enter",
    );
    let handoff_input = chip_input(
        "handoff_conditions",
        "archetype-handoff",
        &row_ref.handoff_conditions,
        "Type a condition and press Enter",
    );

    let content = format!(
        r##"<div class="page-pad">
  {header}

  <div class="card p-22">
    <form hx-post="{action}" hx-ext="json-enc" hx-target="body" hx-swap="innerHTML">

      <section class="form-section">
        <p class="section-eyebrow">Identity</p>
        <p class="section-help">How operators and tenants pick this persona out of the catalog.</p>
        <div>
          <label for="archetype-label" class="eyebrow lbl">Label</label>
          <input id="archetype-label" class="input" name="label" value="{label}" required>
        </div>
        {slug_field}
        <div class="mt-12">
          <label for="archetype-desc" class="eyebrow lbl">Description</label>
          <input id="archetype-desc" class="input" name="description" value="{description}">
        </div>
      </section>

      <section class="form-section">
        <p class="section-eyebrow">Voice</p>
        <p class="section-help">The opening line and the tone the model should adopt. Catch-phrases bias the model toward in-persona language; the safety classifier reads the voice prompt to decide approval.</p>
        <div>
          <label for="archetype-greeting" class="eyebrow lbl">Greeting</label>
          <input id="archetype-greeting" class="input" name="greeting" value="{greeting}" required>
        </div>
        <div class="mt-12">
          <label for="archetype-voice" class="eyebrow lbl">Voice prompt</label>
          <textarea id="archetype-voice" class="textarea mono" name="voice_prompt" rows="8" required>{voice_prompt}</textarea>
        </div>
        <div class="mt-12">
          <label for="archetype-phrases" class="eyebrow lbl">Catch-phrases</label>
          {phrases_input}
        </div>
      </section>

      <section class="form-section">
        <p class="section-eyebrow">Constraints</p>
        <p class="section-help">Hard guardrails the persona must respect, plus the topics and signals that should trigger handoff to a human.</p>
        <div>
          <label for="archetype-never" class="eyebrow lbl">Never (policy constraints)</label>
          <input id="archetype-never" class="input" name="never" value="{never}">
        </div>
        <div class="mt-12">
          <label for="archetype-off" class="eyebrow lbl">Off-topics</label>
          {off_input}
        </div>
        <div class="mt-12">
          <label for="archetype-handoff" class="eyebrow lbl">Handoff conditions</label>
          {handoff_input}
        </div>
      </section>

      <section class="form-section" x-data="{{ jsonErr: '', validate(v) {{ if (!v) {{ this.jsonErr = ''; return; }} try {{ JSON.parse(v); this.jsonErr = ''; }} catch (e) {{ this.jsonErr = e.message; }} }} }}" x-init="validate($refs.rulesTa.value)">
        <p class="section-eyebrow">Engine</p>
        <p class="section-help">Default conversation rules applied to every tenant who picks this archetype. JSON is validated as you type.</p>
        <div class="json-field">
          <label for="archetype-rules" class="eyebrow lbl">Default rules (JSON)</label>
          <textarea id="archetype-rules" x-ref="rulesTa" class="textarea mono"
                    :class="jsonErr ? 'invalid' : ''"
                    name="default_rules_json" rows="10" required
                    @input="validate($event.target.value)">{rules_json}</textarea>
          <p class="json-error" x-show="jsonErr" x-text="jsonErr" x-cloak></p>
        </div>
      </section>

      <div class="between pt-16 mt-24" style="border-top:1px solid var(--hair)">
        <div>{delete_button}</div>
        <button type="submit" class="btn primary">Save archetype</button>
      </div>
    </form>
  </div>
</div>"##,
        header = header,
        delete_button = delete_button,
        action = action,
        label = html_escape(&row_ref.label),
        slug_field = slug_field,
        description = html_escape(&row_ref.description),
        greeting = html_escape(&row_ref.greeting),
        voice_prompt = html_escape(&row_ref.voice_prompt),
        never = html_escape(&row_ref.never),
        phrases_input = phrases_input,
        off_input = off_input,
        handoff_input = handoff_input,
        rules_json = html_escape(&row_ref.default_rules_json),
    );

    manage_shell(
        "Archetype · Concierge",
        &content,
        "Archetypes",
        actor_email,
        base_url,
        locale,
    )
}

/// Chip-input widget for editing list-of-string fields. Keeps the
/// existing form contract (newline-joined string) via a hidden input
/// so the server-side parser doesn't need to change.
///
/// The Alpine state lives on the wrapper. Initial values are
/// JSON-encoded into the `x-data` expression. The server-side caller
/// is responsible for placing a `<label for="{id}">` adjacent to the
/// returned markup; the `id` attaches to the visible draft input so
/// screen readers announce the right field name on focus.
fn chip_input(name: &str, id: &str, items: &[String], placeholder: &str) -> String {
    let initial_json = serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string());
    // JSON inside a single-quoted HTML attribute: `&` and `'` are the
    // only chars that break that delimiter. html_escape handles both.
    let initial_attr = html_escape(&initial_json);
    format!(
        r##"<div class="chip-input" x-data='{{ items: {initial_attr}, draft: "" }}'>
  <div class="chips">
    <template x-for="(it, idx) in items" :key="idx">
      <span class="pill">
        <span x-text="it"></span>
        <button type="button" aria-label="Remove" @click="items.splice(idx, 1)">×</button>
      </span>
    </template>
  </div>
  <input id="{id}" type="text" class="draft" x-model="draft"
         placeholder="{placeholder}"
         @keydown.enter.prevent="if (draft.trim()) {{ items.push(draft.trim()); draft = ''; }}"
         @keydown.backspace="if (!draft && items.length) items.pop()"
         @blur="if (draft.trim()) {{ items.push(draft.trim()); draft = ''; }}">
  <input type="hidden" name="{name}" :value="items.join('\n')">
  <p class="hint">Press Enter to add. Backspace on empty input removes the last chip.</p>
</div>"##,
        initial_attr = initial_attr,
        id = id,
        name = name,
        placeholder = html_escape(placeholder),
    )
}

// =====================================================================
// Demo controls (/manage/demo)
// =====================================================================

pub fn demo_config_html(
    cfg: &crate::storage::DemoConfig,
    stored: Option<&crate::storage::StoredDemoPersonas>,
    actor_email: &str,
    base_url: &str,
    locale: &Locale,
) -> String {
    let enabled_checked = if cfg.enabled { " checked" } else { "" };
    let prompt_value = html_escape(&cfg.persona_generation_prompt);
    let default_prompt = html_escape(crate::storage::DEFAULT_DEMO_GENERATION_PROMPT);
    let cadence = cfg.regeneration_cadence_mins;
    let idle_timeout_secs = cfg.idle_timeout_secs;
    let max_user_turns = cfg.max_user_turns;

    // Toggle card. Auto-submits on change via HTMX (the form has no
    // visible Save button) so the operator never has to click twice.
    // When the toggle is off, everything below this card is hidden.
    let toggle_card = format!(
        r##"<div class="card p-22 mb-16">
    <form hx-post="{base_url}/manage/demo/toggle" hx-ext="json-enc" hx-target="body" hx-swap="innerHTML" hx-trigger="change from:#demo-enabled">
      <div class="row gap-12" style="align-items:center">
        <input id="demo-enabled" type="checkbox" name="enabled" value="true"{enabled_checked}>
        <label for="demo-enabled" class="fw-600">Demo enabled</label>
        <span class="muted fs-13">When off: homepage hides the chat button.</span>
      </div>
    </form>
  </div>"##,
        base_url = base_url,
        enabled_checked = enabled_checked,
    );

    let header = manage_header("Demo controls", "Live homepage demo", None, None, "");

    if !cfg.enabled {
        let content = format!(
            r##"<div class="page-pad">
  {header}
  {toggle_card}
</div>"##,
            header = header,
            toggle_card = toggle_card,
        );
        return manage_shell(
            "Demo · Concierge",
            &content,
            "Demo",
            actor_email,
            base_url,
            locale,
        );
    }

    let stored_block = stored
        .map(stored_personas_card)
        .unwrap_or_else(|| {
            r#"<p class="muted fs-13 m-0">No personas rolled yet. Hit "Re-roll" to generate the first set, or wait for the next cron tick.</p>"#
                .to_string()
        });
    let stored_meta = match stored {
        Some(s) => format!(
            "Last rolled {ts} · {n} personas",
            ts = html_escape(&s.generated_at),
            n = s
                .response
                .get("personas")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0)
        ),
        None => "Nothing stored yet.".to_string(),
    };

    let content = format!(
        r##"<div class="page-pad" x-data="{{ promptDirty: false, previewOk: false }}">
  {header}

  {toggle_card}

  <!-- Single card, single form, but split into Timing and Prompt
       sections so the operator can scan to the relevant control.
       Save persists everything together (preview-gated only when the
       prompt is dirty); Preview / Re-roll are isolated to the Prompt
       section since they only act on persona generation. -->
  <div class="card p-22 mb-16">
    <form hx-post="{base_url}/manage/demo" hx-ext="json-enc" hx-target="body" hx-swap="innerHTML">
      <input type="hidden" name="prompt_verified" :value="String(previewOk || !promptDirty)">

      <section class="form-section">
        <p class="section-eyebrow">Timing</p>
        <p class="section-help">How often the persona blob refreshes, and how long a homepage visitor can chat before the sign-up CTA takes over the input.</p>
        <div class="row gap-12 mb-12 wrap" style="align-items:center">
          <label for="demo-cadence" class="fw-600">Regenerate every</label>
          <input id="demo-cadence" class="input mono w-input-xs" type="number" name="regeneration_cadence_mins" min="0" max="10080" value="{cadence}">
          <span class="muted fs-13">minutes (0 = manual only).</span>
        </div>
        <div class="row gap-12 mb-12 wrap" style="align-items:center">
          <label for="demo-turns" class="fw-600">User turns per session</label>
          <input id="demo-turns" class="input mono w-input-xs" type="number" name="max_user_turns" min="1" max="20" value="{max_user_turns}">
          <span class="muted fs-13">replaces the chat input with the sign-up CTA after this many user messages.</span>
        </div>
        <div class="row gap-12 wrap" style="align-items:center">
          <label for="demo-idle" class="fw-600">Idle timeout</label>
          <input id="demo-idle" class="input mono w-input-xs" type="number" name="idle_timeout_secs" min="5" max="600" value="{idle_timeout_secs}">
          <span class="muted fs-13">seconds — restarts on every keystroke; fires the CTA when the visitor stops typing.</span>
        </div>
      </section>

      <section class="form-section">
        <div class="between wrap mb-4" style="gap:12px;align-items:flex-start">
          <div>
            <p class="section-eyebrow">Prompt &amp; personas</p>
            <p class="section-help m-0">{stored_meta}</p>
          </div>
        </div>
        <p class="section-help">The prompt is sent as the system message to the LLM with each archetype's label + description as the user message. The model must reply with a JSON array of objects, one per archetype in the same order. Each object: <code>name</code>, <code>business_type</code>, <code>city</code>, <code>hours</code>, <code>goal</code>, <code>goal_url</code>. Leave blank to restore the default.</p>
        <div>
          <label for="demo-prompt" class="eyebrow lbl">Persona generation prompt</label>
          <textarea id="demo-prompt" class="textarea mono" name="persona_generation_prompt" rows="6"
                    placeholder="{default_prompt}"
                    @input="promptDirty = true; previewOk = false">{prompt_value}</textarea>
        </div>

        <div class="row gap-8 mt-12 wrap" style="align-items:center">
          <button type="button" class="btn ghost sm"
                  hx-post="{base_url}/manage/demo/preview"
                  hx-include="[name='persona_generation_prompt']"
                  hx-target="{hash}demo-display"
                  hx-swap="innerHTML"
                  hx-ext="json-enc">
            <span>Preview</span>
            <span class="spinner htmx-indicator" aria-hidden="true"></span>
          </button>
          <button type="button" class="btn ghost sm"
                  hx-post="{base_url}/manage/demo/reroll"
                  hx-target="body" hx-swap="innerHTML">
            <span>Re-roll</span>
            <span class="spinner htmx-indicator" aria-hidden="true"></span>
          </button>
          <button type="submit" class="btn primary"
                  :disabled="promptDirty && !previewOk">
            <span>Save</span>
            <span class="spinner htmx-indicator" aria-hidden="true"></span>
          </button>
          <span class="muted fs-12" x-show="promptDirty && !previewOk" x-cloak>Preview must succeed before saving.</span>
        </div>

        <!-- Shared display pane. Server-rendered with the currently
             stored personas on first paint; swapped to the preview
             result after a Preview click. The `@htmx:after-swap`
             listener watches for a `.preview-ok` marker the success
             template emits and flips the Save gate accordingly. -->
        <div id="demo-display" class="mt-16" style="border-top:1px solid var(--hair);padding-top:16px"
             @htmx:after-swap="previewOk = !!document.querySelector('#demo-display .preview-ok')">
          {stored_block}
        </div>
      </section>
    </form>
  </div>
</div>"##,
        header = header,
        base_url = base_url,
        hash = HASH,
        toggle_card = toggle_card,
        prompt_value = prompt_value,
        default_prompt = default_prompt,
        cadence = cadence,
        idle_timeout_secs = idle_timeout_secs,
        max_user_turns = max_user_turns,
        stored_meta = stored_meta,
        stored_block = stored_block,
    );

    manage_shell(
        "Demo · Concierge",
        &content,
        "Demo",
        actor_email,
        base_url,
        locale,
    )
}

/// Render the saved persona blob as a list of business cards. Reads
/// fields out of the stored serde_json::Value defensively so a stale
/// shape doesn't crash the page.
fn stored_personas_card(stored: &crate::storage::StoredDemoPersonas) -> String {
    let arr = match stored.response.get("personas").and_then(|v| v.as_array()) {
        Some(a) if !a.is_empty() => a,
        _ => {
            return r#"<p class="muted fs-13 m-0">Stored blob has no personas. Re-roll to refresh.</p>"#.to_string();
        }
    };
    arr.iter()
        .map(|p| {
            let s = |k: &str| {
                p.get(k)
                    .and_then(|v| v.as_str())
                    .map(html_escape)
                    .unwrap_or_default()
            };
            let biz = p.get("business");
            let bs = |k: &str| {
                biz.and_then(|b| b.get(k))
                    .and_then(|v| v.as_str())
                    .map(html_escape)
                    .unwrap_or_default()
            };
            format!(
                r##"<div class="card p-14 mb-8">
  <div class="row gap-8 mb-6" style="align-items:baseline">
    <span class="chip">{slug}</span>
    <strong>{name}</strong>
    <span class="muted fs-13">{biz_type}{sep}{city}</span>
  </div>
  <div class="muted fs-13"><b>Hours:</b> {hours}</div>
  <div class="muted fs-13"><b>Goal:</b> {goal}{goal_url_block}</div>
</div>"##,
                slug = s("slug"),
                name = if bs("name").is_empty() {
                    s("label")
                } else {
                    bs("name")
                },
                biz_type = bs("business_type"),
                sep = if !bs("business_type").is_empty() && !bs("city").is_empty() {
                    " · "
                } else {
                    ""
                },
                city = bs("city"),
                hours = if bs("hours").is_empty() {
                    "—".to_string()
                } else {
                    bs("hours")
                },
                goal = if bs("goal").is_empty() {
                    "—".to_string()
                } else {
                    bs("goal")
                },
                goal_url_block = if bs("goal_url").is_empty() {
                    String::new()
                } else {
                    format!(r#" <span class="mono fs-12">({})</span>"#, bs("goal_url"))
                },
            )
        })
        .collect()
}

pub fn demo_preview_success_html(
    archetypes: &[crate::types::Archetype],
    businesses: &[crate::handlers::demo_personas_list::DemoBusiness],
) -> String {
    let rows: String = archetypes
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let biz = businesses.get(i);
            let (name, biz_type, city, hours, goal, goal_url) = match biz {
                Some(b) => (
                    html_escape(&b.name),
                    html_escape(&b.business_type),
                    html_escape(&b.city),
                    html_escape(&b.hours),
                    html_escape(&b.goal),
                    html_escape(&b.goal_url),
                ),
                None => (
                    "—".to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ),
            };
            format!(
                r##"<div class="card p-14 mb-8">
  <div class="row gap-8 mb-6" style="align-items:baseline">
    <span class="chip">{slug}</span>
    <strong>{name}</strong>
    <span class="muted fs-13">{biz_type}{biz_type_sep}{city}</span>
  </div>
  <div class="muted fs-13"><b>Hours:</b> {hours}</div>
  <div class="muted fs-13"><b>Goal:</b> {goal}{goal_url_block}</div>
</div>"##,
                slug = html_escape(&a.slug),
                name = name,
                biz_type = biz_type,
                biz_type_sep = if biz.is_some() { " · " } else { "" },
                city = city,
                hours = if hours.is_empty() {
                    "—".to_string()
                } else {
                    hours
                },
                goal = if goal.is_empty() {
                    "—".to_string()
                } else {
                    goal
                },
                goal_url_block = if goal_url.is_empty() {
                    String::new()
                } else {
                    format!(r#" <span class="mono fs-12">({goal_url})</span>"#)
                },
            )
        })
        .collect();

    // Hidden field carries the just-rolled businesses back to the Save
    // POST so the operator's verified preview becomes the persisted
    // blob (no second non-deterministic LLM roll on save). Lives inside
    // #demo-display, which itself lives inside the form, so it's
    // submitted automatically. Cleared whenever the operator edits the
    // prompt (the textarea's @input flips previewOk false but the
    // hidden field stays — the server gates on `prompt_verified`).
    let rolled_json = serde_json::to_string(businesses).unwrap_or_else(|_| "[]".to_string());

    // The `preview-ok` class is the signal the management form's
    // Alpine listener watches for to re-enable Save after a clean
    // preview. Don't drop it.
    format!(
        r#"<input type="hidden" name="rolled_personas_json" value="{rolled}">
<span class="chip ok mb-12 preview-ok" style="display:inline-block">Parsed OK · {n} entries</span>{rows}"#,
        rolled = html_escape(&rolled_json),
        n = businesses.len(),
        rows = rows,
    )
}

pub fn demo_preview_shape_mismatch_html(expected: usize, got: usize) -> String {
    format!(
        r#"<div class="error mb-8">Preview returned {got} entries for {expected} archetypes — the JSON array length must match the archetype count exactly. Save is blocked.</div>"#,
        expected = expected,
        got = got,
    )
}

pub fn demo_preview_error_html(message: &str) -> String {
    format!(
        r#"<div class="error mb-8">Preview failed — fix the prompt and try again.</div>
<pre class="prompt-preview" style="white-space:pre-wrap">{}</pre>"#,
        html_escape(message),
    )
}
