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
        r##"<div style="padding:24px 28px">
  <div class="between" style="margin-bottom:24px">
    <div>
      <div class="eyebrow">Management Panel</div>
      <h2 class="display-sm" style="margin:4px 0 0">Welcome, {email}</h2>
    </div>
  </div>
  <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:16px;margin-bottom:24px">
    <div class="card" style="padding:18px;text-align:center">
      <div class="stat-n serif">{tenant_count}</div>
      <div class="mono muted" style="font-size:11px">Tenants</div>
    </div>
    <div class="card" style="padding:18px;text-align:center">
      <div class="stat-n serif">—</div>
      <div class="mono muted" style="font-size:11px">MRR</div>
    </div>
    <div class="card" style="padding:18px;text-align:center">
      <div class="stat-n serif">—</div>
      <div class="mono muted" style="font-size:11px">Active</div>
    </div>
  </div>
  <div class="card" style="padding:18px">
    <div class="between">
      <div class="eyebrow">Quick actions</div>
    </div>
    <div class="row gap-12" style="margin-top:12px">
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
  <div class="mono muted" style="font-size:11px">{created}</div>
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
        r##"<div style="padding:20px;text-align:center" class="muted">No tenants yet.</div>"##
    } else {
        ""
    };

    let content = format!(
        r##"<div style="padding:24px 28px">
  <div class="between" style="margin-bottom:16px">
    <div>
      <div class="eyebrow">All tenants</div>
      <h2 class="display-sm" style="margin:4px 0 0">{count} tenant{s}</h2>
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
                r##"<div class="side-row"><div style="flex:1;font-size:13px">{name} <span class="mono muted">{phone}</span></div></div>"##,
                name = html_escape(&a.name),
                phone = html_escape(&a.phone_number),
            )
        })
        .collect();

    let ig_list: String = ig
        .iter()
        .map(|a| {
            format!(
                r##"<div class="side-row"><div style="flex:1;font-size:13px">@{username}</div></div>"##,
                username = html_escape(&a.instagram_username),
            )
        })
        .collect();

    let domain_list: String = domains
        .iter()
        .map(|d| {
            format!(
                r##"<div class="side-row"><div style="flex:1;font-size:13px">{domain}</div></div>"##,
                domain = html_escape(&d.domain),
            )
        })
        .collect();

    let content = format!(
        r##"<div style="padding:24px 28px">
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
  <div class="card" style="padding:18px;margin-bottom:16px">
    <h3 style="margin-bottom:12px">Plan</h3>
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
    <div class="card" style="padding:16px">
      <div class="eyebrow">WhatsApp ({wa_count})</div>
      <div class="side-list">{wa_list}</div>
    </div>
    <div class="card" style="padding:16px">
      <div class="eyebrow">Instagram ({ig_count})</div>
      <div class="side-list">{ig_list}</div>
    </div>
    <div class="card" style="padding:16px">
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
            r#"<div class="muted" style="font-size:13px">None</div>"#.to_string()
        } else {
            wa_list
        },
        ig_list = if ig_list.is_empty() {
            r#"<div class="muted" style="font-size:13px">None</div>"#.to_string()
        } else {
            ig_list
        },
        domain_list = if domain_list.is_empty() {
            r#"<div class="muted" style="font-size:13px">None</div>"#.to_string()
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
  <div class="mono muted" style="font-size:11px">{created}</div>
  <div>{actor}</div>
  <div><span class="chip">{action}</span></div>
  <div class="mono muted">{resource}</div>
  <div class="mono muted" style="font-size:11px">{rid}</div>
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
        r##"<div style="padding:20px;text-align:center" class="muted">No audit entries yet.</div>"##
    } else {
        ""
    };

    let content = format!(
        r##"<div style="padding:24px 28px">
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

pub fn billing_overview_html(packs: &[CreditPackRow], base_url: &str) -> String {
    let pack_rows: String = packs
        .iter()
        .map(|p| {
            let status = if p.active == 1 { "Active" } else { "Inactive" };
            let status_class = if p.active == 1 { "ok" } else { "warn" };
            format!(
                r##"<div class="rt-row" style="grid-template-columns:0.5fr 1fr 0.6fr 0.6fr 0.6fr 0.4fr 80px">
  <div class="mono muted">{id}</div>
  <div><strong>{name}</strong></div>
  <div class="mono">{replies}</div>
  <div class="mono">₹{inr}</div>
  <div class="mono">${usd}</div>
  <div><span class="chip {status_class}">{status}</span></div>
  <div>
    <button class="btn ghost sm btn-danger" hx-delete="{base_url}/manage/billing/packs/{id}" hx-confirm="Delete pack {name}?" hx-target="closest .rt-row" hx-swap="outerHTML">Delete</button>
  </div>
</div>"##,
                id = p.id,
                name = html_escape(&p.name),
                replies = p.replies,
                inr = p.price_inr / 100,
                usd = p.price_usd / 100,
                status = status,
                status_class = status_class,
                base_url = base_url,
            )
        })
        .collect();

    let content = format!(
        r##"<div style="padding:24px 28px">
  <div class="eyebrow">Billing</div>
  <h2 class="display-sm" style="margin:4px 0 16px">Credit packs &amp; grants</h2>

  <div class="card" style="padding:0;overflow:hidden;margin-bottom:16px">
    <div class="rt-head" style="grid-template-columns:0.5fr 1fr 0.6fr 0.6fr 0.6fr 0.4fr 80px">
      <div>ID</div><div>Name</div><div>Replies</div><div>INR</div><div>USD</div><div>Status</div><div></div>
    </div>
    {pack_rows}
    <div style="padding:14px 20px;background:var(--cream-2)">
      <div class="eyebrow" style="margin-bottom:8px">Add pack</div>
      <form hx-post="{base_url}/manage/billing/packs" style="display:flex;gap:8px;flex-wrap:wrap;align-items:end">
        <input class="input" name="name" placeholder="Name" required style="max-width:150px">
        <input class="input" name="replies" placeholder="Replies" type="number" min="1" required style="max-width:100px">
        <input class="input" name="price_inr" placeholder="INR (paise)" type="number" required style="max-width:120px">
        <input class="input" name="price_usd" placeholder="USD (cents)" type="number" required style="max-width:120px">
        <input class="input" name="sort_order" placeholder="Order" type="number" value="5" style="max-width:80px">
        <button class="btn sm" type="submit">Add</button>
      </form>
    </div>
  </div>

  <div class="card" style="padding:18px">
    <h3 style="margin-bottom:8px">Grant free replies</h3>
    <p class="muted" style="margin-bottom:12px">Give a tenant reply credits directly.</p>
    <div id="grant-toast"></div>
    <form hx-post="" hx-target="{hash}grant-toast" hx-swap="innerHTML"
          onsubmit="this.setAttribute('hx-post', '{base_url}/manage/billing/grant/' + this.querySelector('[name=tenant_id]').value); htmx.process(this); return false;">
      <div class="row gap-12" style="flex-wrap:wrap">
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
        pack_rows = pack_rows,
    );

    manage_shell("Billing - Concierge", &content, "Billing", base_url)
}
