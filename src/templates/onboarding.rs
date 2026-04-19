//! Onboarding wizard templates

use crate::helpers::html_escape;
use crate::types::*;

use super::base::{base_html, base_html_with_meta, brand_mark, PageMeta, LOGO_INLINE};
use super::HASH;

const STEPS: &[(&str, &str)] = &[
    ("welcome", "Hey"),
    ("channels", "Plug in"),
    ("notifications", "Ping me"),
    ("persona", "Your voice"),
    ("replies", "Quick replies"),
    ("launch", "Go live"),
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
        r#"<div class="wizard">
  <header class="top">
    {brand}
    <div class="rail-wrap">{rail}<div class="rail-counter mono muted">{step_num}/{total}</div></div>
    <div class="top-right">
      <a href="/auth/login" class="btn ghost sm">Sign in</a>
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

pub fn welcome_html(base_url: &str) -> String {
    let content = format!(
        r#"<section class="page welcome">
  <div class="welcome-left">
    <div class="eyebrow">// automated customer engagement</div>
    <h1 class="display">Hello. I'll be answering <br>every <em>DM, WhatsApp &amp; email</em> <br>so you don't have to.</h1>
    <p class="lead">Connect your channels, set a tone, and your concierge handles the rest. Auto-replies across WhatsApp, Instagram, and email. 100 replies free every month.</p>
    <div id="welcome-new">
      <form class="welcome-form" action="/auth/login" method="get">
        <input class="input" name="biz" id="biz-input" placeholder="Business name" required>
        <button class="btn primary lg" type="submit">Start setup &rarr;</button>
      </form>
      <div class="mono fineprint">
        &#x25E6; <a href="/auth/login" style="color:var(--accent)">already have an account? sign in</a>
        &nbsp; &#x25E6; <a href="/pricing" style="color:var(--muted)">pricing</a>
        &nbsp; &#x25E6; <a href="https://github.com/ananthb/concierge-worker" style="color:var(--muted)">open-source</a>
      </div>
    </div>
    <div id="welcome-back" style="display:none">
      <p class="lead" id="wb-greeting" style="font-size:20px;margin-bottom:18px"></p>
      <a href="/auth/login" class="btn primary lg">Sign in &rarr;</a>
      <div class="mono fineprint" style="margin-top:18px">
        &#x25E6; <a href="/pricing" style="color:var(--muted)">pricing</a>
        &nbsp; &#x25E6; <a href="https://github.com/ananthb/concierge-worker" style="color:var(--muted)">open-source</a>
      </div>
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
</section>
<script>
(function() {{
  function getCookie(name) {{
    var c = document.cookie.split(';').map(function(s){{return s.trim()}}).find(function(s){{return s.startsWith(name+'=')}});
    return c ? decodeURIComponent(c.substring(name.length+1)) : null;
  }}

  var provider = getCookie('last_provider');
  var biz = getCookie('onboarding_biz');

  if (provider) {{
    // Returning user: show sign-in view
    document.getElementById('welcome-new').style.display = 'none';
    document.getElementById('welcome-back').style.display = 'block';
    document.getElementById('wb-greeting').textContent = biz ? 'Hi, ' + biz + '.' : 'Welcome back.';
    // Fill all rail segments and highlight "Go live"
    document.querySelectorAll('.rail .seg').forEach(function(seg) {{
      seg.classList.remove('active');
      seg.classList.add('done');
    }});
    var labels = document.querySelectorAll('.rail-labels span');
    labels.forEach(function(l) {{ l.classList.add('done'); l.classList.remove('active'); }});
    if (labels.length > 0) {{ labels[labels.length-1].classList.add('active'); labels[labels.length-1].classList.remove('done'); }}
  }} else {{
    // New user: animate the first rail segment when business name is typed
    var input = document.getElementById('biz-input');
    var firstFill = document.querySelector('.rail .seg.active .fill');
    if (input && firstFill) {{
      input.addEventListener('input', function() {{
        firstFill.style.width = input.value.trim().length > 0 ? '90%' : '55%';
      }});
    }}
  }}
}})();
</script>"#,
        hash = HASH,
    );

    wizard_shell("welcome", base_url, &content)
}

pub fn connect_html(ig_connected: bool, wa_connected: bool, base_url: &str) -> String {
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

    let ready = ig_connected || wa_connected;
    let disabled = if ready { "" } else { " disabled" };
    let hint = if ready {
        ""
    } else {
        r#"<span class="mono muted">connect at least one to continue</span>"#
    };

    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="mono muted">01 / 04</span><span class="eyebrow">Connect your channels</span></div>
  <h2 class="display-md">Where do your customers already talk to you?</h2>
  <p class="lead">We listen on these. Skip anything you don't use - you can wire them up later from the dashboard.</p>
  <div class="channels-grid">{ig_card}{wa_card}</div>
  <div class="note">
    <div class="note-icon">&#x2709;</div>
    <div style="flex:1">
      <div style="font-weight:600">Got a catch-all email? Add it from the dashboard.</div>
      <div class="mono muted" style="font-size:12px">Email routing comes after the wizard, where the fun rule-builder lives.</div>
    </div>
  </div>
  <div class="between" style="margin-top:36px">
    <button class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"welcome"}}' hx-target="body" hx-swap="innerHTML">&larr; Back</button>
    <div class="row gap-12">
      {hint}
      <button class="btn primary"{disabled} hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"notifications"}}' hx-target="body" hx-swap="innerHTML">Continue &rarr;</button>
    </div>
  </div>
</section>"#,
        ig_card = ig_card,
        wa_card = wa_card,
        base_url = base_url,
        hint = hint,
        disabled = disabled,
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

pub fn admin_pick_html(selected: &str, base_url: &str) -> String {
    let discord_sel = if selected == "discord" {
        " selected"
    } else {
        ""
    };
    let email_sel = if selected == "email" { " selected" } else { "" };
    let disabled = if selected.is_empty() { " disabled" } else { "" };

    let content = format!(
        r##"<section class="page narrow">
  <div class="section-label"><span class="mono muted">02 / 04</span><span class="eyebrow">Admin channel</span></div>
  <h2 class="display-md">Where should I ping you when things need a human?</h2>
  <p class="lead">For handoffs, alerts, and the daily digest. Pick one - you can change it anytime.</p>
  <div class="admin-grid">
    <button class="admin-card{discord_sel}" hx-post="{base_url}/admin/wizard/admin-pick" hx-vals='{{"v":"discord"}}' hx-target="body" hx-swap="innerHTML">
      <div class="between"><span class="chip ok">Recommended</span></div>
      <div class="row gap-12" style="margin-top:14px">
        <div class="admin-mark">{discord_icon}</div>
        <div><div class="serif" style="font-size:28px;line-height:1">Discord</div>
        <div class="mono muted" style="font-size:11px">threaded &middot; real-time</div></div>
      </div>
      <p style="margin-top:14px;color:var(--ink-2)">Every conversation lands as a thread. Reply in Discord &rarr; flows back to the customer automatically.</p>
    </button>
    <button class="admin-card{email_sel}" hx-post="{base_url}/admin/wizard/admin-pick" hx-vals='{{"v":"email"}}' hx-target="body" hx-swap="innerHTML">
      <div class="between"><span class="chip">Classic</span></div>
      <div class="row gap-12" style="margin-top:14px">
        <div class="admin-mark">{mail_icon}</div>
        <div><div class="serif" style="font-size:28px;line-height:1">Email</div>
        <div class="mono muted" style="font-size:11px">forwarded &middot; digested &middot; familiar</div></div>
      </div>
      <p style="margin-top:14px;color:var(--ink-2)">Forwarded to the address you already check. Daily digest at 8am so your inbox doesn't explode.</p>
    </button>
  </div>
  <div class="between" style="margin-top:36px">
    <button class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"channels"}}' hx-target="body" hx-swap="innerHTML">&larr; Back</button>
    <button class="btn primary"{disabled} hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"persona"}}' hx-target="body" hx-swap="innerHTML">Continue &rarr;</button>
  </div>
</section>"##,
        base_url = base_url,
        discord_sel = discord_sel,
        email_sel = email_sel,
        disabled = disabled,
        discord_icon = channel_icon("discord"),
        mail_icon = channel_icon("mail"),
    );

    wizard_shell("notifications", base_url, &content)
}

pub fn persona_html(persona: &PersonaConfig, base_url: &str) -> String {
    let prompt = persona.to_system_prompt();
    let disabled = if persona.biz_type.is_empty() {
        " disabled"
    } else {
        ""
    };

    let content = format!(
        r#"<section class="page">
  <div class="section-label"><span class="mono muted">03 / 04</span><span class="eyebrow">Your auto-reply voice</span></div>
  <h2 class="display-md">Configure your AI persona.</h2>
  <p class="lead">Tell the AI about your business so it can reply in your voice.</p>
  <div class="card" style="padding:22px">
    <form hx-post="{base_url}/admin/wizard/persona" hx-target="body" hx-swap="innerHTML">
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px">
        <div>
          <label class="eyebrow" style="display:block;margin-bottom:6px">Type of business</label>
          <input class="input" name="biz_type" value="{biz_type}" placeholder="florist, hair salon, coffee shop...">
        </div>
        <div>
          <label class="eyebrow" style="display:block;margin-bottom:6px">City</label>
          <input class="input" name="city" value="{city}" placeholder="Sydney, Berlin...">
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
      <button class="btn primary" type="submit" style="margin-top:16px">Save persona</button>
    </form>
  </div>
  <div class="card" style="padding:18px;background:var(--ink);color:var(--cream);margin-top:16px;border-color:var(--ink)">
    <div class="mono" style="font-size:11px;letter-spacing:.2em;color:var(--accent-soft);margin-bottom:10px">COMPILED SYSTEM PROMPT</div>
    <pre class="mono" style="margin:0;white-space:pre-wrap;font-size:12px;color:var(--cream);line-height:1.6">{prompt}</pre>
  </div>
  <div class="between" style="margin-top:32px">
    <button class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"notifications"}}' hx-target="body" hx-swap="innerHTML" hx-include="[name=biz_type],[name=city],[name=tone],[name=never]">&larr; Back</button>
    <button class="btn primary"{disabled} hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"replies"}}' hx-target="body" hx-swap="innerHTML" hx-include="[name=biz_type],[name=city],[name=tone],[name=never]">Continue &rarr;</button>
  </div>
</section>"#,
        base_url = base_url,
        biz_type = html_escape(&persona.biz_type),
        city = html_escape(&persona.city),
        prompt = html_escape(&prompt),
        disabled = disabled,
        tone_wc = sel_attr(&persona.tone, "warm & chatty"),
        tone_cp = sel_attr(&persona.tone, "concise & professional"),
        tone_pe = sel_attr(&persona.tone, "playful with emoji"),
        tone_op = sel_attr(&persona.tone, "old-school polite"),
        never_qp = sel_attr(&persona.never, "quote prices"),
        never_pd = sel_attr(&persona.never, "promise dates"),
        never_hr = sel_attr(&persona.never, "handle refunds"),
    );

    wizard_shell("persona", base_url, &content)
}

fn sel_attr(current: &str, value: &str) -> &'static str {
    if current == value {
        " selected"
    } else {
        ""
    }
}

pub fn replies_html(canned: &[CannedReply], base_url: &str) -> String {
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
        r#"<div class="replies-row"><span class="muted" style="grid-column:1/-1;text-align:center">No canned replies yet. Add one below.</span></div>"#
    } else {
        ""
    };

    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="mono muted">04 / 04</span><span class="eyebrow">Canned replies (optional)</span></div>
  <h2 class="display-md">When they ask X, always say Y.</h2>
  <p class="lead">Static replies fire before the AI, so common questions get instant, perfect answers. Glob patterns work - <span class="mono">*</span> matches anything.</p>
  <form hx-post="{base_url}/admin/wizard/replies/save" hx-target="body" hx-swap="innerHTML">
    <div class="card replies-card">
      <div class="replies-head"><div>When message matches</div><div>Reply with</div><div></div></div>
      {rows}{empty}
      <div class="replies-add">
        <button type="button" class="btn ghost sm" hx-post="{base_url}/admin/wizard/replies/add" hx-target="body" hx-swap="innerHTML">+ Add reply</button>
      </div>
    </div>
    <div class="between" style="margin-top:32px">
      <button type="button" class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"persona"}}' hx-target="body" hx-swap="innerHTML">&larr; Back</button>
      <button type="submit" class="btn primary">Run a test message &rarr;</button>
    </div>
  </form>
</section>"#,
        base_url = base_url,
        rows = rows,
        empty = empty,
    );

    wizard_shell("replies", base_url, &content)
}

pub fn test_html(base_url: &str) -> String {
    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="eyebrow">Final check</span></div>
  <h2 class="display-md">Let's fake a customer and make sure I work.</h2>
  <p class="lead">We'll push a test message through the pipeline, route it, draft a reply, and confirm your admin channel receives the handoff.</p>
  <div class="terminal">
    <div class="term-chrome">
      <span class="term-dot" style="background:#C46B1A"></span>
      <span class="term-dot" style="background:#5C6B3A"></span>
      <span class="term-dot" style="background:#6B5C3A"></span>
      <span class="term-title mono">concierge://test-run</span>
      <span id="test-status" class="mono" style="margin-left:auto;color:#8A7E6B">&#x25CB; ready</span>
    </div>
    <div id="term-body">
      <div class="term-idle">&gt; Setup complete. Hit "Finish" to go to the dashboard.</div>
    </div>
  </div>
  <div class="between" style="margin-top:24px">
    <button class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"replies"}}' hx-target="body" hx-swap="innerHTML">&larr; Back</button>
    <a href="{base_url}/admin" class="btn primary">Open dashboard &rarr;</a>
  </div>
</section>"#,
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
  <div>&#x20B9;{inr} / ${usd}</div>
  <div>&#x20B9;{per_inr:.2} / ${per_usd:.4}</div>
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
        r##"<div style="max-width:720px;margin:0 auto;padding:54px 32px 64px">
  <div style="text-align:center;margin-bottom:32px">
    {logo}
    <div class="serif" style="font-size:28px;margin-top:8px">Concierge</div>
  </div>
  <h1 class="display-md" style="text-align:center">Simple pricing. Pay per reply.</h1>
  <p class="lead" style="text-align:center;margin:0 auto 32px">Every account gets 100 free replies each month. After that, buy a pack. Bigger packs cost less per reply.</p>

  <div class="card" style="padding:0;overflow:hidden;margin-bottom:24px">
    <div class="rt-head" style="grid-template-columns:1fr 1fr 1fr 1fr">
      <div>Pack</div><div>Replies</div><div>Price</div><div>Per reply</div>
    </div>
    <div class="rt-row" style="grid-template-columns:1fr 1fr 1fr 1fr;background:var(--cream-2)">
      <div><strong>Free</strong></div><div>100 / month</div><div>$0</div><div>$0</div>
    </div>
    {pack_rows}
  </div>

  <div class="card" style="padding:18px;margin-bottom:24px">
    <div class="eyebrow" style="margin-bottom:8px">What counts as a reply?</div>
    <p class="muted" style="margin:0">Every auto-reply sent by the concierge on WhatsApp, Instagram, or email uses one reply credit. Inbound messages, email forwarding, and Discord relay are free.</p>
  </div>

  <div style="text-align:center">
    <a href="/" class="btn ghost">&larr; Back to home</a>
  </div>
</div>"##,
        logo = LOGO_INLINE,
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
