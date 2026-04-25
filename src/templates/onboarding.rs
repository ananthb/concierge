//! Onboarding wizard templates

use crate::helpers::html_escape;
use crate::types::*;

use super::base::{base_html, base_html_with_meta, brand_mark, PageMeta};
use super::HASH;

/// Escape a string for safe embedding inside a single-quoted JS string in an
/// HTML attribute. Handles backslashes, single quotes, newlines, and the
/// HTML-meaningful `<`/`>`/`&`/`"` characters.
fn js_attr_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

const STEPS: &[(&str, &str)] = &[
    ("basics", "The basics"),
    ("channels", "Plug in"),
    ("notifications", "Heads up"),
    ("replies", "Replies"),
    ("launch", "Ship it"),
];

fn rail_html(current: &str, progress_expr: &str) -> String {
    let idx = STEPS.iter().position(|(id, _)| *id == current).unwrap_or(0);

    let segs: String = STEPS
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if i < idx {
                r#"<div class="seg done"><span class="fill"></span></div>"#.to_string()
            } else if i == idx {
                // Active segment: width reacts to the Alpine progress expression.
                // Floor at 8% so it doesn't look empty on first paint.
                format!(
                    r#"<div class="seg active"><span class="fill" :style="`width: ${{Math.max(8, Math.min(100, ({progress_expr}) * 100))}}%`"></span></div>"#,
                    progress_expr = progress_expr,
                )
            } else {
                r#"<div class="seg"><span class="fill"></span></div>"#.to_string()
            }
        })
        .collect();

    let labels: String = STEPS
        .iter()
        .enumerate()
        .map(|(i, (_, label))| {
            let class = if i < idx {
                "done"
            } else if i == idx {
                "active"
            } else {
                ""
            };
            format!(r#"<span class="{class}">{label}</span>"#)
        })
        .collect();

    format!(r#"<div class="rail">{segs}</div><div class="rail-labels">{labels}</div>"#)
}

/// Wrap step content with the wizard chrome.
///
/// `x_data` is an Alpine state expression (e.g. `"{ name: 'foo' }"`). Inputs
/// inside `content` should `x-model` into it. `progress_expr` is a JS
/// expression evaluating to a 0..1 float that drives the active rail
/// segment's fill.
fn wizard_shell(
    step: &str,
    _base_url: &str,
    x_data: &str,
    progress_expr: &str,
    content: &str,
) -> String {
    let idx = STEPS.iter().position(|(id, _)| *id == step).unwrap_or(0);

    let inner = format!(
        r#"<div class="wizard" x-data="{x_data}" hx-ext="json-enc">
  <header class="top">
    {brand}
    <div class="rail-wrap">{rail}<div class="rail-counter mono muted">{step_num}/{total}</div></div>
    <div class="top-right">
      <a href="/auth/logout" class="btn ghost sm">Sign out</a>
    </div>
  </header>
  <main>{content}</main>
</div>"#,
        brand = brand_mark(),
        rail = rail_html(step, progress_expr),
        step_num = idx + 1,
        total = STEPS.len(),
        x_data = x_data,
        content = content,
    );

    base_html(&format!("Concierge - Setup"), &inner)
}

pub fn welcome_html(_base_url: &str) -> String {
    let header = super::base::public_nav_html("");

    let content = format!(
        r#"{header}
<section class="page welcome">
  <div class="welcome-left">
    <div class="eyebrow">// automated customer engagement</div>
    <h1 class="display">Hello. I'll be answering <br>every <em>DM, WhatsApp &amp; email</em> <br>so you don't have to.</h1>
    <p class="lead">Connect your channels, set a tone, and your concierge handles the rest. Auto-replies across WhatsApp, Instagram, Discord, and email. 100 AI replies free every month — static replies always free.</p>
    <div class="row gap-12 wrap mt-16">
      <a href="/auth/login" class="btn primary lg">Get started &rarr;</a>
      <a href="/features" class="btn ghost lg">See features</a>
    </div>
  </div>
  <aside class="postcard" aria-hidden="true">
    <div class="postcard-card">
      <div class="postcard-head"><span class="mono muted">LOG &middot; TUE 09:47</span><span class="dot ok"></span></div>
      <div class="log-row"><span class="log-a">IG &nbsp;@leo</span><span class="log-b">hi what time u open</span></div>
      <div class="log-row"><span class="log-a">&rarr; &nbsp;concierge</span><span class="log-b">We're open 9-7 today! Walk-ins welcome</span></div>
      <div class="log-row"><span class="log-a">WA &nbsp;+61 431...</span><span class="log-b">can i move my booking</span></div>
      <div class="log-row"><span class="log-a">&rarr; &nbsp;concierge</span><span class="log-b">Yes - what day works better for you?</span></div>
      <div class="log-row"><span class="log-a">&nbsp;&#x2709; &nbsp;orders@</span><span class="log-b">invoice {hash}8821</span></div>
      <div class="log-row"><span class="log-a">&rarr; &nbsp;discord</span><span class="log-b">forwarded &middot; silent</span></div>
      <div class="mono muted fs-10" style="margin-top:10px;letter-spacing:.18em">142 handled today &middot; 0 sent to you</div>
    </div>
    <div class="stamp">ON<br>DUTY<br>24/7</div>
  </aside>
</section>"#,
        header = header,
        hash = HASH,
    );

    base_html("Concierge - Automated customer messaging", &content)
}

pub fn basics_html(business: &crate::types::BusinessInfo, base_url: &str) -> String {
    let biz_type_options = [
        ("", "Select type..."),
        ("unregistered", "Unregistered / Individual"),
        ("sole_proprietorship", "Sole Proprietorship"),
        ("partnership", "Partnership"),
        ("pvt_ltd", "Private Limited"),
        ("llp", "LLP"),
    ];
    let biz_type_html: String = biz_type_options
        .iter()
        .map(|(val, label)| {
            let sel = if business.business_type == *val {
                " selected"
            } else {
                ""
            };
            format!(r#"<option value="{val}"{sel}>{label}</option>"#)
        })
        .collect();

    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="mono muted">01 / 05</span><span class="eyebrow">The basics</span></div>
  <h2 class="display-md">Tell us about you.</h2>
  <p class="lead">For invoicing and compliance. Your details are never shared.</p>
  <form hx-post="{base_url}/admin/wizard/basics" hx-target="body" hx-swap="innerHTML">
    <div class="card p-24">
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px">
        <div>
          <label class="eyebrow lbl">Brand name *</label>
          <input class="input" name="name" value="{name}" placeholder="Blossom Florist" required x-model="name">
        </div>
        <div>
          <label class="eyebrow lbl">Your name *</label>
          <input class="input" name="contact_name" value="{contact_name}" placeholder="Full name">
        </div>
        <div>
          <label class="eyebrow lbl">Phone *</label>
          <input class="input" type="tel" name="phone" value="{phone}" placeholder="+91 98765 43210" required x-model="phone">
        </div>
        <div>
          <label class="eyebrow lbl">Entity type</label>
          <select class="select" name="business_type" x-model="bizType">{biz_type_html}</select>
        </div>
      </div>
      <div class="mt-16" x-show="bizType &amp;&amp; bizType !== 'unregistered'" x-cloak style="grid-template-columns:1fr 1fr;gap:16px;display:grid">
        <div>
          <label class="eyebrow lbl">PAN</label>
          <input class="input" name="pan" value="{pan}" placeholder="ABCDE1234F" style="text-transform:uppercase">
        </div>
        <div>
          <label class="eyebrow lbl">GSTIN <span class="muted">(optional)</span></label>
          <input class="input" name="gstin" value="{gstin}" placeholder="22AAAAA0000A1Z5" style="text-transform:uppercase">
        </div>
        <div style="grid-column:1/-1">
          <label class="eyebrow lbl">Registered address</label>
          <textarea class="textarea" name="address" rows="2" placeholder="Shop 12, Main Road...">{address}</textarea>
        </div>
        <div>
          <label class="eyebrow lbl">State</label>
          <input class="input" name="state" value="{state}" placeholder="Tamil Nadu">
        </div>
        <div>
          <label class="eyebrow lbl">Pincode</label>
          <input class="input" name="pincode" value="{pincode}" placeholder="600001" pattern="[0-9]{{6}}" maxlength="6">
        </div>
      </div>
    </div>
    <div class="between mt-36">
      <a href="/" class="btn ghost">&larr; Back</a>
      <button class="btn primary" type="submit" :disabled="!(name &amp;&amp; name.trim() &amp;&amp; phone &amp;&amp; phone.trim())">Continue &rarr;</button>
    </div>
  </form>
</section>"#,
        base_url = base_url,
        name = html_escape(&business.name),
        contact_name = html_escape(&business.contact_name),
        phone = html_escape(&business.phone),
        biz_type_html = biz_type_html,
        pan = html_escape(&business.pan),
        gstin = html_escape(&business.gstin),
        address = html_escape(&business.address),
        state = html_escape(&business.state),
        pincode = html_escape(&business.pincode),
    );

    let x_data = format!(
        "{{ name: '{}', phone: '{}', bizType: '{}' }}",
        js_attr_escape(&business.name),
        js_attr_escape(&business.phone),
        js_attr_escape(&business.business_type),
    );
    let progress_expr =
        "((name && name.trim() ? 0.4 : 0) + (phone && phone.trim() ? 0.4 : 0) + (bizType ? 0.2 : 0))";

    wizard_shell("basics", base_url, &x_data, progress_expr, &content)
}

pub fn connect_html(
    ig_connected: bool,
    wa_connected: bool,
    email_addresses: &[crate::types::EmailAddress],
    suggested_slug: &str,
    email_base_domain: &str,
    _currency: &str,
    tenant_id: &str,
    discord: Option<&crate::types::DiscordConfig>,
    base_url: &str,
) -> String {
    let ig_card = channel_card(
        "ig",
        "Instagram DMs",
        ig_connected,
        "@blossom.florist",
        "Meta login. We'll read DMs from your business account.",
        tenant_id,
        base_url,
    );
    let wa_card = channel_card(
        "wa",
        "WhatsApp Business",
        wa_connected,
        "+61 431 555 019",
        "Uses your Meta Business access token + phone number ID.",
        tenant_id,
        base_url,
    );
    let (dc_connected, dc_handle) = match discord {
        Some(c) => (
            true,
            c.guild_name.clone().unwrap_or_else(|| "Connected".into()),
        ),
        None => (false, String::new()),
    };
    let dc_card = channel_card(
        "discord",
        "Discord",
        dc_connected,
        &dc_handle,
        "Install the bot to relay messages, approve AI drafts, and run slash commands.",
        tenant_id,
        base_url,
    );

    let email_rows: String = email_addresses
        .iter()
        .map(|a| {
            let full = format!("{}@{}", a.local_part, email_base_domain);
            format!(
                r#"<div class="side-row" style="padding:10px 14px">
  <span>{mail_icon}</span>
  <div class="flex-1"><span class="mono fs-13">{full}</span></div>
  <button class="btn ghost sm text-warn" hx-post="{base_url}/admin/wizard/email/remove" hx-vals='{{"label":"{label}"}}' hx-target="body" hx-swap="innerHTML">Remove</button>
</div>"#,
                mail_icon = channel_icon("mail"),
                full = html_escape(&full),
                label = html_escape(&a.local_part),
                base_url = base_url,
            )
        })
        .collect();

    let email_section = if email_base_domain.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="channel" style="grid-column:1/-1">
  <div class="channel-head">
    <div class="channel-mark">{mail_icon}</div>
    <div><div class="channel-name">Email</div></div>
  </div>
  <div class="channel-body">
    <p class="muted m-0 mb-12">Pick a name to receive mail at <code>name@{base_domain}</code>. Replies go to the sender; you and your team get a copy via Cc/Bcc.</p>
    {email_rows}
    <form hx-post="{base_url}/admin/wizard/email/add" hx-target="body" hx-swap="innerHTML"
          class="row gap-8 mt-8">
      <input class="input fs-13" type="text" name="label" value="{slug}" placeholder="your-name" style="max-width:160px">
      <span class="mono muted fs-13">@{base_domain}</span>
      <button type="submit" class="btn sm ml-auto">Add</button>
    </form>
    <div class="mono muted fs-11 mt-6">First address is free. Need more? Buy a 5-pack later for ₹49 / 50¢.</div>
  </div>
</div>"#,
            mail_icon = channel_icon("mail"),
            email_rows = email_rows,
            base_url = base_url,
            slug = html_escape(suggested_slug),
            base_domain = html_escape(email_base_domain),
        )
    };

    let has_anything = ig_connected || wa_connected || !email_addresses.is_empty();
    let continue_label = if has_anything {
        "Continue &rarr;"
    } else {
        "Skip &rarr;"
    };

    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="mono muted">02 / 05</span><span class="eyebrow">Plug in</span></div>
  <h2 class="display-md">Where do your customers already talk to you?</h2>
  <p class="lead">Connect your channels. Skip anything you don't use &mdash; you can add more from the dashboard later.</p>
  <div class="channels-grid">{ig_card}{wa_card}{dc_card}{email_section}</div>
  <div class="between mt-36">
    <button class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"basics"}}' hx-target="body" hx-swap="innerHTML">&larr; Back</button>
    <button class="btn primary" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"notifications"}}' hx-target="body" hx-swap="innerHTML">{continue_label}</button>
  </div>
</section>"#,
        ig_card = ig_card,
        wa_card = wa_card,
        dc_card = dc_card,
        email_section = email_section,
        base_url = base_url,
        continue_label = continue_label,
    );

    // Progress: 30% Instagram, 30% WhatsApp, 20% Discord, 20% any email address.
    let x_data = format!(
        "{{ ig: {}, wa: {}, dc: {}, emails: {} }}",
        ig_connected,
        wa_connected,
        dc_connected,
        email_addresses.len(),
    );
    let progress_expr =
        "((ig ? 0.3 : 0) + (wa ? 0.3 : 0) + (dc ? 0.2 : 0) + (emails > 0 ? 0.2 : 0))";

    wizard_shell("channels", base_url, &x_data, progress_expr, &content)
}

/// Props for a channel card. Shared between the wizard Channels step and the
/// Settings "Integrations" section so the UI stays identical.
pub struct ChannelCardProps<'a> {
    /// Icon key: "ig" | "wa" | "discord" | "mail".
    pub key: &'a str,
    pub name: &'a str,
    pub connected: bool,
    /// One-line status: handle/identifier when connected, flavor text when not.
    pub status_line: &'a str,
    pub connect_href: &'a str,
    pub manage_href: &'a str,
}

pub fn channel_card_html(p: &ChannelCardProps) -> String {
    if p.connected {
        format!(
            r#"<div class="channel is-connected">
  <div class="ribbon">connected</div>
  <div class="channel-head">
    <div class="channel-mark">{icon}</div>
    <div><div class="channel-name">{name}</div></div>
    <span class="dot ok ml-auto"></span>
  </div>
  <div class="channel-body">
    <div class="mono text-ok fs-12">&#x25CF; active</div>
    <div class="serif mt-4" style="font-size:18px;line-height:1.2">{status}</div>
  </div>
  <div class="row gap-8">
    <a href="{manage_href}" class="btn ghost sm">Manage</a>
  </div>
</div>"#,
            icon = channel_icon(p.key),
            name = html_escape(p.name),
            status = html_escape(p.status_line),
            manage_href = p.manage_href,
        )
    } else {
        format!(
            r#"<div class="channel">
  <div class="channel-head">
    <div class="channel-mark">{icon}</div>
    <div><div class="channel-name">{name}</div></div>
    <span class="dot ml-auto"></span>
  </div>
  <div class="channel-body"><p class="muted m-0">{flavor}</p></div>
  <a href="{connect_href}" class="btn">Connect &rarr;</a>
</div>"#,
            icon = channel_icon(p.key),
            name = html_escape(p.name),
            flavor = html_escape(p.status_line),
            connect_href = p.connect_href,
        )
    }
}

// Thin wrapper kept for the wizard's `connect_html` call sites.
fn channel_card(
    key: &str,
    name: &str,
    connected: bool,
    handle: &str,
    flavor: &str,
    tenant_id: &str,
    base_url: &str,
) -> String {
    let connect_href = match key {
        "ig" => format!("{base_url}/instagram/auth/{}", html_escape(tenant_id)),
        "wa" => format!("{base_url}/admin/whatsapp/new"),
        "discord" => format!("{base_url}/admin/discord/install?from=wizard_channels"),
        _ => format!("{base_url}/admin/{key}"),
    };
    let manage_href = match key {
        "ig" => format!("{base_url}/admin/instagram"),
        "wa" => format!("{base_url}/admin/whatsapp"),
        "discord" => format!("{base_url}/admin/discord"),
        _ => format!("{base_url}/admin/{key}"),
    };
    let status_line = if connected { handle } else { flavor };
    channel_card_html(&ChannelCardProps {
        key,
        name,
        connected,
        status_line,
        connect_href: &connect_href,
        manage_href: &manage_href,
    })
}

pub fn channel_icon(key: &str) -> &'static str {
    match key {
        "ig" => {
            r#"<svg width="22" height="22" viewBox="0 0 24 24" fill="none"><rect x="3" y="3" width="18" height="18" rx="5" stroke="currentColor" stroke-width="1.6"/><circle cx="12" cy="12" r="4.2" stroke="currentColor" stroke-width="1.6"/><circle cx="17.2" cy="6.8" r="1.1" fill="currentColor"/></svg>"#
        }
        "wa" => {
            r#"<svg width="22" height="22" viewBox="0 0 24 24" fill="none"><path d="M4 20l1.3-4.1A8 8 0 1 1 8.2 18.8L4 20z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg>"#
        }
        "discord" => {
            r#"<svg width="22" height="22" viewBox="0 0 24 24" fill="none"><path d="M7 7c1.4-.7 3-1 5-1s3.6.3 5 1l1 1 1.5 4.5c.2 2-.3 3.8-1.5 5.5-1 .3-2 .5-3 .5l-1-1.5c.5-.2 1-.4 1.5-.8-.3-.2-.8-.4-1.2-.5-2 .7-4.6.7-6.6 0-.4.1-.9.3-1.2.5.5.4 1 .6 1.5.8L6 17.5c-1 0-2-.2-3-.5-1.2-1.7-1.7-3.5-1.5-5.5L3 7l1-1z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/></svg>"#
        }
        "mail" => {
            r#"<svg width="22" height="22" viewBox="0 0 24 24" fill="none"><rect x="3" y="5" width="18" height="14" rx="2" stroke="currentColor" stroke-width="1.6"/><path d="M3.5 6.5l8.5 6 8.5-6" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg>"#
        }
        _ => "",
    }
}

pub fn notifications_html(
    config: &crate::types::NotificationConfig,
    discord_installed: bool,
    base_url: &str,
) -> String {
    let freq_options = |current: u32, opts: &[(u32, &str)]| -> String {
        opts.iter()
            .map(|(val, label)| {
                let sel = if current == *val { " selected" } else { "" };
                format!(r#"<option value="{val}"{sel}>{label}</option>"#)
            })
            .collect()
    };

    let approval_freq_html = freq_options(
        config.approval_email_frequency_minutes,
        &[(5, "5 min"), (15, "15 min"), (30, "30 min"), (60, "1 hour")],
    );
    let digest_freq_html = freq_options(
        config.digest_email_frequency_minutes,
        &[
            (1440, "Daily"),
            (10080, "Weekly"),
            (43200, "Monthly"),
            (129600, "Quarterly"),
        ],
    );

    let b = |v: bool| if v { "true" } else { "false" };

    let content = format!(
        r##"<section class="page narrow">
  <div class="section-label"><span class="mono muted">03 / 05</span><span class="eyebrow">Heads up</span></div>
  <h2 class="display-md">How should we notify you?</h2>
  <p class="lead">Approvals are required — that's how the AI asks you before sending. Digests are optional.</p>

  <form hx-post="{base_url}/admin/wizard/notifications" hx-target="#notif-toast" hx-swap="innerHTML">
    <div class="card p-22 mb-16">
      <div class="eyebrow mb-12">AI reply approvals <span class="text-warn">*</span></div>
      <p class="muted mb-14 fs-14">When the AI drafts a reply, where should we ask you to approve it? Pick at least one.</p>
      <div class="admin-grid">
        <label class="admin-card" :class="approval.discord ? 'selected' : ''" style="min-height:auto;cursor:pointer">
          <input type="hidden" name="approval_discord" value="false">
          <input type="checkbox" name="approval_discord" value="true" class="hidden" x-model="approval.discord">
          <div class="row gap-12">
            <div class="admin-mark icon-chip">{discord_icon}</div>
            <div><div class="fw-600">Discord</div>
            <div class="mono muted fs-11">real-time threads</div></div>
          </div>
        </label>
        <label class="admin-card" :class="approval.email ? 'selected' : ''" style="min-height:auto;cursor:pointer">
          <input type="hidden" name="approval_email" value="false">
          <input type="checkbox" name="approval_email" value="true" class="hidden" x-model="approval.email">
          <div class="row gap-12">
            <div class="admin-mark icon-chip">{mail_icon}</div>
            <div><div class="fw-600">Email</div>
            <div class="mono muted fs-11">batched digest</div></div>
          </div>
          <div class="freq-row row gap-8 mt-12" x-show="approval.email" x-cloak>
            <span class="mono muted fs-12">Every</span>
            <select class="select fs-13" name="approval_freq" style="width:auto;padding:6px 10px">{approval_freq_html}</select>
          </div>
        </label>
      </div>
      <div class="card-soft p-14 mt-12" x-show="(approval.discord || digest.discord) && !{dc_installed_js}" x-cloak>
        <div class="row gap-12">
          <div class="fs-13 flex-1">Discord isn't installed yet. You need the bot in a server before approvals or digests land there.</div>
          <a href="{base_url}/admin/discord/install?from=wizard_heads_up" class="btn sm primary">Install Discord</a>
        </div>
      </div>
    </div>

    <div class="card p-22">
      <div class="eyebrow mb-12">Activity digest <span class="muted">(optional)</span></div>
      <p class="muted mb-14 fs-14">A periodic summary of messages handled, credits used, and anything that needs attention.</p>
      <div class="admin-grid">
        <label class="admin-card" :class="digest.discord ? 'selected' : ''" style="min-height:auto;cursor:pointer">
          <input type="hidden" name="digest_discord" value="false">
          <input type="checkbox" name="digest_discord" value="true" class="hidden" x-model="digest.discord">
          <div class="row gap-12">
            <div class="admin-mark icon-chip">{discord_icon}</div>
            <div><div class="fw-600">Discord</div>
            <div class="mono muted fs-11">channel post</div></div>
          </div>
        </label>
        <label class="admin-card" :class="digest.email ? 'selected' : ''" style="min-height:auto;cursor:pointer">
          <input type="hidden" name="digest_email" value="false">
          <input type="checkbox" name="digest_email" value="true" class="hidden" x-model="digest.email">
          <div class="row gap-12">
            <div class="admin-mark icon-chip">{mail_icon}</div>
            <div><div class="fw-600">Email</div>
            <div class="mono muted fs-11">periodic summary</div></div>
          </div>
          <div class="freq-row row gap-8 mt-12" x-show="digest.email" x-cloak>
            <span class="mono muted fs-12">Every</span>
            <select class="select fs-13" name="digest_freq" style="width:auto;padding:6px 10px">{digest_freq_html}</select>
          </div>
        </label>
      </div>
    </div>

    <div class="between mt-36">
      <button type="button" class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"channels"}}' hx-target="body" hx-swap="innerHTML">&larr; Back</button>
      <button type="submit" class="btn primary" :disabled="!approval.discord && !approval.email">Continue &rarr;</button>
    </div>
    <div id="notif-toast" class="mt-12"></div>
  </form>
</section>"##,
        base_url = base_url,
        discord_icon = channel_icon("discord"),
        mail_icon = channel_icon("mail"),
        approval_freq_html = approval_freq_html,
        digest_freq_html = digest_freq_html,
        dc_installed_js = if discord_installed { "true" } else { "false" },
    );

    let x_data = format!(
        "{{ approval: {{ discord: {ad}, email: {ae} }}, digest: {{ discord: {dd}, email: {de} }} }}",
        ad = b(config.approval_discord),
        ae = b(config.approval_email),
        dd = b(config.digest_discord),
        de = b(config.digest_email),
    );
    // Required approval gives 60%; optional digest adds up to 40%.
    let progress_expr =
        "((approval.discord || approval.email) ? 0.6 : 0) + ((digest.discord || digest.email) ? 0.4 : 0)";

    wizard_shell("notifications", base_url, &x_data, progress_expr, &content)
}

fn sel_attr(current: &str, value: &str) -> &'static str {
    if current == value {
        " selected"
    } else {
        ""
    }
}

pub fn replies_html(persona: &PersonaConfig, canned: &[CannedReply], base_url: &str) -> String {
    let rows: String = canned
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                r#"<div class="replies-row">
  <input class="input mono" name="trigger_{i}" value="{trigger}" placeholder="hours | open | *closed*">
  <textarea class="textarea" name="reply_{i}" style="min-height:60px;font-family:var(--f-body)" placeholder="Hi! We're open 9-7...">{reply}</textarea>
  <button class="btn ghost icon" hx-post="{base_url}/admin/wizard/replies/del" hx-vals='{{"i":"{i}"}}' hx-target="body" hx-swap="innerHTML">&#x2715;</button>
</div>"#,
                i = i,
                trigger = html_escape(&r.trigger),
                reply = html_escape(&r.reply),
                base_url = base_url,
            )
        })
        .collect();

    let empty = if canned.is_empty() {
        r#"<div class="replies-row"><span class="muted ta-center" style="grid-column:1/-1">No canned replies yet. Add one below — or just let the AI handle everything.</span></div>"#
    } else {
        ""
    };

    let prompt = persona.to_system_prompt();

    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="mono muted">04 / 05</span><span class="eyebrow">Replies</span></div>
  <h2 class="display-md">How should your concierge respond?</h2>
  <p class="lead">Configure the AI voice for general replies, then add canned overrides for specific questions.</p>

  <div class="card p-22 mb-16">
    <div class="eyebrow mb-12">AI persona</div>
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px">
      <div>
        <label class="eyebrow lbl">Type of business</label>
        <input class="input" name="biz_type" value="{biz_type}" placeholder="florist, hair salon, coffee shop..." x-model="bizType">
      </div>
      <div>
        <label class="eyebrow lbl">City</label>
        <input class="input" name="city" value="{city}" placeholder="Chennai, Berlin..." x-model="city">
      </div>
      <div>
        <label class="eyebrow lbl">Tone</label>
        <select class="select" name="tone" x-model="tone">
          <option value="">Choose a tone...</option>
          <option value="warm &amp; chatty"{tone_wc}>warm &amp; chatty</option>
          <option value="concise &amp; professional"{tone_cp}>concise &amp; professional</option>
          <option value="playful with emoji"{tone_pe}>playful with emoji</option>
          <option value="old-school polite"{tone_op}>old-school polite</option>
        </select>
      </div>
      <div>
        <label class="eyebrow lbl">Never do this</label>
        <select class="select" name="never" x-model="never">
          <option value="">Choose a boundary...</option>
          <option value="quote prices"{never_qp}>quote prices</option>
          <option value="promise dates"{never_pd}>promise dates</option>
          <option value="handle refunds"{never_hr}>handle refunds</option>
        </select>
      </div>
    </div>
    <div class="card p-14 mt-16" style="background:var(--ink);color:var(--cream);border-color:var(--ink);border-radius:var(--r-sm)">
      <div class="mono fs-10 mb-6" style="letter-spacing:.18em;color:var(--accent-soft)">SYSTEM PROMPT</div>
      <pre class="mono m-0 fs-11" style="white-space:pre-wrap;color:var(--cream);line-height:1.5">{prompt}</pre>
    </div>
  </div>

  <div class="card p-22 mt-16">
    <div class="eyebrow mb-8">Wait before replying</div>
    <p class="muted fs-13 m-0 mb-12">If a customer fires a few messages in a row, hold off until they pause so the AI sees the whole burst at once. Default applies to every WhatsApp / Instagram / Discord channel; override per account in Settings.</p>
    <div class="row gap-12">
      <input type="range" min="0" max="30" step="1"
             x-model.number="waitSeconds"
             class="flex-1"
             style="accent-color:var(--accent)">
      <div class="mono ta-right" style="min-width:80px">
        <span x-text="waitSeconds === 0 ? 'instant' : waitSeconds + 's'"></span>
      </div>
    </div>
  </div>

  <form hx-post="{base_url}/admin/wizard/replies/save" hx-target="body" hx-swap="innerHTML">
    <input type="hidden" name="biz_type" value="{biz_type}">
    <input type="hidden" name="city" value="{city}">
    <input type="hidden" name="tone" value="{tone_raw}">
    <input type="hidden" name="never" value="{never_raw}">
    <input type="hidden" name="default_wait_seconds" :value="waitSeconds">
    <div class="card replies-card">
      <div class="eyebrow" style="padding:14px 20px 0">Canned replies <span class="muted">(optional)</span></div>
      <p class="muted fs-13" style="padding:4px 20px 0">These fire before the AI. Glob patterns work &mdash; <span class="mono">*</span> matches anything.</p>
      <div class="replies-head mt-12"><div>When message matches</div><div>Reply with</div><div></div></div>
      {rows}{empty}
      <div class="replies-add">
        <button type="button" class="btn ghost sm" hx-post="{base_url}/admin/wizard/replies/add" hx-target="body" hx-swap="innerHTML">+ Add reply</button>
      </div>
    </div>
    <div class="between mt-32">
      <button type="button" class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"notifications"}}' hx-target="body" hx-swap="innerHTML">&larr; Back</button>
      <button type="submit" class="btn primary">Continue &rarr;</button>
    </div>
  </form>
</section>"#,
        base_url = base_url,
        biz_type = html_escape(&persona.biz_type),
        city = html_escape(&persona.city),
        tone_raw = html_escape(&persona.tone),
        never_raw = html_escape(&persona.never),
        prompt = html_escape(&prompt),
        tone_wc = sel_attr(&persona.tone, "warm & chatty"),
        tone_cp = sel_attr(&persona.tone, "concise & professional"),
        tone_pe = sel_attr(&persona.tone, "playful with emoji"),
        tone_op = sel_attr(&persona.tone, "old-school polite"),
        never_qp = sel_attr(&persona.never, "quote prices"),
        never_pd = sel_attr(&persona.never, "promise dates"),
        never_hr = sel_attr(&persona.never, "handle refunds"),
        rows = rows,
        empty = empty,
    );

    let x_data = format!(
        "{{ bizType: '{}', city: '{}', tone: '{}', never: '{}', waitSeconds: {} }}",
        js_attr_escape(&persona.biz_type),
        js_attr_escape(&persona.city),
        js_attr_escape(&persona.tone),
        js_attr_escape(&persona.never),
        persona.default_wait_seconds,
    );
    let progress_expr =
        "((bizType ? 0.35 : 0) + (tone ? 0.35 : 0) + (city ? 0.15 : 0) + (never ? 0.15 : 0))";

    wizard_shell("replies", base_url, &x_data, progress_expr, &content)
}

pub fn launch_html(
    email_addresses: &[crate::types::EmailAddress],
    base_domain: &str,
    currency: &str,
    base_url: &str,
) -> String {
    let email_rows: String = email_addresses
        .iter()
        .map(|a| {
            let full = format!("{}@{}", a.local_part, base_domain);
            format!(
                r#"<div class="side-row" style="padding:10px 14px">
  <span>{mail_icon}</span>
  <div class="flex-1"><span class="mono fs-13">{full}</span></div>
</div>"#,
                mail_icon = channel_icon("mail"),
                full = html_escape(&full),
            )
        })
        .collect();

    let email_section = if email_addresses.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="card p-22 mb-16">
  <div class="eyebrow mb-8">Email addresses</div>
  <p class="muted mb-12 fs-14">These addresses are live. Inbound mail will be auto-replied if you turn on auto-reply for them in Email.</p>
  {email_rows}
</div>"#
        )
    };

    let credits_section = format!(
        r#"<div class="mb-16">
  {slider}
  <p class="muted ta-center mt-8 fs-12">Optional — 100 replies are free every month. Top up later from Billing if you need.</p>
</div>"#,
        slider = crate::templates::credit_slider::slider_html(
            currency,
            base_url,
            crate::templates::credit_slider::SliderMode::Buy {
                return_to: "/admin/wizard/launch",
            },
        ),
    );

    let status_card = r#"<div class="card p-22" style="border-color:var(--ok);background:linear-gradient(135deg,var(--paper),#E8F0DE)">
    <div class="row gap-12">
      <span class="dot ok"></span>
      <div>
        <div class="fw-600">Ready to go live</div>
        <p class="muted fs-14 m-0 mt-4">Hit finish to open your dashboard. Connect channels, set up email rules, and start receiving auto-replies.</p>
      </div>
    </div>
  </div>"#;

    let content = format!(
        r##"<section class="page narrow">
  <div class="section-label"><span class="mono muted">05 / 05</span><span class="eyebrow">Ship it</span></div>
  <h2 class="display-md">You're all set.</h2>
  <p class="lead">Your concierge is configured and ready to handle messages. Here's a summary of what's next.</p>

  {email_section}
  {credits_section}

  {status_card}

  <div class="between mt-36">
    <button class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"replies"}}' hx-target="body" hx-swap="innerHTML">&larr; Back</button>
    <button class="btn primary" hx-post="{base_url}/admin/wizard/complete" hx-target="body">Finish setup &rarr;</button>
  </div>
</section>"##,
        email_section = email_section,
        credits_section = credits_section,
        status_card = status_card,
        base_url = base_url,
    );

    // Progress on the launch step is always full: addresses are live the
    // moment they're added (no payment gate any more).
    let _ = email_addresses;
    let _ = currency;
    let x_data = "{}".to_string();
    let progress_expr = "1";

    wizard_shell("launch", base_url, &x_data, progress_expr, &content)
}

/// Public pricing page at /pricing
pub fn pricing_html(default_currency: &str) -> String {
    // Public pricing page. Visitors aren't logged in, so the slider's
    // checkout button is replaced with a sign-in CTA. The ?c= query param
    // carries the chosen currency so the toggle is shareable.
    let currency = if default_currency.eq_ignore_ascii_case("usd") {
        "USD"
    } else {
        "INR"
    };
    let slider = crate::templates::credit_slider::slider_html(
        currency,
        "",
        crate::templates::credit_slider::SliderMode::Preview {
            cta_href: "/auth/login",
            cta_label: "Sign in to buy",
        },
    );
    let (per_reply, pack_price) = if currency == "USD" {
        ("$0.02", "$0.50")
    } else {
        ("₹2", "₹49")
    };
    let (inr_cls, usd_cls) = if currency == "USD" {
        ("btn ghost sm", "btn sm")
    } else {
        ("btn sm", "btn ghost sm")
    };

    let nav = super::base::public_nav_html("pricing");
    let content = format!(
        r##"{nav}
<article class="legal">
  <div class="between">
    <h1 class="m-0">{per_reply} per AI reply. Everything else is free.</h1>
    <div class="row gap-8">
      <a href="/pricing?c=inr" class="{inr_cls}">INR</a>
      <a href="/pricing?c=usd" class="{usd_cls}">USD</a>
    </div>
  </div>
  <p class="muted">100 free AI replies every account every month. After that, top up with as many credits as you want — no tiers, no contracts. Purchased credits never expire.</p>

  <div style="margin:24px 0">{slider}</div>

  <div class="card p-18">
    <div class="eyebrow mb-8">What costs a credit?</div>
    <ul class="muted m-0">
      <li><strong>AI auto-replies</strong> on WhatsApp, Instagram, email, or Discord: <strong>1 credit each.</strong></li>
      <li><strong>Static auto-replies</strong> (canned text you wrote yourself): always <strong>free</strong>.</li>
      <li>Inbound messages, notification CCs/BCCs, Discord relay, slash commands: always <strong>free</strong>.</li>
    </ul>
  </div>

  <h2 style="margin-top:2rem">Email addresses</h2>
  <p class="muted">Each address you set up at <code>name@cncg.email</code> can auto-reply to inbound mail. Replies go to the original sender; you and your team get a copy via Cc/Bcc.</p>

  <div class="card p-18" style="margin:24px 0">
    <p class="m-0"><strong>1 address free per account.</strong> Need more? Buy a 5-pack for <strong>{pack_price}</strong> — one-time charge, never expires.</p>
    <p class="muted" style="margin:8px 0 0">Static replies stay free; AI replies draw from your credit balance above.</p>
  </div>
</article>"##,
        nav = nav,
        per_reply = per_reply,
        pack_price = pack_price,
        slider = slider,
        inr_cls = inr_cls,
        usd_cls = usd_cls,
    );

    base_html_with_meta(
        "Pricing - Concierge",
        &content,
        &PageMeta {
            description: "Simple, prepaid pricing for Concierge. ₹2 / $0.02 per AI reply, no tiers. Static auto-replies free. 100 free AI replies every month. 1 free email address; 5-packs from ₹49 / $0.50.",
            og_title: "Concierge Pricing",
            ..PageMeta::default()
        },
    )
}
