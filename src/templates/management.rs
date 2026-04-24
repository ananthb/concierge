//! Management panel templates — super-admin UI

use crate::helpers::html_escape;
use crate::types::*;

use super::base::{base_html, brand_mark, LOGO_INLINE};
use super::HASH;

fn manage_shell(title: &str, content: &str, active: &str, base_url: &str) -> String {
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

    base_html(title, &inner)
}

pub fn dashboard_html(email: &str, tenant_count: usize, base_url: &str) -> String {
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
  <div class="card p-18">
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
        base_url = base_url,
    );

    manage_shell("Management - Concierge", &content, "Dashboard", base_url)
}

pub fn tenants_list_html(tenants: &[Tenant], base_url: &str) -> String {
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
                plan = html_escape(&t.plan),
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

    manage_shell("Tenants - Concierge", &content, "Tenants", base_url)
}

pub fn tenant_detail_html(
    tenant: &Tenant,
    wa: &[WhatsAppAccount],
    ig: &[InstagramAccount],
    domains: &[EmailSubdomain],
    base_url: &str,
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

    let domain_list: String = domains
        .iter()
        .map(|d| {
            format!(
                r##"<div class="side-row"><div class="flex-1 fs-13">{domain}</div></div>"##,
                domain = html_escape(&d.domain),
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
  <div id="toast"></div>
  <div class="card p-18 mb-16">
    <h3 class="mb-12">Plan</h3>
    <form hx-put="{base_url}/manage/tenants/{id}" hx-target="{hash}toast" hx-swap="innerHTML">
      <div class="row gap-12">
        <select class="select" name="plan" style="max-width:200px">
          <option value="free"{free_sel}>Free</option>
          <option value="starter"{starter_sel}>Starter</option>
          <option value="pro"{pro_sel}>Pro</option>
          <option value="business"{business_sel}>Business</option>
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
        plan = html_escape(&tenant.plan),
        created = html_escape(&tenant.created_at.get(..10).unwrap_or(&tenant.created_at)),
        free_sel = if tenant.plan == "free" {
            " selected"
        } else {
            ""
        },
        starter_sel = if tenant.plan == "starter" {
            " selected"
        } else {
            ""
        },
        pro_sel = if tenant.plan == "pro" {
            " selected"
        } else {
            ""
        },
        business_sel = if tenant.plan == "business" {
            " selected"
        } else {
            ""
        },
        wa_count = wa.len(),
        ig_count = ig.len(),
        domain_count = domains.len(),
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
    )
}

pub fn audit_html(log: &[serde_json::Value], base_url: &str) -> String {
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

    manage_shell("Audit Log - Concierge", &content, "Audit Log", base_url)
}

pub fn billing_overview_html(base_url: &str) -> String {
    let content = format!(
        r##"<div class="page-pad">
  <div class="eyebrow">Billing</div>
  <h2 class="display-sm m-0 mt-4 mb-16">Grant credits</h2>

  <div class="card p-22 mb-16">
    <p class="muted m-0">Pricing is a flat <strong>₹2 / $0.02 per reply</strong>. Tenants top up via slider on /admin/billing — no packs to manage.</p>
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
    );

    manage_shell("Billing - Concierge", &content, "Billing", base_url)
}
