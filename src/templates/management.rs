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
    base_url: &str,
    locale: &Locale,
) -> String {
    let nav_items = [
        ("Dashboard", "/manage"),
        ("Tenants", "/manage/tenants"),
        ("Personas", "/manage/personas"),
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

    let inner = format!(
        r##"<div class="app">
  <header class="app-top" style="border-bottom-color:var(--accent)">
    {brand}
    <nav class="app-nav">{nav}</nav>
    <div class="row gap-12">
      <span class="chip warn">management</span>
    </div>
  </header>
  {content}
</div>"##,
        brand = brand_mark(),
        nav = nav,
        content = content,
    );

    base_html(title, &inner, locale)
}

pub fn dashboard_html(
    email: &str,
    tenant_count: usize,
    health: &crate::handlers::health::HealthReport,
    base_url: &str,
    locale: &Locale,
) -> String {
    let health_panel = health_panel_html(health);
    let content = format!(
        r##"<div class="page-pad">
  <div class="between mb-24">
    <div>
      <div class="eyebrow">Management Panel</div>
      <h2 class="display-sm m-0 mt-4">Welcome, {email}</h2>
    </div>
  </div>
  <div class="mb-24" style="display:grid;grid-template-columns:repeat(3,1fr);gap:16px">
    <div class="card p-18 ta-center">
      <div class="stat-n serif">{tenant_count}</div>
      <div class="mono muted fs-11">Tenants</div>
    </div>
    <div class="card p-18 ta-center">
      <div class="stat-n serif">—</div>
      <div class="mono muted fs-11">MRR</div>
    </div>
    <div class="card p-18 ta-center">
      <div class="stat-n serif">—</div>
      <div class="mono muted fs-11">Active</div>
    </div>
  </div>

  {health_panel}

  <div class="card p-18 mt-16">
    <div class="between">
      <div class="eyebrow">Quick actions</div>
    </div>
    <div class="row gap-12 mt-12">
      <a href="{base_url}/manage/tenants" class="btn sm">View tenants</a>
      <a href="{base_url}/manage/audit" class="btn ghost sm">Audit log</a>
    </div>
  </div>
</div>"##,
        email = html_escape(email),
        tenant_count = tenant_count,
        health_panel = health_panel,
        base_url = base_url,
    );

    manage_shell(
        "Management - Concierge",
        &content,
        "Dashboard",
        base_url,
        locale,
    )
}

fn health_panel_html(report: &crate::handlers::health::HealthReport) -> String {
    use crate::handlers::health::Status;
    let overall_chip = match report.overall {
        Status::Ok => r#"<span class="chip ok">All systems normal</span>"#,
        Status::Warn => r#"<span class="chip warn">Degraded</span>"#,
        Status::Error => {
            r#"<span class="chip warn" style="background:#FCE8D5;border-color:#E08070;color:#8A1F0E">Issues detected</span>"#
        }
    };
    let rows: String = report
        .checks
        .iter()
        .map(|c| {
            let dot = match c.status {
                Status::Ok => r#"<span class="dot ok"></span>"#,
                Status::Warn => r#"<span class="dot" style="background:var(--warn)"></span>"#,
                Status::Error => {
                    r#"<span class="dot" style="background:#C03020;box-shadow:0 0 0 3px rgba(192,48,32,.2)"></span>"#
                }
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

pub fn tenants_list_html(tenants: &[Tenant], base_url: &str, locale: &Locale) -> String {
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
    <button class="btn ghost sm btn-danger" hx-delete="{base_url}/manage/tenants/{id}" hx-confirm="Delete tenant {email} and ALL their data?" hx-target="closest .rt-row" hx-swap="outerHTML">Delete</button>
  </div>
</div>"##,
                base_url = base_url,
                id = html_escape(&t.id),
                email = html_escape(&t.email),
                name = html_escape(t.name.as_deref().unwrap_or("—")),
                plan = html_escape(t.plan.label()),
                created = html_escape(&t.created_at.get(..10).unwrap_or(&t.created_at)),
            )
        })
        .collect();

    let empty = if tenants.is_empty() {
        r##"<div class="muted p-20 ta-center">No tenants yet.</div>"##
    } else {
        ""
    };

    let content = format!(
        r##"<div class="page-pad">
  <div class="between mb-16">
    <div>
      <div class="eyebrow">All tenants</div>
      <h2 class="display-sm m-0 mt-4">{count} tenant{s}</h2>
    </div>
  </div>
  <div class="card" style="padding:0;overflow:hidden">
    <div class="rt-head" style="grid-template-columns:1fr 1fr 0.6fr 0.5fr 80px">
      <div>Email</div><div>Name</div><div>Plan</div><div>Created</div><div></div>
    </div>
    {rows}{empty}
  </div>
</div>"##,
        count = tenants.len(),
        s = if tenants.len() == 1 { "" } else { "s" },
        rows = rows,
        empty = empty,
    );

    manage_shell("Tenants - Concierge", &content, "Tenants", base_url, locale)
}

pub fn tenant_detail_html(
    tenant: &Tenant,
    wa: &[WhatsAppAccount],
    ig: &[InstagramAccount],
    addrs: &[EmailAddress],
    billing: &TenantBilling,
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

    let content = format!(
        r##"<div class="page-pad">
  <p><a href="{base_url}/manage/tenants">&larr; Back to tenants</a></p>
  <div class="between" style="margin:16px 0">
    <div>
      <div class="eyebrow">Tenant</div>
      <h2 class="display-sm">{email}</h2>
      <div class="muted">{name} &middot; {plan} &middot; joined {created}</div>
    </div>
    <button class="btn ghost sm btn-danger" hx-delete="{base_url}/manage/tenants/{id}" hx-confirm="Delete this tenant and ALL their data?">Delete tenant</button>
  </div>
  <div id="toast" role="status" aria-live="polite" aria-atomic="true"></div>
  <div class="card p-18 mb-16">
    <h3 class="mb-12">Plan</h3>
    <form hx-put="{base_url}/manage/tenants/{id}" hx-target="{hash}toast" hx-swap="innerHTML">
      <div class="row gap-12">
        <select class="select" name="plan" style="max-width:200px">
          {plan_options}
        </select>
        <button class="btn sm" type="submit">Update</button>
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

  <div class="card p-18 mt-16">
    <h3 class="mb-8">Grant free replies</h3>
    <p class="muted mb-12">Add reply credits to this tenant's balance. Current balance: <strong>{balance}</strong>.</p>
    <div id="grant-replies-toast"></div>
    <form hx-post="{base_url}/manage/tenants/{id}/grant-replies" hx-target="{hash}grant-replies-toast" hx-swap="innerHTML" hx-ext="json-enc">
      <div class="row gap-12 wrap">
        <input class="input" name="replies" placeholder="Replies" type="number" min="1" required style="max-width:160px">
        <input class="input" name="expires_days" placeholder="Expires in (days)" type="number" min="1" value="365" style="max-width:180px">
        <button class="btn sm" type="submit">Grant replies</button>
      </div>
    </form>
  </div>

  <div class="card p-18 mt-16">
    <h3 class="mb-8">Grant reply-email addresses</h3>
    <p class="muted mb-12">Add to this tenant's reply-email quota. Current quota: <strong>{quota}</strong> address(es).</p>
    <div id="grant-addresses-toast"></div>
    <form hx-post="{base_url}/manage/tenants/{id}/grant-addresses" hx-target="{hash}grant-addresses-toast" hx-swap="innerHTML" hx-ext="json-enc">
      <div class="row gap-12 wrap">
        <input class="input" name="addresses" placeholder="Address slots" type="number" min="1" required style="max-width:160px">
        <button class="btn sm" type="submit">Grant addresses</button>
      </div>
    </form>
  </div>
</div>"##,
        base_url = base_url,
        hash = HASH,
        id = html_escape(&tenant.id),
        email = html_escape(&tenant.email),
        name = html_escape(tenant.name.as_deref().unwrap_or("—")),
        plan = html_escape(tenant.plan.label()),
        created = html_escape(&tenant.created_at.get(..10).unwrap_or(&tenant.created_at)),
        plan_options = crate::types::Plan::ALL
            .iter()
            .map(|p| {
                let sel = if *p == tenant.plan { " selected" } else { "" };
                format!(
                    r#"<option value="{val}"{sel}>{label}</option>"#,
                    val = p.as_str(),
                    label = p.label(),
                )
            })
            .collect::<String>(),
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
        &format!("{} - Concierge", tenant.email),
        &content,
        "Tenants",
        base_url,
        locale,
    )
}

pub fn audit_html(log: &[serde_json::Value], base_url: &str, locale: &Locale) -> String {
    let rows: String = log
        .iter()
        .map(|entry| {
            let actor = entry
                .get("actor_email")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let action = entry.get("action").and_then(|v| v.as_str()).unwrap_or("?");
            let resource = entry
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

            format!(
                r##"<div class="rt-row" style="grid-template-columns:0.8fr 1fr 0.6fr 0.6fr 0.5fr">
  <div class="mono muted fs-11">{created}</div>
  <div>{actor}</div>
  <div><span class="chip">{action}</span></div>
  <div class="mono muted">{resource}</div>
  <div class="mono muted fs-11">{rid}</div>
</div>"##,
                created = html_escape(created.get(..19).unwrap_or(created)),
                actor = html_escape(actor),
                action = html_escape(action),
                resource = html_escape(resource),
                rid = html_escape(resource_id.get(..8).unwrap_or(resource_id)),
            )
        })
        .collect();

    let empty = if log.is_empty() {
        r##"<div class="muted p-20 ta-center">No audit entries yet.</div>"##
    } else {
        ""
    };

    let content = format!(
        r##"<div class="page-pad">
  <div class="eyebrow">Audit Log</div>
  <h2 class="display-sm" style="margin:4px 0 16px">Management actions</h2>
  <div class="card" style="padding:0;overflow:hidden">
    <div class="rt-head" style="grid-template-columns:0.8fr 1fr 0.6fr 0.6fr 0.5fr">
      <div>Time</div><div>Actor</div><div>Action</div><div>Resource</div><div>ID</div>
    </div>
    {rows}{empty}
  </div>
</div>"##,
        rows = rows,
        empty = empty,
    );

    manage_shell(
        "Audit Log - Concierge",
        &content,
        "Audit Log",
        base_url,
        locale,
    )
}

fn scheduled_grants_table(scheduled: &[crate::types::ScheduledGrant], base_url: &str) -> String {
    if scheduled.is_empty() {
        return r#"<p class="muted fs-13 m-0">No scheduled grants yet.</p>"#.to_string();
    }

    let rows: String = scheduled
        .iter()
        .map(|g| {
            let expiry = if g.expires_in_days <= 0 {
                "never".to_string()
            } else {
                format!("{}d", g.expires_in_days)
            };
            let last_run = g.last_run_at.as_deref().unwrap_or("—");
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
      <button class="btn ghost sm" type="submit">Remove</button>
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
    base_url: &str,
    locale: &Locale,
    cfg: &crate::storage::Pricing,
    scheduled: &[crate::types::ScheduledGrant],
    schedule_form_msg: Option<&str>,
) -> String {
    let pricing_table = pricing_form_table(cfg, base_url);
    let scheduled_rows = scheduled_grants_table(scheduled, base_url);
    let schedule_msg = schedule_form_msg.unwrap_or("");

    let content = format!(
        r##"<div class="page-pad">
  <div class="eyebrow">Billing</div>
  <h2 class="display-sm m-0 mt-4 mb-16">Pricing &amp; grants</h2>

  <div class="card p-22 mb-16">
    <h3 class="mb-8">Pricing</h3>
    <p class="muted mb-12">One column per supported currency. Each cell is the price in that currency's own unit (no conversion). The unit caption under each row shows whether the value is in minor units or milli-minor units.</p>
    <div id="pricing-toast"></div>
    <form hx-post="{base_url}/manage/billing/settings" hx-target="{hash}pricing-toast" hx-swap="innerHTML" hx-ext="json-enc">
      {pricing_table}

      <div class="row gap-12 wrap mb-12 mt-16">
        <label class="flex-1" style="min-width:240px">
          <div class="eyebrow mb-4">Addresses per reply-email pack</div>
          <input class="input mono" name="email_pack_size" type="number" min="1" required value="{email_pack_size}">
          <div class="muted fs-11 mt-4">currency-independent · tenants receive this many addresses per active pack</div>
        </label>
      </div>

      <button class="btn sm mt-12" type="submit">Save settings</button>
    </form>
  </div>

  <div class="card p-22 mb-16">
    <h3 class="mb-8">Recurring credit grants</h3>
    <p class="muted mb-12">Automated grants that fire on a calendar cadence and apply to every tenant. Use these in place of a baked-in monthly free allowance.</p>
    {scheduled_rows}
    <div id="schedule-toast">{schedule_msg}</div>
    <form hx-post="{base_url}/manage/billing/schedule" hx-target="body" hx-swap="innerHTML" hx-ext="json-enc" class="mt-12">
      <div class="row gap-12 wrap mb-12">
        <label style="min-width:220px">
          <div class="eyebrow mb-4">Cadence</div>
          <select class="input" name="cadence" required>
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
        <label style="min-width:140px">
          <div class="eyebrow mb-4">Credits</div>
          <input class="input mono" name="credits" type="number" min="1" required>
        </label>
        <label style="min-width:140px">
          <div class="eyebrow mb-4">Expires in (days, 0 = never)</div>
          <input class="input mono" name="expires_in_days" type="number" min="0" value="0" required>
        </label>
      </div>
      <button class="btn sm" type="submit">Add scheduled grant</button>
    </form>
  </div>
</div>"##,
        base_url = base_url,
        hash = HASH,
        pricing_table = pricing_table,
        email_pack_size = cfg.email_pack_size,
        scheduled_rows = scheduled_rows,
        schedule_msg = schedule_msg,
    );

    manage_shell("Billing - Concierge", &content, "Billing", base_url, locale)
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
                r##" <button type="button" class="btn ghost sm btn-danger" hx-delete="{base_url}/manage/billing/currency/{code}" hx-confirm="Remove all {code} prices?" hx-target="body" hx-swap="innerHTML" title="Remove {code}">×</button>"##,
                base_url = base_url,
                code = html_escape(c),
            );
            format!(
                r##"<th class="ta-right">
  <div class="row gap-4" style="justify-content:flex-end;align-items:center">
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
                        r##"<td><input class="input mono" name="{name}" type="number" min="1" required value="{value}" style="max-width:160px"></td>"##,
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
                r#"<option value="{code}">{code} — {name} ({symbol})</option>"#,
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
    <button class="btn sm" type="submit">Add</button>
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
// Persona catalog
// =====================================================================

pub fn personas_list_html(
    rows: &[crate::types::PersonaCatalogRow],
    base_url: &str,
    locale: &Locale,
) -> String {
    let row_html: String = rows
        .iter()
        .map(|r| {
            let status_chip = persona_status_chip(&r.safety.status);
            let archetype_label = match &r.source {
                crate::types::PersonaSource::Builder(b) => b.archetype.label(),
                crate::types::PersonaSource::Custom(_) => "Custom",
            };
            let system_badge = if r.is_system {
                r#"<span class="chip" style="margin-left:6px">system</span>"#
            } else {
                ""
            };
            format!(
                r##"<tr>
  <td><a href="{base_url}/manage/personas/{slug}">{slug}</a>{system_badge}</td>
  <td>{label}</td>
  <td>{archetype}</td>
  <td>{status}</td>
</tr>"##,
                base_url = base_url,
                slug = html_escape(&r.slug),
                system_badge = system_badge,
                label = html_escape(&r.label),
                archetype = html_escape(archetype_label),
                status = status_chip,
            )
        })
        .collect();

    let content = format!(
        r##"<div class="page-pad">
  <div class="row between mb-16">
    <h1 class="display-sm m-0">Persona catalog</h1>
    <a class="btn primary" href="{base_url}/manage/personas/new">+ New persona</a>
  </div>
  <p class="muted mb-16">Catalog rows surface in the public demo and to tenants picking a starter persona at onboarding. Every save runs through the safety classifier; only Approved rows are visible outside this page.</p>
  <table class="rt">
    <thead><tr>
      <th>Slug</th><th>Label</th><th>Archetype</th><th>Safety</th>
    </tr></thead>
    <tbody>{rows}</tbody>
  </table>
</div>"##,
        base_url = base_url,
        rows = row_html,
    );
    manage_shell(
        "Personas — Concierge",
        &content,
        "Personas",
        base_url,
        locale,
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

pub fn persona_edit_html(
    row: Option<&crate::types::PersonaCatalogRow>,
    base_url: &str,
    locale: &Locale,
) -> String {
    let is_new = row.is_none();
    let blank_builder = crate::types::PersonaBuilder::default();
    let blank_row;
    let row_ref = match row {
        Some(r) => r,
        None => {
            blank_row = crate::types::PersonaCatalogRow {
                slug: String::new(),
                label: String::new(),
                description: String::new(),
                source: crate::types::PersonaSource::Builder(blank_builder.clone()),
                greeting: String::new(),
                is_system: false,
                safety: crate::types::PersonaSafety::default(),
                created_at: None,
                updated_at: None,
            };
            &blank_row
        }
    };

    let (mode, builder, custom_text) = match &row_ref.source {
        crate::types::PersonaSource::Builder(b) => ("builder", b.clone(), String::new()),
        crate::types::PersonaSource::Custom(s) => {
            ("custom", crate::types::PersonaBuilder::default(), s.clone())
        }
    };

    let archetype_options: String = PersonaPreset::ALL
        .iter()
        .map(|a| {
            format!(
                r#"<label class="row gap-6"><input type="radio" name="archetype" value="{slug}" x-model="builder.archetype"> {label}</label>"#,
                slug = html_escape(a.slug()),
                label = html_escape(a.label()),
            )
        })
        .collect();

    let action = if is_new {
        format!("{base_url}/manage/personas/new")
    } else {
        format!("{base_url}/manage/personas/{}", row_ref.slug)
    };

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

    let delete_button = if is_new || row_ref.is_system {
        String::new()
    } else {
        format!(
            r##"<form hx-post="{action}/delete" hx-target="body" hx-swap="innerHTML" style="display:inline">
              <button type="submit" class="btn ghost sm" style="color:var(--warn)" onclick="return confirm('Delete this persona? This cannot be undone.')">Delete</button>
            </form>"##,
            action = action,
        )
    };

    let slug_field = if is_new {
        format!(
            r#"<div class="mt-12">
              <label for="persona-slug" class="eyebrow lbl">Slug (lowercase, _ or -)</label>
              <input id="persona-slug" class="input" name="slug" required pattern="[a-z0-9_-]+">
            </div>"#
        )
    } else {
        format!(
            r#"<input type="hidden" name="slug" value="{}">"#,
            html_escape(&row_ref.slug)
        )
    };

    fn esc_js(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    }

    let x_data = format!(
        "{{ mode: '{mode}', customText: '{custom}', builder: {{ archetype: '{archetype}', biz_name: '{biz_name}', biz_type: '{biz_type}', city: '{city}', hours: '{hours}', goal: '{goal}', goal_url: '{goal_url}', catch_phrases: '{cp}', off_topics: '{ot}', never: '{never}', handoff_conditions: '{handoff}' }} }}",
        mode = esc_js(mode),
        custom = esc_js(&custom_text),
        archetype = esc_js(builder.archetype.slug()),
        biz_name = esc_js(&builder.biz_name),
        biz_type = esc_js(&builder.biz_type),
        city = esc_js(&builder.city),
        hours = esc_js(&builder.hours),
        goal = esc_js(&builder.goal),
        goal_url = esc_js(&builder.goal_url),
        cp = esc_js(&builder.catch_phrases.join("\n")),
        ot = esc_js(&builder.off_topics.join("\n")),
        never = esc_js(&builder.never),
        handoff = esc_js(&builder.handoff_conditions.join("\n")),
    );

    let h1 = if is_new {
        "New persona".to_string()
    } else {
        format!("Edit persona — {}", html_escape(&row_ref.label))
    };

    let content = format!(
        r##"<div class="page-pad" x-data="{x_data}" hx-ext="json-enc">
  <p><a href="{base_url}/manage/personas" class="btn ghost sm">← Back</a></p>
  <h1 class="display-sm m-0 mb-4">{h1}</h1>
  <p class="muted fs-13 mb-16">Catalog row. Saves reset the safety verdict to Draft and re-run the classifier. Tenants and the demo only see Approved rows.</p>

  <div class="card p-14 mb-16 row gap-10" style="align-items:center">
    {safety_chip}
    <span class="muted fs-13">{safety_detail}</span>
  </div>

  <form hx-post="{action}" hx-target="body" hx-swap="innerHTML">
    <input type="hidden" name="mode" :value="mode">

    <div class="card p-22 mb-16">
      {slug_field}
      <div class="mt-12">
        <label for="persona-label" class="eyebrow lbl">Label</label>
        <input id="persona-label" class="input" name="label" value="{label}" required>
      </div>
      <div class="mt-12">
        <label for="persona-description" class="eyebrow lbl">Description</label>
        <input id="persona-description" class="input" name="description" value="{description}" required>
      </div>
      <div class="mt-12">
        <label for="persona-greeting" class="eyebrow lbl">Greeting (first assistant turn in the demo)</label>
        <input id="persona-greeting" class="input" name="greeting" value="{greeting}" required>
      </div>

      <div class="mt-16 eyebrow lbl">Source mode</div>
      <div class="row gap-8 mb-12" style="flex-wrap:wrap">
        <label class="row gap-6"><input type="radio" name="mode" value="builder" x-model="mode"> Builder</label>
        <label class="row gap-6"><input type="radio" name="mode" value="custom" x-model="mode"> Custom (raw middle)</label>
      </div>

      <!-- BUILDER -->
      <div x-show="mode === 'builder'" x-cloak :aria-hidden="mode !== 'builder'">
        <div class="eyebrow lbl mb-6">Archetype (voice)</div>
        <div class="row gap-12 mb-12" style="flex-wrap:wrap">{archetype_options}</div>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px">
          <div>
            <label for="persona-biz-name" class="eyebrow lbl">Business name</label>
            <input id="persona-biz-name" class="input" name="biz_name" x-model="builder.biz_name">
          </div>
          <div>
            <label for="persona-biz-type" class="eyebrow lbl">Business type (e.g. florist)</label>
            <input id="persona-biz-type" class="input" name="biz_type" x-model="builder.biz_type">
          </div>
          <div>
            <label for="persona-city" class="eyebrow lbl">City (optional)</label>
            <input id="persona-city" class="input" name="city" x-model="builder.city">
          </div>
          <div>
            <label for="persona-hours" class="eyebrow lbl">Hours (optional)</label>
            <input id="persona-hours" class="input" name="hours" x-model="builder.hours" placeholder="Tue–Sun 9am–7pm">
          </div>
          <div>
            <label for="persona-never" class="eyebrow lbl">Never (one short rule)</label>
            <input id="persona-never" class="input" name="never" x-model="builder.never">
          </div>
          <div>
            <label for="persona-goal" class="eyebrow lbl">Goal (drives the conversation)</label>
            <input id="persona-goal" class="input" name="goal" x-model="builder.goal" maxlength="120" placeholder="book a delivery slot">
          </div>
          <div>
            <label for="persona-goal-url" class="eyebrow lbl">Goal URL (optional)</label>
            <input id="persona-goal-url" class="input" name="goal_url" x-model="builder.goal_url" maxlength="200" placeholder="/book">
          </div>
        </div>
        <div class="mt-12">
          <label for="persona-catch" class="eyebrow lbl">Catch phrases (one per line)</label>
          <textarea id="persona-catch" class="textarea" name="catch_phrases" x-model="builder.catch_phrases" rows="3"></textarea>
        </div>
        <div class="mt-12">
          <label for="persona-off" class="eyebrow lbl">Off-topic subjects (one per line)</label>
          <textarea id="persona-off" class="textarea" name="off_topics" x-model="builder.off_topics" rows="3"></textarea>
        </div>
        <div class="mt-12">
          <label for="persona-handoff" class="eyebrow lbl">Hand off to a human if (one per line, max 5)</label>
          <textarea id="persona-handoff" class="textarea" name="handoff_conditions" x-model="builder.handoff_conditions" rows="3" placeholder="refund or complaint&#10;the customer is upset"></textarea>
        </div>
      </div>

      <!-- CUSTOM -->
      <div x-show="mode === 'custom'" x-cloak :aria-hidden="mode !== 'custom'">
        <label for="persona-custom" class="eyebrow lbl">Custom prompt middle (still envelope-wrapped at AI-call time)</label>
        <textarea id="persona-custom" class="textarea mono" name="custom_text" x-model="customText" rows="14" maxlength="2000"></textarea>
        <p class="muted fs-12 mt-4"><span x-text="customText.length"></span> / 2000</p>
      </div>
    </div>

    <div class="row gap-8" style="justify-content:space-between">
      <div>{delete_button}</div>
      <button type="submit" class="btn primary">Save</button>
    </div>
  </form>
</div>"##,
        x_data = x_data,
        base_url = base_url,
        h1 = h1,
        safety_chip = safety_chip,
        safety_detail = safety_detail,
        action = action,
        slug_field = slug_field,
        label = html_escape(&row_ref.label),
        description = html_escape(&row_ref.description),
        greeting = html_escape(&row_ref.greeting),
        archetype_options = archetype_options,
        delete_button = delete_button,
    );
    manage_shell(
        "Persona — Concierge",
        &content,
        "Personas",
        base_url,
        locale,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_overview_renders_per_currency_inputs() {
        use crate::storage::PricingConcept::*;
        let l = Locale::default_inr();
        let mut cfg = crate::storage::Pricing {
            email_pack_size: 7,
            amounts: std::collections::BTreeMap::new(),
        };
        cfg.amounts.insert((UnitPriceMilli, "INR".into()), 12_345);
        cfg.amounts.insert((UnitPriceMilli, "USD".into()), 234);
        cfg.amounts.insert((AddressPrice, "INR".into()), 5_555);
        cfg.amounts.insert((AddressPrice, "USD".into()), 77);
        cfg.amounts.insert((VerificationAmount, "INR".into()), 199);
        cfg.amounts.insert((VerificationAmount, "USD".into()), 33);

        let html = billing_overview_html("https://example.test", &l, &cfg, &[], None);

        // Form posts to the management settings endpoint.
        assert!(
            html.contains(r#"hx-post="https://example.test/manage/billing/settings""#),
            "settings form missing"
        );

        // Per-(concept, currency) cell names.
        assert!(html.contains(r#"name="unit_price_milli__INR""#));
        assert!(html.contains(r#"value="12345""#));
        assert!(html.contains(r#"name="unit_price_milli__USD""#));
        assert!(html.contains(r#"value="234""#));
        assert!(html.contains(r#"name="address_price__INR""#));
        assert!(html.contains(r#"value="5555""#));
        assert!(html.contains(r#"name="address_price__USD""#));
        assert!(html.contains(r#"value="77""#));
        assert!(html.contains(r#"name="verification_amount__INR""#));
        assert!(html.contains(r#"value="199""#));
        assert!(html.contains(r#"name="verification_amount__USD""#));
        assert!(html.contains(r#"value="33""#));
        assert!(html.contains(r#"name="email_pack_size""#));
        assert!(html.contains(r#"value="7""#));

        // Currency column headers carry the rusty_money symbol + name.
        assert!(html.contains("INR"));
        assert!(html.contains("USD"));
        assert!(html.contains("Indian Rupee"));
        assert!(html.contains("United States Dollar"));

        // No conversion text — currencies render side-by-side, not derived.
        assert!(!html.contains("paise_per_usd"));
        assert!(!html.contains("USD exchange rate"));

        // Add-currency picker offers an unused currency (EUR).
        assert!(html.contains(r#"name="__currencies""#));
        assert!(html.contains(r#"value="EUR""#));

        // Recurring-grants section still rendered.
        assert!(html.contains("Recurring credit grants"));
        assert!(html.contains("No scheduled grants yet."));
    }
}
