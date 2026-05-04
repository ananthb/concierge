//! Onboarding wizard templates

use crate::helpers::html_escape;
use crate::i18n::t;
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

/// Step-id keys (English, used as routing slugs and as the stable `active`
/// argument for `wizard_shell`). The user-visible label comes from FTL.
const STEPS: &[(&str, &str)] = &[
    ("basics", "wizard-step-basics-label"),
    ("channels", "wizard-step-channels-label"),
    ("notifications", "wizard-step-notifications-label"),
    ("replies", "wizard-step-replies-label"),
    ("launch", "wizard-step-launch-label"),
];

fn rail_html(current: &str, progress_expr: &str, locale: &crate::locale::Locale) -> String {
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
        .map(|(i, (_, key))| {
            let class = if i < idx {
                "done"
            } else if i == idx {
                "active"
            } else {
                ""
            };
            let label = t(locale, key);
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
    locale: &crate::locale::Locale,
) -> String {
    let idx = STEPS.iter().position(|(id, _)| *id == step).unwrap_or(0);

    let inner = format!(
        r#"<div class="wizard" x-data="{x_data}" hx-ext="json-enc">
  <header class="top">
    {brand}
    <div class="rail-wrap">{rail}<div class="rail-counter mono muted">{step_num}/{total}</div></div>
    <div class="top-right">
      <a href="/auth/logout" class="btn ghost sm">{signout}</a>
    </div>
  </header>
  <main>{content}</main>
</div>"#,
        brand = brand_mark(),
        rail = rail_html(step, progress_expr, locale),
        step_num = idx + 1,
        total = STEPS.len(),
        x_data = x_data,
        content = content,
        signout = t(locale, "wizard-signout"),
    );

    base_html(&t(locale, "wizard-title"), &inner, locale)
}

pub fn welcome_html(_base_url: &str, locale: &crate::locale::Locale, demo_enabled: bool) -> String {
    use crate::i18n::t;
    let header = super::base::public_nav_html("", locale);

    // Headline variants. The first is rendered statically so the page
    // looks correct on first paint; the rest get rotated in by the
    // typewriter script below.
    let variants: [String; 5] = [
        t(locale, "welcome-headline"),
        t(locale, "welcome-headline-2"),
        t(locale, "welcome-headline-3"),
        t(locale, "welcome-headline-4"),
        t(locale, "welcome-headline-5"),
    ];
    let variants_json = variants
        .iter()
        .map(|s| js_string_for_html(s))
        .collect::<Vec<_>>()
        .join(",");
    let rotator = HERO_ROTATOR_JS.replace("__VARIANTS__", &format!("[{variants_json}]"));

    let chat_error = t(locale, "demo-chat-error");
    let chat_rate_limited = t(locale, "demo-chat-rate-limited");
    // Personas are no longer embedded in the page; the chat factory
    // fetches `/demo/personas` on first open. That endpoint reads the
    // D1 catalog and returns only Approved rows.
    let chat_script = HERO_CHAT_JS
        .replace("__PREAMBLE__", &js_string_for_html(crate::prompt::PREAMBLE))
        .replace(
            "__POSTAMBLE__",
            &js_string_for_html(crate::prompt::POSTAMBLE),
        )
        .replace("__ERROR__", &js_string_for_html(&chat_error))
        .replace("__RATE_LIMITED__", &js_string_for_html(&chat_rate_limited));

    let chat_hint = html_escape(&t(locale, "demo-chat-hint"));
    let chat_title = html_escape(&t(locale, "demo-chat-title"));
    // Subtitle has two variants: business-roleplay (default) vs.
    // Concierge-direct. The Alpine factory picks per persona; we just
    // ship both strings into the chat module so it can swap them.
    // These end up inside Alpine `'…'` JS literals in an HTML attribute,
    // so they need `js_attr_escape` (which renders `'` as `\'`), not
    // `html_escape` (which would only entity-encode for HTML and leave
    // a bare apostrophe to terminate the JS string).
    let chat_subtitle = js_attr_escape(&t(locale, "demo-chat-subtitle"));
    let chat_subtitle_concierge = js_attr_escape(&t(locale, "demo-chat-subtitle-concierge"));
    let chat_persona_label = html_escape(&t(locale, "demo-chat-persona-label"));
    let chat_roleplay_prefix = html_escape(&t(locale, "demo-chat-roleplay-prefix"));
    let chat_roleplay_suffix = html_escape(&t(locale, "demo-chat-roleplay-suffix"));
    let chat_channels_note = html_escape(&t(locale, "demo-chat-channels-note"));
    let chat_lbl_hours = html_escape(&t(locale, "demo-chat-business-hours"));
    let chat_lbl_city = html_escape(&t(locale, "demo-chat-business-city"));
    let chat_lbl_type = html_escape(&t(locale, "demo-chat-business-type"));
    let chat_lbl_goal = html_escape(&t(locale, "demo-chat-business-goal"));
    let chat_handoff_chip = html_escape(&t(locale, "demo-chat-handoff-chip"));
    let chat_view_prompt = html_escape(&t(locale, "demo-chat-view-prompt"));
    let chat_hide_prompt = html_escape(&t(locale, "demo-chat-hide-prompt"));
    let chat_prompt_heading = html_escape(&t(locale, "demo-chat-prompt-heading"));
    let chat_envelope_note = html_escape(&t(locale, "demo-chat-envelope-note"));
    // Same as the subtitles: these go inside an Alpine `'…'` literal in
    // a `:placeholder` attribute, so escape for JS-in-attr.
    let chat_placeholder = js_attr_escape(&t(locale, "demo-chat-placeholder"));
    let chat_placeholder_prefix =
        js_attr_escape(&t(locale, "demo-chat-placeholder-customer-prefix"));
    let chat_placeholder_suffix =
        js_attr_escape(&t(locale, "demo-chat-placeholder-customer-suffix"));
    let chat_send = html_escape(&t(locale, "demo-chat-send"));
    let chat_close = html_escape(&t(locale, "demo-chat-close"));
    let chat_thinking = html_escape(&t(locale, "demo-chat-thinking"));
    let chat_cta_heading = html_escape(&t(locale, "demo-chat-cta-heading"));
    let chat_cta_body = html_escape(&t(locale, "demo-chat-cta-body"));
    let chat_cta_button = html_escape(&t(locale, "demo-chat-cta-button"));

    // The hero headline is a click target into the demo modal when the
    // demo is enabled. When the operator has disabled the demo, drop
    // the click handlers, the `hero-clickable` class (which carries the
    // hover effect), and the floating chat hint.
    let initial_headline = &variants[0];
    let (hero_headline, hero_hint) = if demo_enabled {
        (
            format!(
                r#"<h1 class="display hero-clickable" id="hero-headline"
          tabindex="0" aria-describedby="demo-chat-hint-text"
          @click="open = true"
          @keydown.enter.prevent="open = true"
          @keydown.space.prevent="open = true">{headline}<span class="hero-caret" aria-hidden="true"></span></h1>"#,
                headline = initial_headline,
            ),
            format!(
                r#"<button type="button" id="demo-chat-hint-text" class="hero-hint" @click="open = true">
        <span class="hero-hint-arrow" aria-hidden="true">↑</span>
        <span class="hero-hint-text">{chat_hint}</span>
      </button>"#,
                chat_hint = chat_hint,
            ),
        )
    } else {
        (
            format!(
                r#"<h1 class="display" id="hero-headline">{headline}<span class="hero-caret" aria-hidden="true"></span></h1>"#,
                headline = initial_headline,
            ),
            String::new(),
        )
    };

    let content = format!(
        r#"{header}
<div x-data="conciergeChat()" x-effect="window.__heroPaused = open">
<section class="page welcome">
  <div class="welcome-left">
    <div class="eyebrow">{eyebrow}</div>
    <div class="hero-hint-anchor">
      {hero_headline}
      {hero_hint}
    </div>
    <p class="lead">{lead}</p>
    <div class="row gap-12 wrap mt-16">
      <a href="/auth/login" class="btn primary lg">{cta_primary}</a>
      <a href="/features" class="btn ghost lg">{cta_secondary}</a>
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
</section>
<div x-show="open" x-cloak
  role="dialog" aria-modal="true" aria-labelledby="demo-chat-modal-title"
  x-trap.noscroll.inert="open"
  @keydown.escape.window="open = false"
  style="position:fixed;inset:0;background:rgba(0,0,0,0.4);display:flex;align-items:flex-end;justify-content:center;z-index:1000;padding:20px">
  <div class="card" style="max-width:560px;width:100%;display:flex;flex-direction:column;padding:18px 22px 16px;max-height:80vh;margin-bottom:max(20px,env(safe-area-inset-bottom))">
    <div class="between mb-8">
      <div>
        <h2 id="demo-chat-modal-title" class="display-sm" style="margin:0">{chat_title}</h2>
        <p class="muted fs-13" style="margin:2px 0 0" x-text="currentPersona.slug === 'concierge' ? '{chat_subtitle_concierge}' : '{chat_subtitle}'"></p>
      </div>
      <button type="button" class="btn icon ghost" @click="open = false" aria-label="{chat_close}" style="padding:4px 10px;font-size:18px;line-height:1">&times;</button>
    </div>
    <div class="chat-controls">
      <label class="chat-persona-label">
        <span class="eyebrow">{chat_persona_label}</span>
        <select class="select chat-persona-select" x-model="personaSlug" :disabled="!personas.length" data-testid="demo-chat-persona">
          <!-- Placeholder while the catalog is loading or empty. `x-if`
               (vs `x-show`) actually removes the <option> from the DOM,
               because browsers treat `display:none` on <option>
               inconsistently otherwise, which manifests as a
               cropped/misaligned arrow. -->
          <template x-if="!personas.length">
            <option :value="personaSlug" x-text="personasLoaded ? '(no personas available)' : 'Loading…'"></option>
          </template>
          <template x-for="p in personas" :key="p.slug">
            <option :value="p.slug" x-text="p.label"></option>
          </template>
        </select>
      </label>
      <button type="button" class="btn ghost sm" @click="showPrompt = !showPrompt" :aria-expanded="showPrompt" :disabled="!personas.length" aria-controls="demo-chat-prompt-panel">
        <span x-show="!showPrompt">{chat_view_prompt}</span>
        <span x-show="showPrompt" x-cloak>{chat_hide_prompt}</span>
      </button>
    </div>
    <p class="muted fs-13 chat-persona-desc" x-text="personas.length ? currentPersona.description : (personasLoaded ? 'The persona catalog isn\'t ready yet. Apply the migration on the production D1 to populate it.' : 'Loading personas…')"></p>
    <!-- Roleplay frame card: shown only when the visitor picked a sample
         business persona (not the Concierge-self row). Tells them
         they're playing one of that business's customers, and lists the
         business's profile so they have something concrete to ask
         about. -->
    <div class="chat-business-card" x-show="currentPersona.slug !== 'concierge' && currentPersona.business" x-cloak>
      <p class="roleplay">{chat_roleplay_prefix} <strong x-text="currentPersona.business && currentPersona.business.name"></strong>{chat_roleplay_suffix}</p>
      <p class="biz-meta">
        <span x-show="currentPersona.business && currentPersona.business.business_type"><b>{chat_lbl_type}</b><span x-text="currentPersona.business && currentPersona.business.business_type"></span></span>
        <span x-show="currentPersona.business && currentPersona.business.city"><b>{chat_lbl_city}</b><span x-text="currentPersona.business && currentPersona.business.city"></span></span>
        <span x-show="currentPersona.business && currentPersona.business.hours"><b>{chat_lbl_hours}</b><span x-text="currentPersona.business && currentPersona.business.hours"></span></span>
        <span x-show="currentPersona.business && currentPersona.business.goal">
          <b>{chat_lbl_goal}</b>
          <span x-text="currentPersona.business && currentPersona.business.goal"></span>
          <template x-if="currentPersona.business && currentPersona.business.goal_url">
            <a class="biz-goal-link" :href="currentPersona.business.goal_url" x-text="currentPersona.business.goal_url" target="_blank" rel="noopener"></a>
          </template>
        </span>
      </p>
    </div>
    <!-- Single scroll region for the conversation + the toggleable prompt
         panel. Form stays pinned at the bottom even when the prompt panel
         is open and pushes the messages region's content overflow. -->
    <div class="chat-scroll" x-ref="msgs">
      <section id="demo-chat-prompt-panel" class="chat-prompt-panel" x-show="showPrompt" x-cloak aria-live="polite">
        <div class="eyebrow mb-6">{chat_prompt_heading}</div>
        <p class="muted fs-12 mb-6">{chat_envelope_note}</p>
        <pre class="chat-prompt-body chat-prompt-fixed" x-text="preamble"></pre>
        <pre class="chat-prompt-body chat-prompt-middle" x-text="currentPersona.prompt"></pre>
        <pre class="chat-prompt-body chat-prompt-fixed" x-text="postamble"></pre>
      </section>
      <div class="chat-messages">
        <template x-for="(m, i) in messages" :key="i">
          <div :class="'chat-msg ' + m.role" x-text="m.content"></div>
        </template>
        <div class="chat-thinking" x-show="sending">{chat_thinking}</div>
      </div>
    </div>
    <!-- Handoff chip: appears once the model has emitted the handoff
         token on a turn. Pure demo theater (there's no real human to
         take over), but it signals the same UX a tenant would see in
         their tenant-facing dashboard. -->
    <div class="chat-handoff-chip" x-show="handoff" x-cloak>
      <span class="chat-handoff-dot" aria-hidden="true"></span>
      {chat_handoff_chip}
    </div>
    <!-- Channels hint: visible to everyone, with a slightly different
         emphasis for the Concierge persona vs sample business personas.
         Reinforces that this chat box is the demo only; real customer
         messages arrive in WhatsApp / IG / Discord / email. -->
    <p class="chat-channels-note">{chat_channels_note}</p>
    <div class="chat-error" x-show="error" x-text="error"></div>
    <form x-show="!showCta" @submit.prevent="send()" class="row gap-8 mt-12 chat-form">
      <textarea class="chat-input" x-model="input" :placeholder="currentPersona.slug === 'concierge' ? '{chat_placeholder}' : ('{chat_placeholder_prefix} ' + currentPersona.label + ' {chat_placeholder_suffix}')"
        :disabled="sending || !personas.length" x-ref="input" maxlength="300" rows="2"
        @keydown.enter="if (!$event.shiftKey) {{ $event.preventDefault(); send(); }}"
        autocomplete="off" autocorrect="off" autocapitalize="off"></textarea>
      <button type="submit" class="btn primary" :disabled="sending || !input.trim() || !personas.length">{chat_send}</button>
    </form>
    <div class="chat-cta" x-show="showCta" x-cloak>
      <div class="chat-cta-text">
        <strong>{chat_cta_heading}</strong>
        <span>{chat_cta_body}</span>
      </div>
      <a href="/auth/login" class="btn primary">{chat_cta_button}</a>
    </div>
  </div>
</div>
</div>
{rotator}
{chat_script}"#,
        header = header,
        eyebrow = t(locale, "welcome-eyebrow"),
        hero_headline = hero_headline,
        hero_hint = hero_hint,
        lead = t(locale, "welcome-lead"),
        cta_primary = t(locale, "welcome-cta-primary"),
        cta_secondary = t(locale, "welcome-cta-secondary"),
        chat_title = chat_title,
        chat_subtitle = chat_subtitle,
        chat_subtitle_concierge = chat_subtitle_concierge,
        chat_persona_label = chat_persona_label,
        chat_roleplay_prefix = chat_roleplay_prefix,
        chat_roleplay_suffix = chat_roleplay_suffix,
        chat_channels_note = chat_channels_note,
        chat_lbl_hours = chat_lbl_hours,
        chat_lbl_city = chat_lbl_city,
        chat_lbl_type = chat_lbl_type,
        chat_lbl_goal = chat_lbl_goal,
        chat_handoff_chip = chat_handoff_chip,
        chat_view_prompt = chat_view_prompt,
        chat_hide_prompt = chat_hide_prompt,
        chat_prompt_heading = chat_prompt_heading,
        chat_placeholder = chat_placeholder,
        chat_placeholder_prefix = chat_placeholder_prefix,
        chat_placeholder_suffix = chat_placeholder_suffix,
        chat_send = chat_send,
        chat_close = chat_close,
        chat_thinking = chat_thinking,
        chat_cta_heading = chat_cta_heading,
        chat_cta_body = chat_cta_body,
        chat_cta_button = chat_cta_button,
        hash = HASH,
        rotator = rotator,
        chat_script = chat_script,
    );

    base_html("Concierge - Automated customer messaging", &content, locale)
}

/// Wrap a string for safe inline-`<script>` embedding: produce a JSON
/// double-quoted literal, then escape `<`, `>`, `&` so the text body
/// can't end the surrounding `</script>` or otherwise interact with the
/// HTML parser. The escapes survive JSON parsing intact at runtime.
fn js_string_for_html(s: &str) -> String {
    serde_json::to_string(s)
        .unwrap_or_else(|_| String::from("\"\""))
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

/// Typewriter rotator for the welcome page hero. The static headline is
/// rendered server-side; this script settles for ~10s, then mimics the
/// "type a few backspaces, give up, hit Ctrl+A, delete" pattern before
/// typing out a different variant. Suppressed entirely when the user
/// prefers reduced motion. Stands down while `window.__heroPaused` is
/// set (the chat modal flips that flag while it's open).
const HERO_ROTATOR_JS: &str = r##"<script type="module" nonce="__CSP_NONCE__">
((variants) => {
  const el = document.getElementById('hero-headline');
  if (!el || variants.length < 2) return;
  const mq = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)');
  if (mq && mq.matches) return;

  const tokenize = (html) => {
    const out = [];
    let i = 0;
    while (i < html.length) {
      const c = html[i];
      if (c === '<') {
        const j = html.indexOf('>', i);
        if (j < 0) { out.push(c); i += 1; continue; }
        out.push(html.slice(i, j + 1));
        i = j + 1;
      } else if (c === '&') {
        const j = html.indexOf(';', i);
        if (j > -1 && j - i <= 8) { out.push(html.slice(i, j + 1)); i = j + 1; }
        else { out.push(c); i += 1; }
      } else {
        out.push(c);
        i += 1;
      }
    }
    return out;
  };
  const isTag = (t) => t.length > 1 && t[0] === '<';
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const caret = '<span class="hero-caret" aria-hidden="true"></span>';

  let current = tokenize(variants[0]);
  let idx = 0;
  const render = (showCaret) => {
    el.innerHTML = current.join('') + (showCaret ? caret : '');
  };

  const cycle = async () => {
    while (true) {
      await sleep(10000);
      while (window.__heroPaused) await sleep(500);
      let next = idx;
      while (next === idx) next = Math.floor(Math.random() * variants.length);
      const target = tokenize(variants[next]);
      // Backspace burst: chip away a couple of words, like a user starting
      // to delete and then giving up. Targets ~10 chars on the default
      // headline and scales with length.
      let burst = Math.min(12, Math.max(4, Math.floor(current.length / 8)));
      while (current.length && burst > 0) {
        const top = current.pop();
        render(true);
        if (!isTag(top)) {
          burst -= 1;
          await sleep(50 + Math.random() * 30);
        }
      }
      // Ctrl+A flash: highlight everything that's left and hold long enough
      // to read as a deliberate select-all.
      if (current.length) {
        el.innerHTML = '<span class="hero-select">' + current.join('') + '</span>' + caret;
        await sleep(540);
      }
      // Delete: clear the field, hold the caret at the start for a beat.
      current.length = 0;
      render(true);
      await sleep(220);
      for (const tok of target) {
        current.push(tok);
        render(true);
        if (!isTag(tok)) await sleep(28 + Math.random() * 30);
      }
      // Caret stays visible at idle; never call render(false).
      idx = next;
    }
  };
  cycle().catch(() => {});
})(__VARIANTS__);
</script>"##;

/// Alpine factory for the welcome-page chat modal. Powers the "Ask me
/// about Concierge" CTA: opens the modal, posts the running transcript
/// to `/demo/chat`, and renders the model's reply. Pauses the headline
/// rotator (`window.__heroPaused`) while open so the headline doesn't
/// mutate behind the modal.
///
/// Plain `<script>` (not `type="module"`) on purpose: it must define
/// `window.conciergeChat` *before* Alpine's deferred script in `<head>`
/// boots and processes `x-data="conciergeChat()"`. Inline classic
/// scripts run synchronously during parsing; module scripts are
/// deferred and would be too late.
const HERO_CHAT_JS: &str = r##"<script nonce="__CSP_NONCE__">
(() => {
  const PREAMBLE = __PREAMBLE__;
  const POSTAMBLE = __POSTAMBLE__;
  const findPersona = (slug, personas) =>
    personas.find((p) => p.slug === slug) || personas[0] || null;
  // Demo session is intentionally short. After this many user turns
  // the input form is replaced with the sign-up CTA; the visitor has
  // either gotten the gist by then or they haven't.
  const TURN_LIMIT = 3;
  // Same CTA fires automatically after this many ms of the modal
  // being open, so visitors who park the modal without typing also
  // see the next step.
  const CTA_TIMEOUT_MS = 30000;
  window.conciergeChat = () => ({
    open: false,
    sending: false,
    error: '',
    input: '',
    showPrompt: false,
    // Set true once /demo/chat returns handoff:true on a turn. Echoed
    // back on every subsequent send so the server replies under the
    // holding-pattern middle. Resets on persona switch and modal close.
    handoff: false,
    // Personas come from /demo/personas (Approved-only D1 catalog).
    // Empty until init() fetches; dropdown shows a loading row.
    personas: [],
    personasLoaded: false,
    personaSlug: 'concierge',
    messages: [],
    preamble: PREAMBLE,
    postamble: POSTAMBLE,
    // Sticky for the duration of the modal session: once the timer
    // fires (or the visitor crosses the turn limit) we keep the CTA
    // up even if they switch personas. Cleared on modal close.
    ctaShown: false,
    _ctaTimer: null,
    _defaultPersonaSlug: 'concierge',
    get userTurns() {
      let n = 0;
      for (const m of this.messages) if (m.role === 'user') n += 1;
      return n;
    },
    get atTurnLimit() {
      return this.userTurns >= TURN_LIMIT;
    },
    get showCta() {
      return this.ctaShown || this.atTurnLimit;
    },
    get currentPersona() {
      return findPersona(this.personaSlug, this.personas) || {
        slug: 'concierge', label: 'Concierge', description: '',
        greeting: 'Loading…', prompt: '', slug: '', business: null,
      };
    },
    async init() {
      try {
        const r = await fetch('/demo/personas');
        const data = r.ok ? await r.json() : { personas: [] };
        this.personas = (data && Array.isArray(data.personas)) ? data.personas : [];
      } catch (_) {
        this.personas = [];
      }
      this.personasLoaded = true;
      if (this.personas.length) {
        const has = this.personas.some((p) => p.slug === this.personaSlug);
        if (!has) this.personaSlug = this.personas[0].slug;
      }
      this._defaultPersonaSlug = this.personaSlug;
      this.resetTranscript();
      this.$watch('open', (v) => {
        window.__heroPaused = !!v;
        if (v) {
          this._ctaTimer = setTimeout(() => { this.ctaShown = true; }, CTA_TIMEOUT_MS);
          this.$nextTick(() => {
            if (this.$refs.input) this.$refs.input.focus();
            this.scrollDown();
          });
        } else {
          if (this._ctaTimer) { clearTimeout(this._ctaTimer); this._ctaTimer = null; }
          this.ctaShown = false;
          this.personaSlug = this._defaultPersonaSlug;
          this.resetTranscript();
        }
      });
      this.$watch('personaSlug', () => {
        this.resetTranscript();
        this.$nextTick(() => {
          if (this.$refs.input) this.$refs.input.focus();
          this.scrollDown();
        });
      });
    },
    resetTranscript() {
      const p = this.currentPersona;
      this.messages = [{ role: 'assistant', content: p.greeting }];
      this.error = '';
      this.input = '';
      this.showPrompt = false;
      this.handoff = false;
    },
    scrollDown() {
      const el = this.$refs.msgs;
      if (el) el.scrollTop = el.scrollHeight;
    },
    async send() {
      if (this.showCta) return;
      const text = this.input.trim();
      if (!text || this.sending) return;
      this.error = '';
      this.input = '';
      this.messages.push({ role: 'user', content: text });
      this.$nextTick(() => this.scrollDown());
      this.sending = true;
      try {
        // Drop any leading assistant turns: the client-side greeting is for
        // display only. Llama chat templates expect the first non-system
        // message to be user, so leading with assistant breaks generation.
        const wireMessages = [];
        let started = false;
        for (const m of this.messages) {
          if (!started && m.role !== 'user') continue;
          started = true;
          wireMessages.push({ role: m.role, content: m.content });
        }
        const r = await fetch('/demo/chat', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            persona: this.personaSlug,
            messages: wireMessages,
            handoff: this.handoff,
          }),
        });
        const data = await r.json().catch(() => ({}));
        if (!r.ok) {
          this.error = (r.status === 429) ? __RATE_LIMITED__ : (data && data.error) || __ERROR__;
          if (r.status >= 400 && r.status !== 429) {
            this.messages.pop();
          }
        } else if (data && typeof data.reply === 'string' && data.reply.length) {
          this.messages.push({ role: 'assistant', content: data.reply });
          // Once the model emits [[HANDOFF]] (server strips the token
          // before sending us the reply but flags it here), flip into
          // holding-pattern mode for any further sends.
          if (data.handoff === true) this.handoff = true;
        } else {
          this.error = __ERROR__;
        }
      } catch (e) {
        this.error = __ERROR__;
      }
      this.sending = false;
      this.$nextTick(() => {
        this.scrollDown();
        if (this.$refs.input) this.$refs.input.focus();
      });
    },
  });
})();
</script>"##;

pub fn basics_html(
    business: &crate::types::BusinessInfo,
    base_url: &str,
    locale: &crate::locale::Locale,
) -> String {
    let biz_type_options: [(&str, String); 6] = [
        ("", t(locale, "wizard-basics-type-default")),
        ("unregistered", t(locale, "wizard-basics-type-unregistered")),
        (
            "sole_proprietorship",
            t(locale, "wizard-basics-type-sole-prop"),
        ),
        ("partnership", t(locale, "wizard-basics-type-partnership")),
        ("pvt_ltd", t(locale, "wizard-basics-type-pvt-ltd")),
        ("llp", t(locale, "wizard-basics-type-llp")),
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
  <div class="section-label"><span class="mono muted">01 / 05</span><span class="eyebrow">{eyebrow}</span></div>
  <h2 class="display-md">{headline}</h2>
  <p class="lead">{lead}</p>
  <form hx-post="{base_url}/admin/wizard/basics" hx-target="body" hx-swap="innerHTML">
    <div class="card p-24">
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px">
        <div>
          <label for="biz-name" class="eyebrow lbl">{lbl_name}</label>
          <input id="biz-name" class="input" name="name" value="{name}" placeholder="{ph_name}" required aria-required="true" x-model="name">
        </div>
        <div>
          <label for="biz-contact-name" class="eyebrow lbl">{lbl_contact}</label>
          <input id="biz-contact-name" class="input" name="contact_name" value="{contact_name}" placeholder="{ph_contact}">
        </div>
        <div>
          <label for="biz-phone" class="eyebrow lbl">{lbl_phone}</label>
          <input id="biz-phone" class="input" type="tel" name="phone" value="{phone}" placeholder="{ph_phone}" required aria-required="true" x-model="phone">
        </div>
        <div>
          <label for="biz-type" class="eyebrow lbl">{lbl_type}</label>
          <select id="biz-type" class="select" name="business_type" x-model="bizType">{biz_type_html}</select>
        </div>
      </div>
      <div class="mt-16" x-show="bizType &amp;&amp; bizType !== 'unregistered'" x-cloak :aria-hidden="!(bizType &amp;&amp; bizType !== 'unregistered')" style="grid-template-columns:1fr 1fr;gap:16px;display:grid">
        <div>
          <label for="biz-pan" class="eyebrow lbl">{lbl_pan}</label>
          <input id="biz-pan" class="input" name="pan" value="{pan}" placeholder="{ph_pan}" style="text-transform:uppercase">
        </div>
        <div>
          <label for="biz-gstin" class="eyebrow lbl">{lbl_gstin_pre} <span class="muted">{lbl_gstin_suf}</span></label>
          <input id="biz-gstin" class="input" name="gstin" value="{gstin}" placeholder="{ph_gstin}" style="text-transform:uppercase">
        </div>
        <div style="grid-column:1/-1">
          <label for="biz-address" class="eyebrow lbl">{lbl_address}</label>
          <textarea id="biz-address" class="textarea" name="address" rows="2" placeholder="{ph_address}">{address}</textarea>
        </div>
        <div>
          <label for="biz-state" class="eyebrow lbl">{lbl_state}</label>
          <input id="biz-state" class="input" name="state" value="{state}" placeholder="{ph_state}">
        </div>
        <div>
          <label for="biz-pincode" class="eyebrow lbl">{lbl_pincode}</label>
          <input id="biz-pincode" class="input" name="pincode" value="{pincode}" placeholder="{ph_pincode}" pattern="[0-9]{{6}}" maxlength="6">
        </div>
      </div>
    </div>
    <div class="between mt-36">
      <a href="/" class="btn ghost">{back}</a>
      <button class="btn primary" type="submit" :disabled="!(name &amp;&amp; name.trim() &amp;&amp; phone &amp;&amp; phone.trim())">{cont}</button>
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
        eyebrow = t(locale, "wizard-basics-eyebrow"),
        headline = t(locale, "wizard-basics-headline"),
        lead = t(locale, "wizard-basics-lead"),
        lbl_name = t(locale, "wizard-basics-label-name"),
        lbl_contact = t(locale, "wizard-basics-label-contact"),
        lbl_phone = t(locale, "wizard-basics-label-phone"),
        lbl_type = t(locale, "wizard-basics-label-type"),
        lbl_pan = t(locale, "wizard-basics-label-pan"),
        lbl_gstin_pre = t(locale, "wizard-basics-label-gstin-prefix"),
        lbl_gstin_suf = t(locale, "wizard-basics-label-gstin-suffix"),
        lbl_address = t(locale, "wizard-basics-label-address"),
        lbl_state = t(locale, "wizard-basics-label-state"),
        lbl_pincode = t(locale, "wizard-basics-label-pincode"),
        ph_name = t(locale, "wizard-basics-placeholder-name"),
        ph_contact = t(locale, "wizard-basics-placeholder-contact"),
        ph_phone = t(locale, "wizard-basics-placeholder-phone"),
        ph_pan = t(locale, "wizard-basics-placeholder-pan"),
        ph_gstin = t(locale, "wizard-basics-placeholder-gstin"),
        ph_address = t(locale, "wizard-basics-placeholder-address"),
        ph_state = t(locale, "wizard-basics-placeholder-state"),
        ph_pincode = t(locale, "wizard-basics-placeholder-pincode"),
        back = t(locale, "wizard-back"),
        cont = t(locale, "wizard-continue"),
    );

    let x_data = format!(
        "{{ name: '{}', phone: '{}', bizType: '{}' }}",
        js_attr_escape(&business.name),
        js_attr_escape(&business.phone),
        js_attr_escape(&business.business_type),
    );
    let progress_expr =
        "((name && name.trim() ? 0.4 : 0) + (phone && phone.trim() ? 0.4 : 0) + (bizType ? 0.2 : 0))";

    wizard_shell("basics", base_url, &x_data, progress_expr, &content, locale)
}

pub fn connect_html(
    ig_connected: bool,
    wa_connected: bool,
    email_addresses: &[crate::types::EmailAddress],
    suggested_slug: &str,
    email_base_domain: &str,
    tenant_id: &str,
    discord: Option<&crate::types::DiscordConfig>,
    base_url: &str,
    locale: &crate::locale::Locale,
    cfg: &crate::storage::Pricing,
) -> String {
    let address_paise = cfg.address_price("INR");
    let address_cents = cfg.address_price("USD");
    let email_pack_size = cfg.email_pack_size;
    let ig_name = t(locale, "wizard-channels-name-instagram");
    let ig_flavor = t(locale, "wizard-channels-flavor-instagram");
    let ig_demo = t(locale, "wizard-channels-handle-instagram-demo");
    let ig_card = channel_card(
        "ig",
        &ig_name,
        ig_connected,
        &ig_demo,
        &ig_flavor,
        tenant_id,
        base_url,
        locale,
    );
    let wa_name = t(locale, "wizard-channels-name-whatsapp");
    let wa_flavor = t(locale, "wizard-channels-flavor-whatsapp");
    let wa_demo = t(locale, "wizard-channels-handle-whatsapp-demo");
    let wa_card = channel_card(
        "wa",
        &wa_name,
        wa_connected,
        &wa_demo,
        &wa_flavor,
        tenant_id,
        base_url,
        locale,
    );
    let dc_fallback = t(locale, "wizard-channels-discord-handle-fallback");
    let (dc_connected, dc_handle) = match discord {
        Some(c) => (
            true,
            c.guild_name.clone().unwrap_or_else(|| dc_fallback.clone()),
        ),
        None => (false, String::new()),
    };
    let dc_name = t(locale, "wizard-channels-name-discord");
    let dc_flavor = t(locale, "wizard-channels-flavor-discord");
    let dc_card = channel_card(
        "discord",
        &dc_name,
        dc_connected,
        &dc_handle,
        &dc_flavor,
        tenant_id,
        base_url,
        locale,
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
    <div><div class="channel-name">{email_name}</div></div>
  </div>
  <div class="channel-body">
    <p class="muted m-0 mb-12">{lead_prefix} <code>name@{base_domain}</code>{lead_suffix}</p>
    {email_rows}
    <form hx-post="{base_url}/admin/wizard/email/add" hx-target="body" hx-swap="innerHTML"
          class="row gap-8 mt-8">
      <label for="wiz-email-label" class="sr-only">{email_name}</label>
      <input id="wiz-email-label" class="input fs-13" type="text" name="label" value="{slug}" placeholder="{ph}" style="max-width:160px">
      <span class="mono muted fs-13">@{base_domain}</span>
      <button type="submit" class="btn sm ml-auto">{add}</button>
    </form>
    <div class="mono muted fs-11 mt-6">{help}</div>
  </div>
</div>"#,
            mail_icon = channel_icon("mail"),
            email_rows = email_rows,
            base_url = base_url,
            slug = html_escape(suggested_slug),
            base_domain = html_escape(email_base_domain),
            email_name = t(locale, "wizard-channels-name-email"),
            lead_prefix = t(locale, "wizard-channels-email-lead-prefix"),
            lead_suffix = t(locale, "wizard-channels-email-lead-suffix"),
            ph = t(locale, "wizard-channels-email-placeholder"),
            add = t(locale, "wizard-channels-email-add"),
            help = crate::i18n::t_args(
                locale,
                "wizard-channels-email-help",
                &[
                    ("inr", &format!("₹{:.0}", address_paise as f64 / 100.0)),
                    ("usd", &format!("${:.0}", address_cents as f64 / 100.0)),
                    ("pack_size", &email_pack_size.to_string()),
                ],
            ),
        )
    };

    let has_anything = ig_connected || wa_connected || !email_addresses.is_empty();
    let continue_label = if has_anything {
        t(locale, "wizard-channels-continue")
    } else {
        t(locale, "wizard-channels-skip")
    };

    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="mono muted">02 / 05</span><span class="eyebrow">{eyebrow}</span></div>
  <h2 class="display-md">{headline}</h2>
  <p class="lead">{lead}</p>
  <div class="channels-grid">{ig_card}{wa_card}{dc_card}{email_section}</div>
  <div class="between mt-36">
    <button class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"basics"}}' hx-target="body" hx-swap="innerHTML">{back}</button>
    <button class="btn primary" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"notifications"}}' hx-target="body" hx-swap="innerHTML">{continue_label}</button>
  </div>
</section>"#,
        ig_card = ig_card,
        wa_card = wa_card,
        dc_card = dc_card,
        email_section = email_section,
        base_url = base_url,
        continue_label = continue_label,
        eyebrow = t(locale, "wizard-channels-eyebrow"),
        headline = t(locale, "wizard-channels-headline"),
        lead = t(locale, "wizard-channels-lead"),
        back = t(locale, "wizard-back"),
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

    wizard_shell(
        "channels",
        base_url,
        &x_data,
        progress_expr,
        &content,
        locale,
    )
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

pub fn channel_card_html(p: &ChannelCardProps, locale: &crate::locale::Locale) -> String {
    if p.connected {
        format!(
            r#"<div class="channel is-connected">
  <div class="ribbon">{connected_lbl}</div>
  <div class="channel-head">
    <div class="channel-mark">{icon}</div>
    <div><div class="channel-name">{name}</div></div>
    <span class="dot ok ml-auto"></span>
  </div>
  <div class="channel-body">
    <div class="mono text-ok fs-12">&#x25CF; {active_lbl}</div>
    <div class="serif mt-4" style="font-size:18px;line-height:1.2">{status}</div>
  </div>
  <div class="row gap-8">
    <a href="{manage_href}" class="btn ghost sm">{manage_lbl}</a>
  </div>
</div>"#,
            icon = channel_icon(p.key),
            name = html_escape(p.name),
            status = html_escape(p.status_line),
            manage_href = p.manage_href,
            connected_lbl = t(locale, "wizard-channels-card-connected"),
            active_lbl = t(locale, "wizard-channels-card-active"),
            manage_lbl = t(locale, "wizard-channels-card-manage"),
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
  <a href="{connect_href}" class="btn">{connect_lbl}</a>
</div>"#,
            icon = channel_icon(p.key),
            name = html_escape(p.name),
            flavor = html_escape(p.status_line),
            connect_href = p.connect_href,
            connect_lbl = t(locale, "wizard-channels-card-connect"),
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
    locale: &crate::locale::Locale,
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
    channel_card_html(
        &ChannelCardProps {
            key,
            name,
            connected,
            status_line,
            connect_href: &connect_href,
            manage_href: &manage_href,
        },
        locale,
    )
}

pub fn channel_icon(key: &str) -> &'static str {
    // All icons are decorative; they render next to a textual channel name
    // (e.g. "Instagram DMs"), so AT users get the name from the label, not
    // the icon.
    match key {
        "ig" => {
            r#"<svg width="22" height="22" viewBox="0 0 24 24" fill="none" aria-hidden="true" focusable="false"><rect x="3" y="3" width="18" height="18" rx="5" stroke="currentColor" stroke-width="1.6"/><circle cx="12" cy="12" r="4.2" stroke="currentColor" stroke-width="1.6"/><circle cx="17.2" cy="6.8" r="1.1" fill="currentColor"/></svg>"#
        }
        "wa" => {
            r#"<svg width="22" height="22" viewBox="0 0 24 24" fill="none" aria-hidden="true" focusable="false"><path d="M4 20l1.3-4.1A8 8 0 1 1 8.2 18.8L4 20z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg>"#
        }
        "discord" => {
            r#"<svg width="22" height="22" viewBox="0 0 24 24" fill="none" aria-hidden="true" focusable="false"><path d="M7 7c1.4-.7 3-1 5-1s3.6.3 5 1l1 1 1.5 4.5c.2 2-.3 3.8-1.5 5.5-1 .3-2 .5-3 .5l-1-1.5c.5-.2 1-.4 1.5-.8-.3-.2-.8-.4-1.2-.5-2 .7-4.6.7-6.6 0-.4.1-.9.3-1.2.5.5.4 1 .6 1.5.8L6 17.5c-1 0-2-.2-3-.5-1.2-1.7-1.7-3.5-1.5-5.5L3 7l1-1z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/></svg>"#
        }
        "mail" => {
            r#"<svg width="22" height="22" viewBox="0 0 24 24" fill="none" aria-hidden="true" focusable="false"><rect x="3" y="5" width="18" height="14" rx="2" stroke="currentColor" stroke-width="1.6"/><path d="M3.5 6.5l8.5 6 8.5-6" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/></svg>"#
        }
        _ => "",
    }
}

pub fn notifications_html(
    config: &crate::types::NotificationConfig,
    discord_installed: bool,
    base_url: &str,
    locale: &crate::locale::Locale,
) -> String {
    use crate::types::DigestCadence;
    let cadences = [
        DigestCadence::Instant,
        DigestCadence::Every15Min,
        DigestCadence::Hourly,
        DigestCadence::Every4Hours,
        DigestCadence::Daily,
    ];
    let approval_freq_html: String = cadences
        .iter()
        .map(|c| {
            let sel = if *c == config.approval_email_cadence {
                " selected"
            } else {
                ""
            };
            format!(
                r#"<option value="{val}"{sel}>{label}</option>"#,
                val = c.as_str(),
                sel = sel,
                label = c.label(),
            )
        })
        .collect();

    let b = |v: bool| if v { "true" } else { "false" };

    let content = format!(
        r##"<section class="page narrow">
  <div class="section-label"><span class="mono muted">03 / 05</span><span class="eyebrow">{eyebrow}</span></div>
  <h2 class="display-md">{headline}</h2>
  <p class="lead">{lead}</p>

  <form hx-post="{base_url}/admin/wizard/notifications" hx-target="#notif-toast" hx-swap="innerHTML">
    <div class="card p-22 mb-16">
      <div class="eyebrow mb-12">{card_eyebrow} <span class="text-warn">{required_mark}</span></div>
      <p class="muted mb-14 fs-14">{card_lead}</p>
      <div class="admin-grid" role="group" aria-label="{card_eyebrow}">
        <label class="admin-card" :class="approval.discord ? 'selected' : ''" style="min-height:auto;cursor:pointer">
          <input type="hidden" name="approval_discord" value="false">
          <input type="checkbox" name="approval_discord" value="true" class="hidden" x-model="approval.discord">
          <div class="row gap-12">
            <div class="admin-mark icon-chip">{discord_icon}</div>
            <div><div class="fw-600">{discord_lbl}</div>
            <div class="mono muted fs-11">{discord_sub}</div></div>
          </div>
        </label>
        <label class="admin-card" :class="approval.email ? 'selected' : ''" style="min-height:auto;cursor:pointer">
          <input type="hidden" name="approval_email" value="false">
          <input type="checkbox" name="approval_email" value="true" class="hidden" x-model="approval.email">
          <div class="row gap-12">
            <div class="admin-mark icon-chip">{mail_icon}</div>
            <div><div class="fw-600">{email_lbl}</div>
            <div class="mono muted fs-11">{email_sub}</div></div>
          </div>
          <div class="freq-row row gap-8 mt-12" x-show="approval.email" x-cloak :aria-hidden="!approval.email">
            <span class="mono muted fs-12">{cadence_prefix}</span>
            <label for="wiz-approval-cadence" class="sr-only">{cadence_prefix}</label>
            <select id="wiz-approval-cadence" class="select fs-13" name="approval_cadence" style="width:auto;padding:6px 10px">{approval_freq_html}</select>
          </div>
        </label>
      </div>
      <div class="card-soft p-14 mt-12" x-show="approval.discord && !{dc_installed_js}" x-cloak :aria-hidden="!(approval.discord && !{dc_installed_js})">
        <div class="row gap-12">
          <div class="fs-13 flex-1">{discord_missing}</div>
          <a href="{base_url}/admin/discord/install?from=wizard_heads_up" class="btn sm primary">{discord_install}</a>
        </div>
      </div>
    </div>

    <div class="between mt-36">
      <button type="button" class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"channels"}}' hx-target="body" hx-swap="innerHTML">{back}</button>
      <button type="submit" class="btn primary" :disabled="!approval.discord && !approval.email">{cont}</button>
    </div>
    <div id="notif-toast" class="mt-12" role="status" aria-live="polite" aria-atomic="true"></div>
  </form>
</section>"##,
        base_url = base_url,
        discord_icon = channel_icon("discord"),
        mail_icon = channel_icon("mail"),
        approval_freq_html = approval_freq_html,
        dc_installed_js = if discord_installed { "true" } else { "false" },
        eyebrow = t(locale, "wizard-notifications-eyebrow"),
        headline = t(locale, "wizard-notifications-headline"),
        lead = t(locale, "wizard-notifications-lead"),
        card_eyebrow = t(locale, "wizard-notifications-card-eyebrow"),
        required_mark = t(locale, "wizard-notifications-card-required"),
        card_lead = t(locale, "wizard-notifications-card-lead"),
        discord_lbl = t(locale, "wizard-notifications-channel-discord"),
        discord_sub = t(locale, "wizard-notifications-channel-discord-sub"),
        email_lbl = t(locale, "wizard-notifications-channel-email"),
        email_sub = t(locale, "wizard-notifications-channel-email-sub"),
        cadence_prefix = t(locale, "wizard-notifications-cadence-prefix"),
        discord_missing = t(locale, "wizard-notifications-discord-missing"),
        discord_install = t(locale, "wizard-notifications-discord-install"),
        back = t(locale, "wizard-back"),
        cont = t(locale, "wizard-notifications-continue"),
    );

    let x_data = format!(
        "{{ approval: {{ discord: {ad}, email: {ae} }} }}",
        ad = b(config.approval_discord),
        ae = b(config.approval_email),
    );
    let progress_expr = "((approval.discord || approval.email) ? 1.0 : 0)";

    wizard_shell(
        "notifications",
        base_url,
        &x_data,
        progress_expr,
        &content,
        locale,
    )
}

pub fn replies_html(
    persona: &PersonaConfig,
    archetypes: &[crate::types::Archetype],
    default_wait_seconds: u32,
    base_url: &str,
    locale: &crate::locale::Locale,
) -> String {
    // Highlight the archetype the tenant currently has saved (if any).
    let current_slug = match &persona.source {
        PersonaSource::Builder(b) => b.archetype_slug.as_str(),
        _ => "",
    };
    // Pre-fill goal + handoff fields if the tenant has already filled
    // them once (the wizard can be re-entered before launch).
    let (current_goal, current_goal_url, current_handoff) = match &persona.source {
        PersonaSource::Builder(b) => (
            b.goal.clone(),
            b.goal_url.clone(),
            b.handoff_conditions.join("\n"),
        ),
        _ => (String::new(), String::new(), String::new()),
    };

    let preset_cards: String = archetypes
        .iter()
        .map(|a| {
            let slug = &a.slug;
            let label = &a.label;
            let desc = &a.description;
            let checked = if slug == current_slug { " checked" } else { "" };
            format!(
                r#"<label class="card p-18 preset-card" style="cursor:pointer;display:block">
  <div class="row gap-12" style="align-items:flex-start">
    <input type="radio" name="preset_id" value="{slug}" x-model="preset"{checked} style="margin-top:4px">
    <div class="flex-1">
      <div class="eyebrow mb-4">{label}</div>
      <p class="m-0 fs-14">{desc}</p>
    </div>
  </div>
</label>"#,
                slug = html_escape(slug),
                label = html_escape(label),
                desc = html_escape(desc),
                checked = checked,
            )
        })
        .collect();

    let content = format!(
        r#"<section class="page narrow">
  <div class="section-label"><span class="mono muted">04 / 05</span><span class="eyebrow">{eyebrow}</span></div>
  <h2 class="display-md">{headline}</h2>
  <p class="lead">{lead_prefix} <a href="{base_url}/admin/persona">{lead_link}</a> {lead_suffix}</p>

  <form hx-post="{base_url}/admin/wizard/replies/save" hx-target="body" hx-swap="innerHTML">
    <div style="display:grid;gap:12px;grid-template-columns:1fr 1fr;margin-bottom:24px" role="radiogroup" aria-labelledby="replies-preset-label">
      <span id="replies-preset-label" class="sr-only">{headline}</span>
      {preset_cards}
    </div>

    <div class="card p-22 mb-16">
      <div class="eyebrow mb-8" id="wiz-goal-label">{goal_eyebrow}</div>
      <p class="muted fs-13 m-0 mb-12">{goal_lead}</p>
      <input id="wiz-goal" class="input" type="text" name="goal" maxlength="120"
             value="{goal_value}"
             placeholder="{goal_placeholder}"
             aria-labelledby="wiz-goal-label"
             autocomplete="off">
      <label for="wiz-goal-url" class="eyebrow lbl mt-12">{goal_url_label}</label>
      <input id="wiz-goal-url" class="input" type="text" name="goal_url" maxlength="200"
             value="{goal_url_value}"
             placeholder="{goal_url_placeholder}"
             autocomplete="off">
    </div>

    <div class="card p-22 mb-16">
      <div class="eyebrow mb-8" id="wiz-handoff-label">{handoff_eyebrow}</div>
      <p class="muted fs-13 m-0 mb-12">{handoff_lead}</p>
      <textarea id="wiz-handoff" class="textarea" name="handoff_conditions" rows="3"
                placeholder="{handoff_placeholder}"
                aria-labelledby="wiz-handoff-label">{handoff_value}</textarea>
    </div>

    <div class="card p-22 mb-16">
      <div class="eyebrow mb-8" id="wiz-wait-label">{wait_eyebrow}</div>
      <p class="muted fs-13 m-0 mb-12">{wait_lead}</p>
      <div class="row gap-12">
        <input type="range" min="0" max="30" step="1" name="default_wait_seconds"
               x-model.number="waitSeconds"
               class="flex-1"
               aria-labelledby="wiz-wait-label"
               style="accent-color:var(--accent)">
        <div class="mono ta-right" style="min-width:80px">
          <span x-text="waitSeconds === 0 ? '{instant}' : waitSeconds + 's'"></span>
        </div>
      </div>
    </div>

    <div class="between mt-32">
      <button type="button" class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"notifications"}}' hx-target="body" hx-swap="innerHTML">{back}</button>
      <button type="submit" class="btn primary" :disabled="!preset">{cont}</button>
    </div>
  </form>
</section>"#,
        base_url = base_url,
        preset_cards = preset_cards,
        eyebrow = t(locale, "wizard-replies-eyebrow"),
        headline = t(locale, "wizard-replies-headline"),
        lead_prefix = t(locale, "wizard-replies-lead-prefix"),
        lead_link = t(locale, "wizard-replies-lead-link"),
        lead_suffix = t(locale, "wizard-replies-lead-suffix"),
        goal_eyebrow = t(locale, "wizard-replies-goal-eyebrow"),
        goal_lead = t(locale, "wizard-replies-goal-lead"),
        goal_placeholder = html_escape(&t(locale, "wizard-replies-goal-placeholder")),
        goal_url_label = t(locale, "wizard-replies-goal-url-label"),
        goal_url_placeholder = html_escape(&t(locale, "wizard-replies-goal-url-placeholder")),
        goal_value = html_escape(&current_goal),
        goal_url_value = html_escape(&current_goal_url),
        handoff_eyebrow = t(locale, "wizard-replies-handoff-eyebrow"),
        handoff_lead = t(locale, "wizard-replies-handoff-lead"),
        handoff_placeholder = html_escape(&t(locale, "wizard-replies-handoff-placeholder")),
        handoff_value = html_escape(&current_handoff),
        wait_eyebrow = t(locale, "wizard-replies-wait-eyebrow"),
        wait_lead = t(locale, "wizard-replies-wait-lead"),
        instant = t(locale, "wizard-replies-wait-instant"),
        back = t(locale, "wizard-back"),
        cont = t(locale, "wizard-replies-continue"),
    );

    let x_data = format!(
        "{{ preset: '{}', waitSeconds: {} }}",
        js_attr_escape(current_slug),
        default_wait_seconds,
    );
    let progress_expr = "(preset ? 1.0 : 0.0)";

    wizard_shell(
        "replies",
        base_url,
        &x_data,
        progress_expr,
        &content,
        locale,
    )
}

pub fn launch_html(
    email_addresses: &[crate::types::EmailAddress],
    base_domain: &str,
    locale: &crate::locale::Locale,
    base_url: &str,
    milli_price: i64,
    verified: bool,
    verification_amount: i64,
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
  <div class="eyebrow mb-8">{eyebrow}</div>
  <p class="muted mb-12 fs-14">{body}</p>
  {email_rows}
</div>"#,
            eyebrow = t(locale, "wizard-launch-email-eyebrow"),
            body = t(locale, "wizard-launch-email-body"),
        )
    };

    let credits_section = format!(
        r#"<div class="mb-16">
  {slider}
  <p class="muted ta-center mt-8 fs-12">{note}</p>
</div>"#,
        slider = crate::templates::credit_slider::slider_html(
            locale,
            base_url,
            crate::templates::credit_slider::SliderMode::Buy {
                return_to: "/admin/wizard/launch",
            },
            milli_price,
        ),
        note = t(locale, "wizard-launch-credits-note"),
    );

    let status_card = if verified {
        format!(
            r#"<div class="card p-22" style="border-color:var(--ok);background:linear-gradient(135deg,var(--paper),#E8F0DE)">
    <div class="row gap-12">
      <span class="dot ok"></span>
      <div>
        <div class="fw-600">{headline}</div>
        <p class="muted fs-14 m-0 mt-4">{body}</p>
      </div>
    </div>
  </div>"#,
            headline = t(locale, "wizard-launch-status-headline"),
            body = t(locale, "wizard-launch-status-body"),
        )
    } else {
        let amount_label = crate::helpers::format_money(verification_amount, locale);
        format!(
            r##"<div class="card p-22" style="border-color:var(--accent)">
    <div class="row gap-12" style="align-items:flex-start">
      <span class="dot"></span>
      <div class="flex-1">
        <div class="fw-600">{headline}</div>
        <p class="muted fs-14 m-0 mt-4 mb-12">{body}</p>
        <form hx-post="{base_url}/admin/billing/verification" hx-target="body" hx-swap="innerHTML" hx-ext="json-enc">
          <input type="hidden" name="return_to" value="/admin/wizard/launch">
          <button type="submit" class="btn primary">{cta} ({amount})</button>
          <span class="mono muted fs-11 ml-12">{refund}</span>
        </form>
      </div>
    </div>
  </div>"##,
            base_url = base_url,
            headline = t(locale, "wizard-launch-verify-headline"),
            body = t(locale, "wizard-launch-verify-body"),
            cta = t(locale, "wizard-launch-verify-cta"),
            refund = t(locale, "wizard-launch-verify-refund"),
            amount = amount_label,
        )
    };

    let finish_attrs = if verified {
        ""
    } else {
        " disabled aria-disabled=\"true\" title=\"Verify your account first\""
    };

    let content = format!(
        r##"<section class="page narrow">
  <div class="section-label"><span class="mono muted">05 / 05</span><span class="eyebrow">{eyebrow}</span></div>
  <h2 class="display-md">{headline}</h2>
  <p class="lead">{lead}</p>

  {email_section}
  {credits_section}

  {status_card}

  <div class="between mt-36">
    <button class="btn ghost" hx-post="{base_url}/admin/wizard/goto" hx-vals='{{"to":"replies"}}' hx-target="body" hx-swap="innerHTML">{back}</button>
    <button class="btn primary" hx-post="{base_url}/admin/wizard/complete" hx-target="body"{finish_attrs}>{finish}</button>
  </div>
</section>"##,
        email_section = email_section,
        credits_section = credits_section,
        status_card = status_card,
        base_url = base_url,
        eyebrow = t(locale, "wizard-launch-eyebrow"),
        headline = t(locale, "wizard-launch-headline"),
        lead = t(locale, "wizard-launch-lead"),
        back = t(locale, "wizard-back"),
        finish = t(locale, "wizard-launch-finish"),
        finish_attrs = finish_attrs,
    );

    // Progress on the launch step is always full: addresses are live the
    // moment they're added (no payment gate any more).
    let _ = email_addresses;
    let x_data = "{}".to_string();
    let progress_expr = "1";

    wizard_shell("launch", base_url, &x_data, progress_expr, &content, locale)
}

/// Public pricing page at /pricing. The `?c=usd` query param swaps the
/// display currency without changing the visitor's UI language.
pub fn pricing_html(
    default_currency: &str,
    locale: &crate::locale::Locale,
    cfg: &crate::storage::Pricing,
) -> String {
    use crate::helpers::format_money;
    use crate::locale::Currency;

    let milli_paise = cfg.unit_price_milli("INR");
    let milli_cents = cfg.unit_price_milli("USD");
    let address_paise = cfg.address_price("INR");
    let address_cents = cfg.address_price("USD");
    let email_pack_size = cfg.email_pack_size;

    let (milli_price, address_price) = if default_currency.eq_ignore_ascii_case("usd") {
        (milli_cents, address_cents)
    } else {
        (milli_paise, address_paise)
    };

    // Public pricing page. Visitors aren't logged in, so the slider's
    // checkout button is replaced with a sign-in CTA. The ?c= query param
    // carries the chosen currency so the toggle is shareable. We keep the
    // visitor's UI language from `locale` and only swap the display
    // currency.
    let display_currency = if default_currency.eq_ignore_ascii_case("usd") {
        Currency::Usd
    } else {
        Currency::Inr
    };
    let locale = crate::locale::Locale {
        langid: locale.langid.clone(),
        currency: display_currency,
    };
    let signin_label = crate::i18n::t(&locale, "pricing-slider-cta-signin");
    let slider = crate::templates::credit_slider::slider_html(
        &locale,
        "",
        crate::templates::credit_slider::SliderMode::Preview {
            cta_href: "/auth/login",
            cta_label: &signin_label,
        },
        milli_price,
    );
    let per_reply = match locale.currency {
        Currency::Usd => format!("${:.3}", milli_price as f64 / 100_000.0),
        Currency::Inr => format!("₹{:.2}", milli_price as f64 / 100_000.0),
    };
    let address_price_label = format_money(address_price, &locale);
    let (inr_cls, usd_cls) = match locale.currency {
        Currency::Usd => ("btn ghost sm", "btn sm"),
        Currency::Inr => ("btn sm", "btn ghost sm"),
    };

    let nav = super::base::public_nav_html("pricing", &locale);
    let inr_label = crate::i18n::t(&locale, "pricing-currency-inr-label");
    let usd_label = crate::i18n::t(&locale, "pricing-currency-usd-label");
    let content = format!(
        r##"{nav}
<article class="legal">
  <div class="between">
    <h1 class="m-0">{per_reply} {headline_suffix}</h1>
    <div class="row gap-8" role="group" aria-label="Display currency">
      <a href="/pricing?c=inr" class="{inr_cls}" title="{inr_label}" aria-label="{inr_label}">&#x20B9;</a>
      <a href="/pricing?c=usd" class="{usd_cls}" title="{usd_label}" aria-label="{usd_label}">$</a>
    </div>
  </div>
  <p class="muted">{lead}</p>

  <div style="margin:24px 0">{slider}</div>

  <div class="card p-18">
    <div class="eyebrow mb-8">{credits_eyebrow}</div>
    <ul class="muted m-0">
      <li>{credits_li_1}</li>
      <li>{credits_li_2}</li>
      <li>{credits_li_3}</li>
    </ul>
  </div>

  <h2 style="margin-top:2rem">{email_h}</h2>
  <p class="muted">{email_body}</p>

  <div class="card p-18" style="margin:24px 0">
    <p class="m-0">{quota_prefix} <strong>{address_price}</strong> {quota_suffix}</p>
    <p class="muted" style="margin:8px 0 0">{billing_note}</p>
  </div>
</article>"##,
        nav = nav,
        inr_label = crate::helpers::html_escape(&inr_label),
        usd_label = crate::helpers::html_escape(&usd_label),
        headline_suffix = crate::i18n::t(&locale, "pricing-headline-prefix"),
        lead = crate::i18n::t(&locale, "pricing-lead"),
        credits_eyebrow = crate::i18n::t(&locale, "pricing-credits-eyebrow"),
        credits_li_1 = crate::i18n::t(&locale, "pricing-credits-li-1"),
        credits_li_2 = crate::i18n::t(&locale, "pricing-credits-li-2"),
        credits_li_3 = crate::i18n::t(&locale, "pricing-credits-li-3"),
        email_h = crate::i18n::t(&locale, "pricing-email-heading"),
        email_body = crate::i18n::t(&locale, "pricing-email-body"),
        quota_prefix = crate::i18n::t_args(
            &locale,
            "pricing-email-quota-prefix",
            &[("pack_size", &email_pack_size.to_string())],
        ),
        quota_suffix = crate::i18n::t(&locale, "pricing-email-quota-suffix"),
        billing_note = crate::i18n::t(&locale, "pricing-email-billing-note"),
        per_reply = per_reply,
        address_price = address_price_label,
        slider = slider,
        inr_cls = inr_cls,
        usd_cls = usd_cls,
    );

    let meta_inr = format!("₹{:.2}", milli_paise as f64 / 100_000.0);
    let meta_usd = format!("${:.3}", milli_cents as f64 / 100_000.0);
    let meta_addr_inr = format_money(address_paise, &crate::locale::Locale::default_inr());
    let meta_addr_usd = format_money(address_cents, &crate::locale::Locale::default_usd());
    let pack_size_str = email_pack_size.to_string();
    let meta_description = crate::i18n::t_args(
        &locale,
        "pricing-meta-description",
        &[
            ("inr", &meta_inr),
            ("usd", &meta_usd),
            ("addr_inr", &meta_addr_inr),
            ("addr_usd", &meta_addr_usd),
            ("pack_size", &pack_size_str),
        ],
    );
    base_html_with_meta(
        "Pricing - Concierge",
        &content,
        &PageMeta {
            description: meta_description,
            og_title: crate::i18n::t(&locale, "pricing-og-title"),
            og_type: "website",
        },
        &locale,
    )
}

#[cfg(test)]
mod pricing_tests {
    use super::*;

    fn cfg_with(milli_paise: i64, address_paise: i64, pack: i64) -> crate::storage::Pricing {
        use crate::storage::PricingConcept::*;
        let mut p = crate::storage::Pricing {
            email_pack_size: pack,
            amounts: std::collections::BTreeMap::new(),
        };
        // INR side as overridden by the caller.
        p.amounts
            .insert((UnitPriceMilli, "INR".into()), milli_paise);
        p.amounts
            .insert((AddressPrice, "INR".into()), address_paise);
        p.amounts.insert((VerificationAmount, "INR".into()), 100);
        // USD side picked to match the legacy default ratios used by tests
        // (~₹85/$1, but rounded to even numbers so asserts read cleanly).
        p.amounts.insert((UnitPriceMilli, "USD".into()), 250);
        p.amounts.insert((AddressPrice, "USD".into()), 200);
        p.amounts.insert((VerificationAmount, "USD".into()), 100);
        p
    }

    #[test]
    fn connect_html_email_help_shows_db_address_price() {
        let l = crate::locale::Locale::default_inr();
        // 5-pack at ₹150 (15_000 paise) per month → ~$1.76 with rate 8500.
        let cfg = cfg_with(10_000, 15_000, 5);
        let html = connect_html(
            false,
            false,
            &[],
            "demo",
            "example.com",
            "tenant_x",
            None,
            "https://example.test",
            &l,
            &cfg,
        );
        assert!(html.contains("₹150"), "wizard help missing INR: {html}");
        assert!(html.contains("$"), "wizard help missing USD: {html}");
        assert!(html.contains("5"), "wizard help missing pack size: {html}");
    }

    #[test]
    fn pricing_html_shows_db_inr_price() {
        let l = crate::locale::Locale::default_inr();
        // 25_000 milli-paise = ₹0.25 per reply.
        let cfg = cfg_with(25_000, 19_900, 5);
        let html = pricing_html("INR", &l, &cfg);
        assert!(html.contains("₹0.25"), "headline price missing: {html}");
        // Address row uses format_money: paise to ₹.
        assert!(html.contains("₹199"), "address inr missing");
    }

    #[test]
    fn pricing_html_usd_currency_uses_cents() {
        let l = crate::locale::Locale::default_inr();
        // 25_000 milli-paise / 8_500 paise-per-USD ≈ 294 milli-cents → "$0.003"
        let cfg = cfg_with(25_000, 19_900, 5);
        let html = pricing_html("usd", &l, &cfg);
        assert!(html.contains("$0.003"), "headline usd price: {html}");
    }

    #[test]
    fn launch_html_unverified_shows_verify_cta_and_disables_finish() {
        let l = crate::locale::Locale::default_inr();
        let html = launch_html(&[], "example.com", &l, "https://x.test", 25_000, false, 100);
        // Verify card surfaces the Razorpay-bound POST and the verify
        // copy + amount.
        assert!(
            html.contains("/admin/billing/verification"),
            "verify form action missing: {html}"
        );
        assert!(
            html.contains("Verify your account"),
            "verify headline missing"
        );
        assert!(html.contains("₹1"), "verify amount missing: {html}");
        // Finish is gated: the button rendered with our `disabled` attr.
        assert!(
            html.contains("/admin/wizard/complete\" hx-target=\"body\" disabled"),
            "finish button should carry the disabled attribute when unverified"
        );
    }

    #[test]
    fn launch_html_verified_enables_finish_and_hides_verify_card() {
        let l = crate::locale::Locale::default_inr();
        let html = launch_html(&[], "example.com", &l, "https://x.test", 25_000, true, 100);
        assert!(
            !html.contains("/admin/billing/verification"),
            "verify form should be hidden when verified"
        );
        assert!(
            html.contains("Ready to go live"),
            "verified status card should render"
        );
        // The Finish button renders with no disabled attribute.
        assert!(
            !html.contains("/admin/wizard/complete\" hx-target=\"body\" disabled"),
            "finish should not be disabled when verified"
        );
    }

    #[test]
    fn pricing_html_no_free_forever_copy() {
        let l = crate::locale::Locale::default_inr();
        let cfg = cfg_with(25_000, 19_900, 5);
        let html = pricing_html("INR", &l, &cfg);
        // Guard against regression: the user explicitly asked for these
        // claims to stay out of the rendered marketing copy.
        let banned = [
            "free, forever",
            "always free",
            "always <strong>free</strong>",
            "1 address free",
            "First address is free",
            "never expires",
        ];
        for needle in banned {
            assert!(
                !html.contains(needle),
                "rendered pricing page contains banned phrase {needle:?}: {html}"
            );
        }
    }
}
