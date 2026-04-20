//! Base HTML wrappers — new cream/paper design system

use crate::helpers::html_escape;

use super::HASH;

pub const CSS: &str = r##"
:root {
  --cream: #F5EFE4; --cream-2: #EFE7D6; --paper: #FBF7EE;
  --ink: #1B1814; --ink-2: #3A332B; --muted: #7A6E5E;
  --hair: #D9CEB8; --hair-2: #C7BA9E;
  --accent: #E86A2C; --accent-2: #C8541C; --accent-soft: #F9D9C2;
  --sage: #6E8A5C; --plum: #6B3E5C; --sky: #3A6B8A; --butter: #E8C66A;
  --ok: #3E7F4A; --warn: #C46B1A;
  --r-sm: 8px; --r-md: 12px; --r-lg: 20px; --r-xl: 28px;
  --shadow-1: 0 1px 0 rgba(27,24,20,.04), 0 2px 6px rgba(27,24,20,.04);
  --shadow-2: 0 2px 0 rgba(27,24,20,.04), 0 12px 28px rgba(27,24,20,.08);
  --f-display: "Instrument Serif", Georgia, serif;
  --f-body: "Inter", system-ui, -apple-system, sans-serif;
  --f-mono: "JetBrains Mono", ui-monospace, SFMono-Regular, monospace;
}
* { box-sizing: border-box; }
html, body { margin:0; padding:0; background:var(--cream); color:var(--ink);
  font-family:var(--f-body); font-size:15px; line-height:1.5;
  -webkit-font-smoothing:antialiased; }
body::before { content:""; position:fixed; inset:0; pointer-events:none; z-index:1; opacity:.35;
  background-image:url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='240' height='240'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='2' stitchTiles='stitch'/><feColorMatrix values='0 0 0 0 0.1  0 0 0 0 0.09  0 0 0 0 0.07  0 0 0 .05 0'/></filter><rect width='100%25' height='100%25' filter='url(%23n)'/></svg>");
  mix-blend-mode:multiply; }
.app-root { position:relative; z-index:2; min-height:100vh; display:flex; flex-direction:column; }
.app-root > *:first-child { flex:1; }
.mono { font-family:var(--f-mono); letter-spacing:.02em; }
.serif { font-family:var(--f-display); }
.eyebrow { font-family:var(--f-mono); font-size:11px; letter-spacing:.18em; text-transform:uppercase; color:var(--muted); }
a { color:var(--accent); }
.btn { display:inline-flex; align-items:center; gap:8px; padding:10px 16px; border-radius:999px;
  border:1px solid var(--ink); background:var(--ink); color:var(--cream);
  font-family:var(--f-body); font-weight:500; font-size:14px; cursor:pointer; text-decoration:none;
  transition:transform .15s ease, background .15s ease, color .15s ease, border-color .15s ease; }
.btn:hover { transform:translateY(-1px); }
.btn:active { transform:translateY(0); }
.btn:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.btn.primary { background:var(--accent); color:#fff; border-color:var(--accent); }
.btn.primary:hover { background:var(--accent-2); border-color:var(--accent-2); }
.btn.ghost { background:transparent; color:var(--ink); border-color:var(--hair-2); }
.btn.ghost:hover { background:rgba(27,24,20,.05); }
.btn.sm { padding:6px 12px; font-size:13px; }
.btn.lg { padding:14px 22px; font-size:15px; }
.btn.icon { padding:8px; }
.btn:disabled { opacity:.45; cursor:not-allowed; transform:none; }
.card { background:var(--paper); border:1px solid var(--hair); border-radius:var(--r-lg); box-shadow:var(--shadow-1); }
.input, .textarea, .select { width:100%; padding:10px 12px; background:#fff;
  border:1px solid var(--hair-2); border-radius:var(--r-sm);
  font-family:var(--f-body); font-size:14px; color:var(--ink); outline:none;
  transition:border-color .15s ease, box-shadow .15s ease; }
.input:focus, .textarea:focus, .select:focus { border-color:var(--accent); box-shadow:0 0 0 4px var(--accent-soft); }
.textarea { resize:vertical; min-height:90px; font-family:var(--f-mono); }
.chip { display:inline-flex; align-items:center; gap:6px; padding:4px 10px;
  border-radius:999px; background:var(--cream-2); border:1px solid var(--hair); font-size:12px; color:var(--ink-2); }
.chip.ok { background:#E8F0DE; border-color:#B5C99B; color:#3E5A26; }
.chip.warn { background:#FCE8D5; border-color:#E9BC8D; color:#8A4B14; }
.dot { width:8px; height:8px; border-radius:50%; background:var(--muted); display:inline-block; }
.dot.ok { background:var(--ok); box-shadow:0 0 0 3px rgba(62,127,74,.18); }
.dot.warn { background:var(--warn); }
.hr { height:1px; background:var(--hair); border:0; margin:16px 0; }
.row { display:flex; align-items:center; }
.between { display:flex; align-items:center; justify-content:space-between; }
.stack { display:flex; flex-direction:column; }
.gap-4{gap:4px} .gap-6{gap:6px} .gap-8{gap:8px} .gap-12{gap:12px}
.gap-16{gap:16px} .gap-20{gap:20px} .gap-24{gap:24px}
.muted { color:var(--muted); }
.success { background:#E8F0DE; color:#3E5A26; padding:1rem; border-radius:var(--r-sm); margin-bottom:1rem; animation: fadeOut 3s ease 2s forwards; }
@keyframes fadeOut { to { opacity: 0; height: 0; padding: 0; margin: 0; overflow: hidden; } }
.error { background:#FCE8D5; color:#8A4B14; padding:1rem; border-radius:var(--r-sm); margin-bottom:1rem; }
.toggle { position:relative; display:inline-block; width:34px; height:20px; }
.toggle input { opacity:0; width:0; height:0; }
.toggle span { position:absolute; inset:0; cursor:pointer; background:var(--hair-2); border-radius:999px; transition:background .2s ease; }
.toggle span::before { content:''; position:absolute; left:2px; top:2px; width:16px; height:16px;
  background:#fff; border-radius:50%; transition:transform .2s ease; }
.toggle input:checked + span { background:var(--sage); }
.toggle input:checked + span::before { transform:translateX(14px); }
.indicator { display:none; width:10px; height:10px; border-radius:50%; background:currentColor; margin-left:6px; animation:pulse 1s infinite; }
.htmx-request .indicator, .indicator.htmx-request { display:inline-block; }
@keyframes fadeUp { from{opacity:0;transform:translateY(8px)} to{opacity:1;transform:none} }
@keyframes pulse { 0%,100%{opacity:1} 50%{opacity:.35} }
.fade-up { animation:fadeUp .4s ease both; }

/* Form elements */
.form-group { margin-bottom: 1.25rem; }
.form-group label {
  display: block;
  margin-bottom: 0.35rem;
  font-weight: 500;
  font-size: 0.9rem;
}
.form-group input,
.form-group select,
.form-group textarea {
  width: 100%;
  padding: 0.6rem 0.75rem;
  min-height: 44px;
  border: 1px solid var(--hair-2);
  border-radius: var(--r-sm);
  font-size: 1rem;
  font-family: inherit;
  background: #fff;
  color: var(--ink);
  transition: border-color 0.15s, box-shadow 0.15s;
}
.form-group input:focus-visible,
.form-group select:focus-visible,
.form-group textarea:focus-visible {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}
.form-group input[type="color"] {
  padding: 0.25rem;
  height: 44px;
  cursor: pointer;
}

/* Tables */
.table-wrap {
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
}
table { width: 100%; border-collapse: collapse; }
th, td {
  padding: 0.6rem 0.75rem;
  text-align: left;
  border-bottom: 1px solid var(--hair);
  white-space: nowrap;
}
th {
  background: var(--cream-2);
  font-weight: 600;
  font-size: 0.85rem;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--muted);
}
td { font-size: 0.9rem; }
code {
  background: var(--cream-2);
  padding: 0.15rem 0.4rem;
  border-radius: 3px;
  font-family: var(--f-mono);
  font-size: 0.85em;
}

/* App shell */
.app { min-height:100vh; }
.app-top { display:flex; align-items:center; gap:28px; padding:16px 28px;
  border-bottom:1px solid var(--hair); background:var(--paper); }
.app-nav { display:flex; gap:18px; flex:1; }
.app-nav a { text-decoration:none; color:var(--muted); font-size:14px; padding:8px 2px;
  border-bottom:2px solid transparent; }
.app-nav a.active { color:var(--ink); border-color:var(--accent); }
.avatar { width:30px; height:30px; border-radius:50%; background:var(--plum); color:#fff;
  display:flex; align-items:center; justify-content:center; font-size:13px; }
.brand { display:flex; align-items:center; gap:10px; font-size:20px; letter-spacing:-0.01em; }
.brand .serif { font-family:var(--f-display); font-size:24px; }

/* Dashboard grid */
.dash-grid { display:grid; grid-template-columns:300px 1fr; gap:24px; padding:24px 28px; }
@media(max-width:900px){.dash-grid{grid-template-columns:1fr}}
.side-list { display:flex; flex-direction:column; gap:10px; margin-top:10px; }
.side-row { display:flex; align-items:center; gap:10px; padding:8px 10px; border-radius:10px; background:var(--cream-2); }
.side-row > *:nth-child(2) { flex:1; font-size:13px; }
.stat-row { display:flex; align-items:baseline; gap:10px; padding:8px 0; border-bottom:1px dashed var(--hair); }
.stat-row:last-child { border-bottom:0; }
.stat-n { font-size:32px; color:var(--ink); }
.display-sm { font-family:var(--f-display); font-size:26px; letter-spacing:-0.01em; margin:0; }
.display-md { font-family:var(--f-display); font-size:clamp(34px,4.2vw,52px); line-height:1.05; letter-spacing:-0.02em; margin:8px 0 4px; }
.lead { color:var(--ink-2); max-width:640px; margin:0 0 22px; font-size:16px; }
.section-label { display:flex; align-items:center; gap:10px; margin-bottom:4px; }

/* Wizard */
.wizard { display:flex; flex-direction:column; }
.top { display:grid; grid-template-columns:auto 1fr auto; align-items:center; gap:28px;
  padding:18px 28px; border-bottom:1px solid var(--hair);
  background:rgba(251,247,238,0.88); backdrop-filter:blur(10px); position:sticky; top:0; z-index:10; }
.top-right { display:flex; align-items:center; gap:12px; }
.rail { display:flex; gap:8px; align-items:center; }
.rail .seg { height:4px; flex:1; background:var(--hair); border-radius:999px; overflow:hidden; }
.rail .seg .fill { display:block; height:100%; background:var(--ink); width:0; transition:width .4s ease; }
.rail .seg.done .fill { width:100%; }
.rail .seg.active .fill { width:55%; background:var(--accent); }
.rail-wrap { max-width:520px; width:100%; }
.rail-counter { text-align:right; margin-top:4px; font-size:10px; letter-spacing:.12em; }
.rail-labels { display:flex; justify-content:space-between; margin-top:6px;
  font-family:var(--f-mono); font-size:10px; color:var(--muted); letter-spacing:.12em; text-transform:uppercase; }
.rail-labels .active { color:var(--accent-2); }
.rail-labels .done { color:var(--ink-2); }
.page { max-width:1120px; margin:0 auto; padding:54px 32px 64px; }
.page.narrow { max-width:1040px; }

/* Channels grid */
.channels-grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(320px,1fr)); gap:16px; }
.channel { background:var(--paper); border:1px solid var(--hair); border-radius:var(--r-lg);
  padding:22px; position:relative; overflow:hidden;
  display:flex; flex-direction:column; gap:12px; min-height:220px;
  box-shadow:var(--shadow-1); transition:all .3s ease; }
.channel.is-connected { border-color:var(--sage); background:linear-gradient(180deg,var(--paper),#F4EBD9); }
.channel-head { display:flex; align-items:center; gap:10px; }
.channel-mark { width:40px; height:40px; border-radius:10px; background:var(--ink); color:var(--cream);
  display:flex; align-items:center; justify-content:center; }
.channel-name { font-weight:600; font-size:16px; }
.channel-body { flex:1; color:var(--ink-2); font-size:14px; }
.ribbon { position:absolute; top:14px; right:-38px; transform:rotate(34deg);
  background:var(--sage); color:#fff; font-family:var(--f-mono); font-size:10px;
  letter-spacing:.2em; padding:4px 40px; text-transform:uppercase;
  box-shadow:0 2px 8px rgba(110,138,92,.3); }
.note { margin-top:26px; padding:16px 18px; background:var(--cream-2);
  border:1px dashed var(--hair-2); border-radius:var(--r-md); display:flex; gap:12px; align-items:center; }
.note-icon { font-size:22px; }

/* Admin pick */
.admin-grid { display:grid; grid-template-columns:repeat(auto-fit,minmax(340px,1fr)); gap:18px; }
.admin-card { text-align:left; cursor:pointer; padding:22px; background:var(--paper);
  border:1px solid var(--hair); border-radius:var(--r-lg); min-height:260px;
  box-shadow:var(--shadow-1); font:inherit; color:inherit; transition:all .2s ease; }
.admin-card:hover { transform:translateY(-2px); box-shadow:var(--shadow-2); }
.admin-card.selected { border:2px solid var(--accent);
  background:linear-gradient(160deg,#FFF 0%,var(--accent-soft) 180%); }
.admin-mark { width:54px; height:54px; border-radius:14px; background:var(--ink); color:var(--cream);
  display:flex; align-items:center; justify-content:center; }
.mini-preview { margin-top:14px; border:1px solid var(--hair); border-radius:10px; overflow:hidden; background:#FFF; }
.mini-head { padding:8px 12px; background:var(--cream-2); border-bottom:1px solid var(--hair); font-size:11px; }
.mini-row { padding:8px 12px; display:flex; gap:10px; align-items:center; }
.mini-ava { width:22px; height:22px; border-radius:50%; flex-shrink:0; }
.mini-body { flex:1; font-size:13px; }

/* Rules table */
.rt-head, .rt-row { display:grid; grid-template-columns:1.4fr 1fr 1fr 1fr 0.6fr 0.5fr 80px;
  gap:12px; padding:12px 20px; align-items:center; border-bottom:1px solid var(--hair); font-size:13px; }
.rt-head { background:var(--cream-2); font-family:var(--f-mono); font-size:11px;
  letter-spacing:.18em; text-transform:uppercase; color:var(--muted); }
.rt-row.disabled { opacity:0.5; }
.rt-foot { padding:12px 20px; background:var(--cream-2); }

/* Welcome form */
.welcome { display:grid; grid-template-columns:minmax(0,1fr) 380px; gap:60px; align-items:center; }
@media(max-width:900px){.welcome{grid-template-columns:1fr;gap:32px}.postcard-card{transform:none}}
.welcome-form { display:flex; gap:10px; flex-wrap:wrap; margin-top:22px; }
.welcome-form .input { max-width:240px; }
.display { font-family:var(--f-display); font-size:clamp(44px,6vw,82px); line-height:1.02; letter-spacing:-0.02em; margin:0 0 16px; }
.display em { color:var(--accent); font-style:italic; }
.fineprint { margin-top:18px; color:var(--muted); font-size:12px; }

/* Postcard */
.postcard { position:relative; }
.postcard-card { position:relative; padding:22px; background:var(--paper);
  border:1px solid var(--hair); border-radius:var(--r-lg);
  box-shadow:var(--shadow-2); transform:rotate(1.5deg);
  font-family:var(--f-mono); font-size:12px; }
.postcard-head { display:flex; justify-content:space-between; align-items:center;
  padding-bottom:10px; border-bottom:1px dashed var(--hair-2); color:var(--muted); letter-spacing:.2em; }
.log-row { display:grid; grid-template-columns:120px 1fr; gap:8px; padding:6px 0;
  border-bottom:1px dashed var(--hair); }
.log-row:last-child { border-bottom:0; }
.log-a { color:var(--accent-2); }
.log-b { color:var(--ink-2); }
.stamp { position:absolute; top:-12px; right:8px; width:84px; height:84px;
  border:2px solid var(--accent); border-radius:50%;
  display:flex; align-items:center; justify-content:center;
  color:var(--accent); font-family:var(--f-mono); font-size:10px;
  text-align:center; transform:rotate(-12deg); background:rgba(232,106,44,.05);
  letter-spacing:.12em; line-height:1.2; }

/* Replies table */
.replies-card { padding:0; overflow:hidden; }
.replies-head, .replies-row { display:grid; grid-template-columns:1fr 2fr 80px; gap:12px;
  padding:14px 20px; align-items:center; border-bottom:1px solid var(--hair); }
.replies-head { background:var(--cream-2); font-family:var(--f-mono); font-size:11px;
  letter-spacing:.18em; color:var(--muted); text-transform:uppercase; }
.replies-row:last-of-type { border-bottom:0; }
.replies-add { padding:14px; background:var(--cream-2); }

/* Terminal */
.terminal { background:#0F0D0B; color:#D9D0BD; border-radius:16px; padding:20px;
  font-family:var(--f-mono); font-size:13px; line-height:1.7; min-height:380px;
  border:1px solid #2A2520; margin-top:8px; }
.term-chrome { display:flex; gap:10px; align-items:center; margin-bottom:14px;
  padding-bottom:10px; border-bottom:1px dashed #2A2520; }
.term-dot { display:inline-block; width:12px; height:12px; border-radius:50%; }
.term-title { margin-left:12px; color:#8A7E6B; }
.term-idle { color:#8A7E6B; }
.term-row { display:grid; grid-template-columns:70px 140px 1fr; gap:12px; }
.t-t { color:#8A7E6B; }
.t-tag { color:var(--accent); }

/* Banner */
.banner { display:flex; justify-content:space-between; align-items:center;
  padding:18px 28px; background:linear-gradient(90deg,var(--accent-soft),transparent 60%);
  border-bottom:1px solid var(--hair); }

/* Legal pages */
.legal { max-width:720px; margin:0 auto; padding:48px 28px 64px; }
.legal h1 { font-family:var(--f-display); font-size:clamp(32px,5vw,48px); letter-spacing:-0.02em; margin:0 0 4px; }
.legal h2 { font-size:1.1rem; margin:2rem 0 .5rem; color:var(--ink-2); }
.legal p, .legal ul { margin-bottom:1rem; color:var(--ink-2); line-height:1.7; }
.legal ul { padding-left:1.5rem; }
@media(max-width:600px){ .legal { padding:28px 16px 40px; } }

/* HTMX loading state */
.htmx-request .btn { opacity: 0.6; pointer-events: none; }

/* Mobile */
@media(max-width:600px){
  .page { padding:24px 16px 40px; }
  .display { font-size:clamp(32px,8vw,44px); margin:0 0 12px; }
  .display br { display:none; }
  .display-md { font-size:clamp(26px,6vw,34px); }
  .lead { font-size:15px; margin:0 0 16px; }
  .welcome-form { flex-direction:column; }
  .welcome-form .input { max-width:100%; }
  .welcome-form .btn { width:100%; justify-content:center; }
  .fineprint { display:flex; flex-direction:column; gap:4px; }
  .top { padding:12px 16px; gap:12px; }
  .top .rail-wrap { visibility:hidden; height:0; overflow:hidden; }
  .top-right { gap:8px; }
  .top-right .mono { display:none; }
  .app-top { padding:12px 16px; gap:12px; flex-wrap:wrap; }
  .app-nav { gap:10px; overflow-x:auto; -webkit-overflow-scrolling:touch; flex-wrap:nowrap; white-space:nowrap; }
  .app-nav a { font-size:13px; flex-shrink:0; }
  .channels-grid { grid-template-columns:1fr; }
  .admin-grid { grid-template-columns:1fr; }
  .admin-card { min-height:auto; }
  .between { flex-wrap:wrap; gap:12px; }
  .dash-grid { padding:16px; }
  .rt-head, .rt-row { grid-template-columns:1fr; gap:4px; padding:10px 14px; }
  .replies-head, .replies-row { grid-template-columns:1fr; gap:4px; padding:10px 14px; }
  .banner { padding:14px 16px; flex-direction:column; gap:10px; text-align:center; }
  .terminal { padding:14px; font-size:12px; min-height:auto; }
  .term-row { grid-template-columns:1fr; gap:2px; }
}
@media(max-width:480px){
  .rail-labels span:not(.active):not(.done) { display:none; }
}

/* Scrollbars */
::-webkit-scrollbar { width:10px; height:10px; }
::-webkit-scrollbar-thumb { background:var(--hair-2); border-radius:999px; }
::-webkit-scrollbar-track { background:transparent; }
"##;

/// Logo inline SVG for use in templates.
pub const LOGO_INLINE: &str = r##"<svg width="30" height="30" viewBox="0 0 100 100" style="display:block"><circle cx="50" cy="50" r="48" fill="var(--accent)"/><path d="M28 40a22 22 0 0 1 44 0v20a22 22 0 0 1-44 0" stroke="#FBF7EE" stroke-width="6" fill="none" stroke-linecap="round"/><circle cx="50" cy="52" r="5.5" fill="#FBF7EE"/></svg>"##;

pub fn brand_mark() -> String {
    format!(
        r#"<a href="/" class="brand" style="text-decoration:none;color:inherit">{}<span class="serif">Concierge</span></a>"#,
        LOGO_INLINE
    )
}

/// Shared footer for all pages.
pub fn footer() -> &'static str {
    r##"<footer style="text-align:center;padding:2rem;color:var(--muted);font-size:13px;border-top:1px solid var(--hair)">
  <a href="https://github.com/ananthb/concierge-worker" style="color:var(--muted)">Open-source</a> &middot;
  Licensed under <a href="https://www.gnu.org/licenses/agpl-3.0.html" style="color:var(--muted)">AGPL-3.0</a> &middot;
  <a href="https://ananthb.github.io/concierge-worker/" style="color:var(--muted)">Docs</a> &middot;
  <a href="/terms" style="color:var(--muted)">Terms</a> &middot;
  <a href="/privacy" style="color:var(--muted)">Privacy</a>
</footer>"##
}

/// OpenGraph / meta description tags for a page.
pub struct PageMeta {
    pub description: &'static str,
    pub og_title: &'static str,
    pub og_type: &'static str, // "website", "article", etc.
}

impl Default for PageMeta {
    fn default() -> Self {
        Self {
            description: "Automated customer messaging for small businesses. Auto-reply across WhatsApp, Instagram DMs, and email. 100 replies free every month.",
            og_title: "Concierge",
            og_type: "website",
        }
    }
}

/// Base HTML wrapper for all pages.
pub fn base_html(title: &str, content: &str) -> String {
    base_html_with_meta(title, content, &PageMeta::default())
}

/// Base HTML wrapper with custom meta tags.
pub fn base_html_with_meta(title: &str, content: &str, meta: &PageMeta) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="{description}">
<meta property="og:title" content="{og_title}">
<meta property="og:description" content="{description}">
<meta property="og:type" content="{og_type}">
<meta property="og:image" content="https://concierge.calculon.tech/logo-512.png">
<meta property="og:site_name" content="Concierge">
<meta name="twitter:card" content="summary">
<meta name="twitter:title" content="{og_title}">
<meta name="twitter:description" content="{description}">
<link rel="icon" href="/logo.svg" type="image/svg+xml">
<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32.png">
<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16.png">
<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">
<link rel="manifest" href="/site.webmanifest">
<meta name="theme-color" content="#E86A2C">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@0;1&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
<script src="https://unpkg.com/htmx.org@2.0.8/dist/htmx.min.js" integrity="sha384-/TgkGk7p307TH7EXJDuUlgG3Ce1UVolAOFopFekQkkXihi5u/6OCvVKyz1W+idaz" crossorigin="anonymous"></script>
<script src="https://unpkg.com/htmx.org@2.0.8/dist/ext/json-enc.js"></script>
<style>{css}</style>
</head>
<body>
<div class="app-root">{content}{footer}</div>
<script>
function copyUrl(btn, url) {{
    navigator.clipboard.writeText(url).then(function() {{
        btn.textContent = 'Copied!';
        setTimeout(function() {{
            btn.textContent = 'Copy';
        }}, 2000);
    }});
}}
document.addEventListener("htmx:responseError", function() {{
  var t = document.getElementById("toast");
  if (t) {{ t.innerHTML = '<div class="error">Something went wrong. Please try again.</div>'; }}
}});
// Send CSRF token with all HTMX requests
document.addEventListener("htmx:configRequest", function(e) {{
  var csrf = document.cookie.split(';').map(function(c){{return c.trim()}}).find(function(c){{return c.startsWith('csrf=')}});
  if (csrf) {{ e.detail.headers['X-CSRF-Token'] = csrf.substring(5); }}
}});
</script>
</body>
</html>"##,
        title = html_escape(title),
        description = html_escape(meta.description),
        og_title = html_escape(meta.og_title),
        og_type = meta.og_type,
        content = content,
        css = CSS,
        footer = footer(),
    )
}

/// Wrap content in the app shell (top nav + main area).
pub fn app_shell(content: &str, active_nav: &str, base_url: &str) -> String {
    let nav_items = [
        ("Overview", "/admin"),
        ("Channels", "/admin/whatsapp"),
        ("Email Routing", "/admin/email"),
        ("Billing", "/admin/billing"),
        ("Settings", "/admin/settings"),
    ];

    let nav: String = nav_items
        .iter()
        .map(|(label, href)| {
            let class = if *label == active_nav { " active" } else { "" };
            format!(r#"<a class="{class}" href="{base_url}{href}">{label}</a>"#)
        })
        .collect();

    format!(
        r#"<div class="app">
  <header class="app-top">
    {brand}
    <nav class="app-nav">{nav}</nav>
    <div class="row gap-12">
      <span class="chip ok">live</span>
      <a href="{base_url}/auth/logout" class="avatar">X</a>
    </div>
  </header>
  {content}
</div>"#,
        brand = brand_mark(),
        nav = nav,
        base_url = base_url,
        content = content,
    )
}
