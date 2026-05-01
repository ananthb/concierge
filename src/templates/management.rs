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
    use crate::types::GrantAudience;

    if scheduled.is_empty() {
        return r#"<p class="muted fs-13 m-0">No scheduled grants yet.</p>"#.to_string();
    }

    let rows: String = scheduled
        .iter()
        .map(|g| {
            let audience = match &g.audience {
                GrantAudience::Everyone => "Every tenant".to_string(),
                GrantAudience::Emails(es) => format!("{} tenant(s)", es.len()),
            };
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
  <td class="mono fs-12">{audience}</td>
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
                audience = html_escape(&audience),
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
                  <th>Audience</th>
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
    milli_paise: i64,
    milli_cents: i64,
    address_paise: i64,
    address_cents: i64,
    email_pack_size: i64,
    free_monthly_credits: i64,
    scheduled: &[crate::types::ScheduledGrant],
    schedule_form_msg: Option<&str>,
) -> String {
    // Display in major units: milli-paise / 100_000 = rupees, milli-cents / 100_000 = dollars.
    let paise_per_reply = format!("{:.2}", milli_paise as f64 / 100_000.0);
    let cents_per_reply = format!("{:.3}", milli_cents as f64 / 100_000.0);
    let address_inr = format!("{:.2}", address_paise as f64 / 100.0);
    let address_usd = format!("{:.2}", address_cents as f64 / 100.0);

    let scheduled_rows = scheduled_grants_table(scheduled, base_url);
    let schedule_msg = schedule_form_msg.unwrap_or("");

    let content = format!(
        r##"<div class="page-pad">
  <div class="eyebrow">Billing</div>
  <h2 class="display-sm m-0 mt-4 mb-16">Pricing &amp; grants</h2>

  <div class="card p-22 mb-16">
    <h3 class="mb-8">AI-reply pricing</h3>
    <p class="muted mb-12">Per-reply rate that applies to every tenant. Stored in milli-units (1/1000 of a paisa or cent) so the slider can offer fine-grained credit amounts.</p>
    <div id="pricing-toast"></div>
    <form hx-post="{base_url}/manage/billing/settings" hx-target="{hash}pricing-toast" hx-swap="innerHTML" hx-ext="json-enc">
      <div class="row gap-12 wrap mb-16">
        <label class="flex-1" style="min-width:220px">
          <div class="eyebrow mb-4">Reply price (₹ / reply)</div>
          <input class="input mono" name="unit_price_millipaise" type="number" min="1" required value="{milli_paise}">
          <div class="muted fs-11 mt-4">milli-paise · currently ₹{paise_per_reply}</div>
        </label>
        <label class="flex-1" style="min-width:220px">
          <div class="eyebrow mb-4">Reply price ($ / reply)</div>
          <input class="input mono" name="unit_price_millicents" type="number" min="1" required value="{milli_cents}">
          <div class="muted fs-11 mt-4">milli-cents · currently ${cents_per_reply}</div>
        </label>
      </div>

      <h3 class="mb-8">Reply-email subscription</h3>
      <p class="muted mb-12">Each pack of N addresses costs the rate below per recurring period (monthly). Tenants pay this in their selected currency.</p>
      <div class="row gap-12 wrap mb-12">
        <label class="flex-1" style="min-width:220px">
          <div class="eyebrow mb-4">Pack price (₹ / month)</div>
          <input class="input mono" name="address_price_paise" type="number" min="1" required value="{address_paise}">
          <div class="muted fs-11 mt-4">paise · currently ₹{address_inr}</div>
        </label>
        <label class="flex-1" style="min-width:220px">
          <div class="eyebrow mb-4">Pack price ($ / month)</div>
          <input class="input mono" name="address_price_cents" type="number" min="1" required value="{address_cents}">
          <div class="muted fs-11 mt-4">cents · currently ${address_usd}</div>
        </label>
        <label class="flex-1" style="min-width:160px">
          <div class="eyebrow mb-4">Addresses per pack</div>
          <input class="input mono" name="email_pack_size" type="number" min="1" required value="{email_pack_size}">
          <div class="muted fs-11 mt-4">tenants get this many addresses per active pack</div>
        </label>
      </div>
      <h3 class="mb-8">Free monthly credits</h3>
      <p class="muted mb-12">How many AI replies every tenant gets at the start of each calendar month. Existing balances aren't touched mid-month.</p>
      <div class="row gap-12 wrap mb-12">
        <label class="flex-1" style="min-width:220px">
          <div class="eyebrow mb-4">Monthly credits per tenant</div>
          <input class="input mono" name="free_monthly_credits" type="number" min="0" required value="{free_monthly_credits}">
          <div class="muted fs-11 mt-4">currently {free_monthly_credits} replies/month</div>
        </label>
      </div>
      <button class="btn sm" type="submit">Save settings</button>
    </form>
  </div>

  <div class="card p-22 mb-16">
    <h3 class="mb-8">Scheduled credit grants</h3>
    <p class="muted mb-12">Automated grants that fire on a calendar cadence. Use these to drop credits into specific tenants (by email) or every tenant on a recurring schedule.</p>
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
      <div class="mb-12">
        <div class="eyebrow mb-4">Audience</div>
        <label class="row gap-8" style="align-items:center;cursor:pointer">
          <input type="radio" name="audience_kind" value="everyone" checked>
          <span>Every tenant</span>
        </label>
        <label class="row gap-8 mt-4" style="align-items:center;cursor:pointer">
          <input type="radio" name="audience_kind" value="emails">
          <span>Specific tenants by email</span>
        </label>
        <textarea class="input mono mt-8" name="emails" rows="3" placeholder="comma- or newline-separated tenant emails"></textarea>
      </div>
      <button class="btn sm" type="submit">Add scheduled grant</button>
    </form>
  </div>

  <div class="card p-18">
    <h3 class="mb-8">Grant free replies</h3>
    <p class="muted mb-12">Give a tenant reply credits directly.</p>
    <div id="grant-toast"></div>
    <form hx-post="" hx-target="{hash}grant-toast" hx-swap="innerHTML"
          onsubmit="this.setAttribute('hx-post', '{base_url}/manage/billing/grant/' + this.querySelector('[name=tenant_id]').value); htmx.process(this); return false;">
      <div class="row gap-12 wrap">
        <input class="input" name="tenant_id" placeholder="Tenant ID" required style="max-width:300px">
        <input class="input" name="replies" placeholder="Replies" type="number" min="1" required style="max-width:140px">
        <input class="input" name="expires_days" placeholder="Expires in (days)" type="number" min="1" value="365" style="max-width:160px">
        <button class="btn sm" type="submit">Grant</button>
      </div>
    </form>
  </div>
</div>"##,
        base_url = base_url,
        hash = HASH,
        milli_paise = milli_paise,
        milli_cents = milli_cents,
        address_paise = address_paise,
        address_cents = address_cents,
        email_pack_size = email_pack_size,
        free_monthly_credits = free_monthly_credits,
        paise_per_reply = paise_per_reply,
        cents_per_reply = cents_per_reply,
        address_inr = address_inr,
        address_usd = address_usd,
        scheduled_rows = scheduled_rows,
        schedule_msg = schedule_msg,
    );

    manage_shell("Billing - Concierge", &content, "Billing", base_url, locale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_overview_renders_inputs_with_db_values() {
        let l = Locale::default_inr();
        // Use distinctive non-default numbers so we can assert they appear.
        let html = billing_overview_html(
            "https://example.test",
            &l,
            12_345, // milli-paise
            234,    // milli-cents
            5_555,  // address paise
            77,     // address cents
            7,      // email pack size
            150,    // free monthly credits
            &[],
            None,
        );

        // Form posts to the management settings endpoint.
        assert!(
            html.contains(r#"hx-post="https://example.test/manage/billing/settings""#),
            "settings form missing: {html}"
        );

        // Each input renders the DB-loaded value.
        assert!(html.contains(r#"name="unit_price_millipaise""#));
        assert!(html.contains(r#"value="12345""#));
        assert!(html.contains(r#"name="unit_price_millicents""#));
        assert!(html.contains(r#"value="234""#));
        assert!(html.contains(r#"name="address_price_paise""#));
        assert!(html.contains(r#"value="5555""#));
        assert!(html.contains(r#"name="address_price_cents""#));
        assert!(html.contains(r#"value="77""#));
        assert!(html.contains(r#"name="email_pack_size""#));
        assert!(html.contains(r#"value="7""#));
        assert!(html.contains(r#"name="free_monthly_credits""#));
        assert!(html.contains(r#"value="150""#));
        // Scheduled-grants section is always rendered.
        assert!(html.contains("Scheduled credit grants"));
        assert!(html.contains("No scheduled grants yet."));

        // Per-reply hint shows the major-currency conversion.
        // 12_345 milli-paise / 100_000 = 0.12345 ≈ 0.12
        assert!(
            html.contains("₹0.12"),
            "missing INR per-reply preview: {html}"
        );
        // 234 milli-cents / 100_000 = 0.00234 ≈ 0.002
        assert!(
            html.contains("$0.002"),
            "missing USD per-reply preview: {html}"
        );
        // Address prices (paise/100, cents/100) render as major units.
        assert!(html.contains("₹55.55"), "missing addr inr: {html}");
        assert!(html.contains("$0.77"), "missing addr usd: {html}");
    }
}
