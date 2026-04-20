//! Onboarding wizard templates

use crate::helpers::html_escape;
use crate::types::*;

use super::base::{base_html, base_html_with_meta, brand_mark, PageMeta};
use super::HASH;

const STEPS: &[(&str, &str)] = &[
    ("basics", "The basics"),
    ("channels", "Plug in"),
    ("notifications", "Heads up"),
    ("replies", "Replies"),
    ("launch", "Ship it"),
];

fn rail_html(current: &str) -> String {
    let idx = STEPS.iter().position(|(id, _)| *id == current).unwrap_or(0);

    let segs: String = STEPS
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let class = if i < idx {
                "seg done"
            } else if i == idx {
                "seg active"
            } else {
                "seg"
            };
            format!(r#"<div class="{class}"><span class="fill"></span></div>"#)
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

fn wizard_shell(step: &str, base_url: &str, content: &str) -> String {
    let idx = STEPS.iter().position(|(id, _)| *id == step).unwrap_or(0);

    let inner = format!(
        r#"<div class="wizard" hx-ext="json-enc">
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
        rail = rail_html(step),
        step_num = idx + 1,
        total = STEPS.len(),
        content = content,
    );

    base_html(&format!("Concierge - Setup"), &inner)
}

pub fn welcome_html(_base_url: &str) -> String {
    let header = format!(
        r#"<header class="site-header">
  {brand}
  <div style="margin-left:auto"><a href="/auth/login" class="btn ghost sm">Sign in</a></div>
</header>"#,
        brand = brand_mark(),
    );

    let content = format!(
        r#"{header}
<section class="page welcome">
  <div class="welcome-left">
    <div class="eyebrow">// automated customer engagement</div>
    <h1 class="display">Hello. I'll be answering <br>every <em>DM, WhatsApp &amp; email</em> <br>so you don't have to.</h1>
    <p class="lead">Connect your channels, set a tone, and your concierge handles the rest. Auto-replies across WhatsApp, Instagram, and email. 100 replies free every month.</p>
    <a href="/auth/login" class="btn primary lg">Get started &rarr;</a>
    <div class="mono fineprint" style="margin-top:18px">
      &#x25E6; <a href="/pricing" style="color:var(--muted)">pricing</a>
      &nbsp; &#x25E6; <a href="https://github.com/ananthb/concierge-worker" style="color:var(--muted)">open-source</a>
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
      <div class="mono muted" style="margin-top:10px;font-size:10px;letter-spacing:.18em">142 handled today &middot; 0 sent to you</div>
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
    let disabled = if business.name.is_empty() || business.phone.is_empty() {
        " disabled"
    } else {
        ""
    };

    let biz_type_options = [
        ("", "Select type..."),
        ("unregistered", "Unregistered / Individual"),
        ("sole_proprietorship", "Sole Proprietorship"),
        ("partnership", "Partnership"),
        ("pvt_ltd", "Private Limited"),
        ("llp", "LLP"),
    ];
    let is_registered =
        !business.business_type.is_empty() && business.business_type != "unregistered";
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

    let reg_display = if is_registered { "grid" } else { "none" };

    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="mono muted">01 / 05</span><span class="eyebrow">The basics</span></div>
  <h2 class="display-md">Tell us about you.</h2>
  <p class="lead">For invoicing and compliance. Your details are never shared.</p>
  <form hx-post="{base_url}/admin/wizard/basics" hx-target="body" hx-swap="innerHTML">
    <div class="card" style="padding:24px">
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px">
        <div>
          <label class="eyebrow" style="display:block;margin-bottom:6px">Brand name *</label>
          <input class="input" name="name" value="{name}" placeholder="Blossom Florist" required>
        </div>
        <div>
          <label class="eyebrow" style="display:block;margin-bottom:6px">Your name *</label>
          <input class="input" name="contact_name" value="{contact_name}" placeholder="Full name">
        </div>
        <div>
          <label class="eyebrow" style="display:block;margin-bottom:6px">Phone *</label>
          <input class="input" type="tel" name="phone" value="{phone}" placeholder="+91 98765 43210" required>
        </div>
        <div>
          <label class="eyebrow" style="display:block;margin-bottom:6px">Entity type</label>
          <select class="select" name="business_type" id="entity-type" onchange="document.getElementById('reg-fields').style.display=this.value&amp;&amp;this.value!=='unregistered'?'grid':'none'">{biz_type_html}</select>
        </div>
      </div>
      <div id="reg-fields" style="display:{reg_display};grid-template-columns:1fr 1fr;gap:16px;margin-top:16px">
        <div>
          <label class="eyebrow" style="display:block;margin-bottom:6px">PAN</label>
          <input class="input" name="pan" value="{pan}" placeholder="ABCDE1234F" style="text-transform:uppercase">
        </div>
        <div>
          <label class="eyebrow" style="display:block;margin-bottom:6px">GSTIN <span class="muted">(optional)</span></label>
          <input class="input" name="gstin" value="{gstin}" placeholder="22AAAAA0000A1Z5" style="text-transform:uppercase">
        </div>
        <div style="grid-column:1/-1">
          <label class="eyebrow" style="display:block;margin-bottom:6px">Registered address</label>
          <textarea class="textarea" name="address" rows="2" placeholder="Shop 12, Main Road...">{address}</textarea>
        </div>
        <div>
          <label class="eyebrow" style="display:block;margin-bottom:6px">State</label>
          <input class="input" name="state" value="{state}" placeholder="Tamil Nadu">
        </div>
        <div>
          <label class="eyebrow" style="display:block;margin-bottom:6px">Pincode</label>
          <input class="input" name="pincode" value="{pincode}" placeholder="600001" pattern="[0-9]{{6}}" maxlength="6">
        </div>
      </div>
    </div>
    <div class="between" style="margin-top:36px">
      <a href="/" class="btn ghost">&larr; Back</a>
      <button id="basics-continue" class="btn primary" type="submit"{disabled}>Continue &rarr;</button>
    </div>
  </form>
</section>
<script>
(function() {{
  var form = document.querySelector('form');
  var btn = document.getElementById('basics-continue');
  if (form && btn) {{
    function update() {{
      var name = form.querySelector('[name=name]').value.trim();
      var phone = form.querySelector('[name=phone]').value.trim();
      btn.disabled = name.length === 0 || phone.length === 0;
    }}
    form.addEventListener('input', update);
    update();
  }}
}})();
</script>"#,
        base_url = base_url,
        name = html_escape(&business.name),
        contact_name = html_escape(&business.contact_name),
        phone = html_escape(&business.phone),
        biz_type_html = biz_type_html,
        reg_display = reg_display,
        pan = html_escape(&business.pan),
        gstin = html_escape(&business.gstin),
        address = html_escape(&business.address),
        state = html_escape(&business.state),
        pincode = html_escape(&business.pincode),
        disabled = disabled,
    );

    wizard_shell("basics", base_url, &content)
}

pub fn connect_html(
    ig_connected: bool,
    wa_connected: bool,
    email_subdomains: &[crate::types::EmailSubdomain],
    suggested_slug: &str,
    email_base_domain: &str,
    base_url: &str,
) -> String {
    let ig_card = channel_card(
        "ig",
        "Instagram DMs",
        ig_connected,
        "@blossom.florist",
        "Meta login. We'll read DMs from your business account.",
        base_url,
    );
    let wa_card = channel_card(
        "wa",
        "WhatsApp Business",
        wa_connected,
        "+61 431 555 019",
        "Uses your Meta Business access token + phone number ID.",
        base_url,
    );

    let email_rows: String = email_subdomains
        .iter()
        .map(|d| {
            format!(
                r#"<div class="side-row" style="padding:10px 14px">
  <span>{mail_icon}</span>
  <div style="flex:1"><span class="mono" style="font-size:13px">{domain}</span></div>
  <button class="btn ghost sm" style="color:var(--warn)" hx-post="{base_url}/admin/wizard/email/remove" hx-vals='{{"label":"{label}"}}' hx-target="body" hx-swap="innerHTML">Remove</button>
</div>"#,
                mail_icon = channel_icon("mail"),
                domain = html_escape(&d.domain),
                label = html_escape(&d.label),
                base_url = base_url,
            )
        })
        .collect();

    let email_empty = if email_subdomains.is_empty() && email_base_domain.is_empty() {
        ""
    } else {
        ""
    };

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
    <p class="muted" style="margin:0 0 12px">Get a dedicated email address. Route, forward, or auto-reply with AI.</p>
    {email_rows}
    <form hx-post="{base_url}/admin/wizard/email/add" hx-target="body" hx-swap="innerHTML"
          style="display:flex;gap:8px;align-items:center;margin-top:8px">
      <input class="input" type="text" name="label" value="{slug}" placeholder="your-name" style="max-width:160px;font-size:13px">
      <span class="mono muted" style="font-size:13px">.{base_domain}</span>
      <button type="submit" class="btn sm" style="margin-left:auto">Add</button>
    </form>
    <div class="mono muted" style="font-size:11px;margin-top:6px">&#x20B9;199 / $2 per month per subdomain. Billed at the end.</div>
  </div>
</div>"#,
            mail_icon = channel_icon("mail"),
            email_rows = email_rows,
            base_url = base_url,
            slug = html_escape(suggested_slug),
            base_domain = html_escape(email_base_domain),
        )
    };

    let has_anything = ig_connected || wa_connected || !email_subdomains.is_empty();
    let continue_label = if has_anything {
        "Continue &rarr;"
    } else {
        "Skip &rarr;"
    };

    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="mono muted">02 / 06</span><span class="eyebrow">Plug in</span></div>
  <h2 class="display-md">Where do your customers already talk to you?</h2>
  <p class="lead">Connect your channels. Skip anything you don't use &mdash; you can add more from the dashboard later.</p>
  <div class="channels-grid">{ig_card}{wa_card}{email_section}</div>
  <div class="between" style="margin-top:36px">
    <button class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"basics"}}' hx-target="body" hx-swap="innerHTML">&larr; Back</button>
    <button class="btn primary" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"notifications"}}' hx-target="body" hx-swap="innerHTML">{continue_label}</button>
  </div>
</section>"#,
        ig_card = ig_card,
        wa_card = wa_card,
        email_section = email_section,
        base_url = base_url,
        continue_label = continue_label,
    );

    wizard_shell("channels", base_url, &content)
}

fn channel_card(
    key: &str,
    name: &str,
    connected: bool,
    handle: &str,
    flavor: &str,
    base_url: &str,
) -> String {
    if connected {
        format!(
            r#"<div class="channel is-connected">
  <div class="ribbon">connected</div>
  <div class="channel-head">
    <div class="channel-mark">{icon}</div>
    <div><div class="channel-name">{name}</div></div>
    <span class="dot ok" style="margin-left:auto"></span>
  </div>
  <div class="channel-body">
    <div class="mono" style="color:var(--ok);font-size:12px">&#x25CF; active</div>
    <div class="serif" style="font-size:22px;line-height:1.1;margin-top:4px">{handle}</div>
  </div>
  <div class="row gap-8">
    <a href="{base_url}/admin/{key_path}" class="btn ghost sm">Manage</a>
  </div>
</div>"#,
            icon = channel_icon(key),
            name = html_escape(name),
            handle = html_escape(handle),
            base_url = base_url,
            key_path = if key == "ig" { "instagram" } else { "whatsapp" },
        )
    } else {
        format!(
            r#"<div class="channel">
  <div class="channel-head">
    <div class="channel-mark">{icon}</div>
    <div><div class="channel-name">{name}</div></div>
    <span class="dot" style="margin-left:auto"></span>
  </div>
  <div class="channel-body"><p class="muted" style="margin:0">{flavor}</p></div>
  <a href="{base_url}/admin/{key_path}" class="btn">Connect &rarr;</a>
</div>"#,
            icon = channel_icon(key),
            name = html_escape(name),
            flavor = html_escape(flavor),
            base_url = base_url,
            key_path = if key == "ig" { "instagram" } else { "whatsapp" },
        )
    }
}

fn channel_icon(key: &str) -> &'static str {
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

pub fn notifications_html(config: &crate::types::NotificationConfig, base_url: &str) -> String {
    let ad = if config.approval_discord {
        " selected"
    } else {
        ""
    };
    let ae = if config.approval_email {
        " selected"
    } else {
        ""
    };
    let dd = if config.digest_discord {
        " selected"
    } else {
        ""
    };
    let de = if config.digest_email { " selected" } else { "" };

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
            (60, "1 hour"),
            (240, "4 hours"),
            (720, "12 hours"),
            (1440, "Daily"),
        ],
    );

    let content = format!(
        r##"<section class="page narrow">
  <div class="section-label"><span class="mono muted">03 / 06</span><span class="eyebrow">Heads up</span></div>
  <h2 class="display-md">How should we notify you?</h2>
  <p class="lead">Pick where you want to receive AI approval requests and activity digests. You can choose both.</p>

  <form hx-post="{base_url}/admin/wizard/notifications" hx-target="body" hx-swap="innerHTML">
    <div class="card" style="padding:22px;margin-bottom:16px">
      <div class="eyebrow" style="margin-bottom:12px">AI reply approvals</div>
      <p class="muted" style="margin-bottom:14px;font-size:14px">When the AI drafts a reply, where should we ask you to approve it?</p>
      <div class="admin-grid">
        <label class="admin-card{ad}" style="min-height:auto;cursor:pointer">
          <input type="hidden" name="approval_discord" value="false">
          <input type="checkbox" name="approval_discord" value="true" style="display:none" onchange="this.closest('.admin-card').classList.toggle('selected',this.checked)"{ad_checked}>
          <div class="row gap-12">
            <div class="admin-mark" style="width:40px;height:40px;border-radius:10px">{discord_icon}</div>
            <div><div style="font-weight:600">Discord</div>
            <div class="mono muted" style="font-size:11px">real-time threads</div></div>
          </div>
        </label>
        <label class="admin-card{ae}" style="min-height:auto;cursor:pointer">
          <input type="hidden" name="approval_email" value="false">
          <input type="checkbox" name="approval_email" value="true" style="display:none" onchange="this.closest('.admin-card').classList.toggle('selected',this.checked);this.closest('.admin-card').querySelector('.freq-row').style.display=this.checked?'flex':'none'"{ae_checked}>
          <div class="row gap-12">
            <div class="admin-mark" style="width:40px;height:40px;border-radius:10px">{mail_icon}</div>
            <div><div style="font-weight:600">Email</div>
            <div class="mono muted" style="font-size:11px">batched digest</div></div>
          </div>
          <div class="freq-row row gap-8" style="margin-top:12px;display:{ae_display}">
            <span class="mono muted" style="font-size:12px">Every</span>
            <select class="select" name="approval_freq" style="width:auto;font-size:13px;padding:6px 10px">{approval_freq_html}</select>
          </div>
        </label>
      </div>
    </div>

    <div class="card" style="padding:22px">
      <div class="eyebrow" style="margin-bottom:12px">Activity digest</div>
      <p class="muted" style="margin-bottom:14px;font-size:14px">A summary of messages handled, credits used, and anything that needs attention.</p>
      <div class="admin-grid">
        <label class="admin-card{dd}" style="min-height:auto;cursor:pointer">
          <input type="hidden" name="digest_discord" value="false">
          <input type="checkbox" name="digest_discord" value="true" style="display:none" onchange="this.closest('.admin-card').classList.toggle('selected',this.checked)"{dd_checked}>
          <div class="row gap-12">
            <div class="admin-mark" style="width:40px;height:40px;border-radius:10px">{discord_icon}</div>
            <div><div style="font-weight:600">Discord</div>
            <div class="mono muted" style="font-size:11px">channel post</div></div>
          </div>
        </label>
        <label class="admin-card{de}" style="min-height:auto;cursor:pointer">
          <input type="hidden" name="digest_email" value="false">
          <input type="checkbox" name="digest_email" value="true" style="display:none" onchange="this.closest('.admin-card').classList.toggle('selected',this.checked);this.closest('.admin-card').querySelector('.freq-row').style.display=this.checked?'flex':'none'"{de_checked}>
          <div class="row gap-12">
            <div class="admin-mark" style="width:40px;height:40px;border-radius:10px">{mail_icon}</div>
            <div><div style="font-weight:600">Email</div>
            <div class="mono muted" style="font-size:11px">periodic summary</div></div>
          </div>
          <div class="freq-row row gap-8" style="margin-top:12px;display:{de_display}">
            <span class="mono muted" style="font-size:12px">Every</span>
            <select class="select" name="digest_freq" style="width:auto;font-size:13px;padding:6px 10px">{digest_freq_html}</select>
          </div>
        </label>
      </div>
    </div>

    <div class="between" style="margin-top:36px">
      <button type="button" class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"channels"}}' hx-target="body" hx-swap="innerHTML">&larr; Back</button>
      <button id="notif-continue" type="submit" class="btn primary"{notif_disabled}>Continue &rarr;</button>
    </div>
  </form>
</section>
<script>
(function() {{
  var form = document.querySelector('form');
  var btn = document.getElementById('notif-continue');
  if (form && btn) {{
    function check() {{
      var boxes = form.querySelectorAll('input[type=checkbox]');
      var any = Array.from(boxes).some(function(b) {{ return b.checked; }});
      btn.disabled = !any;
    }}
    form.addEventListener('change', check);
  }}
}})();
</script>"##,
        base_url = base_url,
        discord_icon = channel_icon("discord"),
        mail_icon = channel_icon("mail"),
        ad = ad,
        ae = ae,
        dd = dd,
        de = de,
        ad_checked = if config.approval_discord {
            " checked"
        } else {
            ""
        },
        ae_checked = if config.approval_email {
            " checked"
        } else {
            ""
        },
        dd_checked = if config.digest_discord {
            " checked"
        } else {
            ""
        },
        de_checked = if config.digest_email { " checked" } else { "" },
        ae_display = if config.approval_email {
            "flex"
        } else {
            "none"
        },
        de_display = if config.digest_email { "flex" } else { "none" },
        approval_freq_html = approval_freq_html,
        digest_freq_html = digest_freq_html,
        notif_disabled = if config.approval_discord
            || config.approval_email
            || config.digest_discord
            || config.digest_email
        {
            ""
        } else {
            " disabled"
        },
    );

    wizard_shell("notifications", base_url, &content)
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
        r#"<div class="replies-row"><span class="muted" style="grid-column:1/-1;text-align:center">No canned replies yet. Add one below — or just let the AI handle everything.</span></div>"#
    } else {
        ""
    };

    let prompt = persona.to_system_prompt();

    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="mono muted">04 / 05</span><span class="eyebrow">Replies</span></div>
  <h2 class="display-md">How should your concierge respond?</h2>
  <p class="lead">Configure the AI voice for general replies, then add canned overrides for specific questions.</p>

  <div class="card" style="padding:22px;margin-bottom:16px">
    <div class="eyebrow" style="margin-bottom:12px">AI persona</div>
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px">
      <div>
        <label class="eyebrow" style="display:block;margin-bottom:6px">Type of business</label>
        <input class="input" name="biz_type" value="{biz_type}" placeholder="florist, hair salon, coffee shop...">
      </div>
      <div>
        <label class="eyebrow" style="display:block;margin-bottom:6px">City</label>
        <input class="input" name="city" value="{city}" placeholder="Chennai, Berlin...">
      </div>
      <div>
        <label class="eyebrow" style="display:block;margin-bottom:6px">Tone</label>
        <select class="select" name="tone">
          <option value="">Choose a tone...</option>
          <option value="warm &amp; chatty"{tone_wc}>warm &amp; chatty</option>
          <option value="concise &amp; professional"{tone_cp}>concise &amp; professional</option>
          <option value="playful with emoji"{tone_pe}>playful with emoji</option>
          <option value="old-school polite"{tone_op}>old-school polite</option>
        </select>
      </div>
      <div>
        <label class="eyebrow" style="display:block;margin-bottom:6px">Never do this</label>
        <select class="select" name="never">
          <option value="">Choose a boundary...</option>
          <option value="quote prices"{never_qp}>quote prices</option>
          <option value="promise dates"{never_pd}>promise dates</option>
          <option value="handle refunds"{never_hr}>handle refunds</option>
        </select>
      </div>
    </div>
    <div class="card" style="padding:14px;background:var(--ink);color:var(--cream);margin-top:16px;border-color:var(--ink);border-radius:var(--r-sm)">
      <div class="mono" style="font-size:10px;letter-spacing:.18em;color:var(--accent-soft);margin-bottom:6px">SYSTEM PROMPT</div>
      <pre class="mono" style="margin:0;white-space:pre-wrap;font-size:11px;color:var(--cream);line-height:1.5">{prompt}</pre>
    </div>
  </div>

  <form hx-post="{base_url}/admin/wizard/replies/save" hx-target="body" hx-swap="innerHTML">
    <input type="hidden" name="biz_type" value="{biz_type}">
    <input type="hidden" name="city" value="{city}">
    <input type="hidden" name="tone" value="{tone_raw}">
    <input type="hidden" name="never" value="{never_raw}">
    <div class="card replies-card">
      <div class="eyebrow" style="padding:14px 20px 0">Canned replies <span class="muted">(optional)</span></div>
      <p class="muted" style="padding:4px 20px 0;font-size:13px">These fire before the AI. Glob patterns work &mdash; <span class="mono">*</span> matches anything.</p>
      <div class="replies-head" style="margin-top:12px"><div>When message matches</div><div>Reply with</div><div></div></div>
      {rows}{empty}
      <div class="replies-add">
        <button type="button" class="btn ghost sm" hx-post="{base_url}/admin/wizard/replies/add" hx-target="body" hx-swap="innerHTML">+ Add reply</button>
      </div>
    </div>
    <div class="between" style="margin-top:32px">
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

    wizard_shell("replies", base_url, &content)
}

pub fn launch_html(
    email_subdomains: &[crate::types::EmailSubdomain],
    packs: &[crate::types::CreditPackRow],
    base_url: &str,
) -> String {
    let pending_emails: String = email_subdomains
        .iter()
        .filter(|s| s.subscription_id.is_none())
        .map(|s| {
            format!(
                r#"<div class="side-row" style="padding:10px 14px">
  <span>{mail_icon}</span>
  <div style="flex:1"><span class="mono" style="font-size:13px">{domain}</span></div>
  <form hx-post="{base_url}/admin/email/subdomains" hx-target="body" style="display:inline" hx-ext="json-enc">
    <input type="hidden" name="subdomain" value="{label}">
    <button type="submit" class="btn sm primary">Subscribe &#x20B9;199/mo</button>
  </form>
</div>"#,
                mail_icon = channel_icon("mail"),
                domain = html_escape(&s.domain),
                label = html_escape(&s.label),
                base_url = base_url,
            )
        })
        .collect();

    let email_section = if pending_emails.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="card" style="padding:22px;margin-bottom:16px">
  <div class="eyebrow" style="margin-bottom:8px">Email subdomains</div>
  <p class="muted" style="margin-bottom:12px;font-size:14px">Subscribe to activate your email addresses. You'll be redirected to Razorpay.</p>
  {pending_emails}
</div>"#
        )
    };

    let pack_buttons: String = packs
        .iter()
        .map(|p| {
            format!(
                r#"<form hx-post="{base_url}/admin/billing/checkout" hx-target="body" hx-swap="innerHTML" style="display:inline" hx-ext="json-enc">
  <input type="hidden" name="credits" value="{replies}">
  <button type="submit" class="card" style="padding:16px;text-align:center;min-width:140px;cursor:pointer;border:1px solid var(--hair)">
    <div class="stat-n serif">{replies}</div>
    <div class="mono muted" style="font-size:11px;margin-bottom:8px">replies</div>
    <div style="font-weight:600;margin-bottom:4px">&#x20B9;{inr} / ${usd}</div>
    <div class="mono muted" style="font-size:10px">never expire</div>
  </button>
</form>"#,
                base_url = base_url,
                replies = p.replies,
                inr = p.price_inr / 100,
                usd = p.price_usd / 100,
            )
        })
        .collect();

    let packs_section = if packs.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="card" style="padding:22px;margin-bottom:16px">
  <div class="eyebrow" style="margin-bottom:8px">Reply credit packs</div>
  <p class="muted" style="margin-bottom:12px;font-size:14px">You get 100 free replies every month. Buy more if you need them — purchased credits never expire.</p>
  <div class="row gap-12" style="flex-wrap:wrap;justify-content:center">{pack_buttons}</div>
</div>"#
        )
    };

    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="mono muted">06 / 06</span><span class="eyebrow">Ship it</span></div>
  <h2 class="display-md">You're all set.</h2>
  <p class="lead">Your concierge is configured and ready to handle messages. Here's a summary of what's next.</p>

  {email_section}
  {packs_section}

  <div class="card" style="padding:22px;border-color:var(--ok);background:linear-gradient(135deg,var(--paper),#E8F0DE)">
    <div class="row gap-12">
      <span class="dot ok"></span>
      <div>
        <div style="font-weight:600">Ready to go live</div>
        <p class="muted" style="margin:4px 0 0;font-size:14px">Hit finish to open your dashboard. Connect channels, set up email rules, and start receiving auto-replies.</p>
      </div>
    </div>
  </div>

  <div class="between" style="margin-top:36px">
    <button class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"replies"}}' hx-target="body" hx-swap="innerHTML">&larr; Back</button>
    <button class="btn primary" hx-post="{base_url}/admin/wizard/complete" hx-target="body">Finish setup &rarr;</button>
  </div>
</section>"#,
        email_section = email_section,
        packs_section = packs_section,
        base_url = base_url,
    );

    wizard_shell("launch", base_url, &content)
}

/// Public pricing page at /pricing
pub fn pricing_html(packs: &[crate::types::CreditPackRow]) -> String {
    let pack_rows: String = packs
        .iter()
        .map(|p| {
            let per_inr = p.price_inr as f64 / p.replies as f64 / 100.0;
            let per_usd = p.price_usd as f64 / p.replies as f64 / 100.0;
            format!(
                r##"<div class="rt-row" style="grid-template-columns:1fr 1fr 1fr 1fr">
  <div><strong>{name}</strong></div>
  <div>{replies}</div>
  <div><span class="p-inr">&#x20B9;{inr}</span><span class="p-usd" style="display:none">${usd}</span></div>
  <div><span class="p-inr">&#x20B9;{per_inr:.2}</span><span class="p-usd" style="display:none">${per_usd:.4}</span></div>
</div>"##,
                name = html_escape(&p.name),
                replies = p.replies,
                inr = p.price_inr / 100,
                usd = p.price_usd / 100,
                per_inr = per_inr,
                per_usd = per_usd,
            )
        })
        .collect();

    let content = format!(
        r##"<header class="site-header">
  {brand}
  <div class="row gap-12" style="margin-left:auto">
    <a href="/" class="btn ghost sm">&larr; Back</a>
    <a href="/auth/login" class="btn ghost sm">Sign in</a>
  </div>
</header>
<article class="legal">
  <div class="between">
    <h1>Simple pricing. Pay per reply.</h1>
    <div style="display:flex;border:1px solid var(--hair);border-radius:999px;overflow:hidden;font-size:13px">
      <button id="btn-inr" onclick="setCurrency('inr')" style="padding:6px 14px;border:none;background:var(--ink);color:var(--cream);cursor:pointer">&#x20B9; INR</button>
      <button id="btn-usd" onclick="setCurrency('usd')" style="padding:6px 14px;border:none;background:transparent;color:var(--ink);cursor:pointer">$ USD</button>
    </div>
  </div>
  <p class="muted">Every account gets 100 free replies each month. After that, buy a pack. Bigger packs cost less per reply.</p>

  <div class="card" style="padding:0;overflow:hidden;margin:24px 0">
    <div class="rt-head" style="grid-template-columns:1fr 1fr 1fr 1fr">
      <div>Pack</div><div>Replies</div><div>Price</div><div>Per reply</div>
    </div>
    <div class="rt-row" style="grid-template-columns:1fr 1fr 1fr 1fr;background:var(--cream-2)">
      <div><strong>Free</strong></div><div>100 / month</div><div>$0</div><div>$0</div>
    </div>
    {pack_rows}
  </div>

  <div class="card" style="padding:18px">
    <div class="eyebrow" style="margin-bottom:8px">What counts as a reply?</div>
    <p class="muted" style="margin:0">Every auto-reply sent by the concierge on WhatsApp, Instagram, or email uses one reply credit. Inbound messages, email forwarding, and Discord relay are free.</p>
  </div>

  <h2 style="margin-top:2rem">Email</h2>
  <p class="muted">Get a dedicated email address like <code>hello@yourname.cncg.email</code> with smart routing, forwarding, and AI replies.</p>

  <div class="card" style="padding:18px;margin:24px 0">
    <p style="margin:0"><strong><span class="p-inr">&#x20B9;199</span><span class="p-usd" style="display:none">$2</span> per subdomain per month.</strong></p>
    <p class="muted" style="margin:8px 0 0">Includes unlimited inbound email, routing rules, forwarding, and Discord relay. AI-generated replies use 1 reply credit each (from your pack above).</p>
  </div>
</article>
<script>
function setCurrency(c) {{
  var inr = document.querySelectorAll('.p-inr');
  var usd = document.querySelectorAll('.p-usd');
  var btnI = document.getElementById('btn-inr');
  var btnU = document.getElementById('btn-usd');
  if (c === 'usd') {{
    inr.forEach(function(el) {{ el.style.display = 'none'; }});
    usd.forEach(function(el) {{ el.style.display = ''; }});
    btnI.style.background = 'transparent'; btnI.style.color = 'var(--ink)';
    btnU.style.background = 'var(--ink)'; btnU.style.color = 'var(--cream)';
  }} else {{
    inr.forEach(function(el) {{ el.style.display = ''; }});
    usd.forEach(function(el) {{ el.style.display = 'none'; }});
    btnI.style.background = 'var(--ink)'; btnI.style.color = 'var(--cream)';
    btnU.style.background = 'transparent'; btnU.style.color = 'var(--ink)';
  }}
}}
</script>"##,
        brand = brand_mark(),
    );

    base_html_with_meta(
        "Pricing - Concierge",
        &content,
        &PageMeta {
            description: "Simple, prepaid pricing for Concierge. 100 free replies every month. Buy credit packs when you need more. Purchased credits never expire.",
            og_title: "Concierge Pricing",
            ..PageMeta::default()
        },
    )
}
