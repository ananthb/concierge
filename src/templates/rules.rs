//! Templates for `/admin/rules/{channel}/{id}`: rules list page and the
//! shared add/edit form. The default rule is rendered at the top of the
//! list (always present, mandatory) and uses a separate edit URL since its
//! matcher is fixed to `Default`.

use crate::handlers::admin_rules::ChannelRef;
use crate::helpers::html_escape;
use crate::types::{default_match_threshold, ReplyConfig, ReplyMatcher, ReplyResponse, ReplyRule};

use super::base::{app_shell, base_html};
use super::HASH;

pub fn rules_list_html(cfg: &ReplyConfig, channel: &ChannelRef<'_>, base_url: &str) -> String {
    let rules_base = channel.rules_base(base_url);
    let back = channel.back_url(base_url);
    let channel_label = html_escape(channel.label());

    let last_idx = cfg.rules.len().saturating_sub(1);
    let rule_rows: String = cfg
        .rules
        .iter()
        .enumerate()
        .map(|(i, rule)| rule_row_html(rule, i, last_idx, &rules_base))
        .collect();

    let empty_note = if cfg.rules.is_empty() {
        r#"<p class="muted ta-center" style="padding:18px">No rules yet. Add one below or rely on the default reply.</p>"#
    } else {
        ""
    };

    let default_summary = render_default_summary(&cfg.default_rule, &rules_base);

    let body = format!(
        r##"<div class="page-pad">
  <p><a href="{back}" class="btn ghost sm">&larr; {channel_label}</a></p>
  <h1 class="display-sm m-0 mb-4">Reply rules</h1>
  <p class="muted mb-16">Inbound messages are checked against these rules in order. The first match fires; if nothing matches, the default reply runs.</p>

  <h2 class="display-xs mb-8">Routing rules</h2>
  <div class="card p-0 mb-12" style="overflow:hidden">
    {rule_rows}{empty_note}
  </div>
  <div class="row gap-8 mb-24">
    <a class="btn primary" href="{rules_base}/new">+ Add rule</a>
  </div>

  <h2 class="display-xs mb-8">Default reply</h2>
  <div class="card p-22 mb-24">
    {default_summary}
  </div>
</div>"##,
        back = back,
        channel_label = channel_label,
        rule_rows = rule_rows,
        empty_note = empty_note,
        rules_base = rules_base,
        default_summary = default_summary,
    );

    let page = app_shell(&body, "Rules", base_url);
    base_html("Reply rules - Concierge", &page)
}

fn rule_row_html(rule: &ReplyRule, idx: usize, last_idx: usize, rules_base: &str) -> String {
    let label = html_escape(&rule.label);
    let matcher_chip = match &rule.matcher {
        ReplyMatcher::Default => r#"<span class="chip">default</span>"#.to_string(),
        ReplyMatcher::StaticText { keywords } => format!(
            r#"<span class="chip">keywords</span> <span class="muted fs-13">{}</span>"#,
            html_escape(&keywords.join(", "))
        ),
        ReplyMatcher::Prompt { description, .. } => format!(
            r#"<span class="chip">prompt</span> <span class="muted fs-13">{}</span>"#,
            html_escape(description)
        ),
    };
    let response_chip = match &rule.response {
        ReplyResponse::Canned { .. } => r#"<span class="chip">canned</span>"#,
        ReplyResponse::Prompt { .. } => r#"<span class="chip ok">AI</span>"#,
    };

    let id = html_escape(&rule.id);
    let up_btn = if idx > 0 {
        format!(
            r#"<button class="btn ghost icon" hx-post="{rules_base}/{id}/move/up" hx-swap="none" title="Move up">&uarr;</button>"#
        )
    } else {
        r#"<span class="btn ghost icon" style="visibility:hidden">&uarr;</span>"#.to_string()
    };
    let down_btn = if idx < last_idx {
        format!(
            r#"<button class="btn ghost icon" hx-post="{rules_base}/{id}/move/down" hx-swap="none" title="Move down">&darr;</button>"#
        )
    } else {
        r#"<span class="btn ghost icon" style="visibility:hidden">&darr;</span>"#.to_string()
    };

    format!(
        r##"<div class="rule-row" id="rule-{id}" style="display:grid;grid-template-columns:auto 1fr auto;gap:12px;align-items:center;padding:14px 18px;border-bottom:1px solid var(--border)">
  <div class="row gap-2">{up_btn}{down_btn}</div>
  <div>
    <div class="row gap-8" style="align-items:center;flex-wrap:wrap">
      <strong>{label}</strong>
      {response_chip}
    </div>
    <div class="mt-4">{matcher_chip}</div>
  </div>
  <div class="row gap-6">
    <a class="btn ghost sm" href="{rules_base}/{id}">Edit</a>
    <button class="btn ghost sm text-warn"
      hx-delete="{rules_base}/{id}"
      hx-confirm="Delete this rule?"
      hx-target="{HASH}rule-{id}" hx-swap="outerHTML">Delete</button>
  </div>
</div>"##,
        rules_base = rules_base,
        id = id,
        label = label,
        matcher_chip = matcher_chip,
        response_chip = response_chip,
        up_btn = up_btn,
        down_btn = down_btn,
        HASH = HASH,
    )
}

fn render_default_summary(default_rule: &ReplyRule, rules_base: &str) -> String {
    let label = html_escape(&default_rule.label);
    let (kind_chip, text) = match &default_rule.response {
        ReplyResponse::Canned { text } => ("canned", text.as_str()),
        ReplyResponse::Prompt { text } => ("AI", text.as_str()),
    };
    format!(
        r#"<div class="row gap-8 mb-8" style="align-items:center;flex-wrap:wrap">
  <strong>{label}</strong>
  <span class="chip">{kind_chip}</span>
</div>
<pre class="mono fs-12 m-0 mb-12" style="white-space:pre-wrap">{text}</pre>
<a class="btn ghost sm" href="{rules_base}/default">Edit default</a>"#,
        label = label,
        kind_chip = kind_chip,
        text = html_escape(text),
        rules_base = rules_base,
    )
}

/// Title shown on the edit form. The default rule has fixed text since its
/// matcher can't change; regular rules use their current label.
pub fn rule_form_title(rule: &ReplyRule) -> &str {
    if rule.id == "default" {
        "Edit default reply"
    } else if rule.label.is_empty() {
        "Edit rule"
    } else {
        &rule.label
    }
}

pub fn rule_form_html(
    channel: &ChannelRef<'_>,
    existing: Option<&ReplyRule>,
    base_url: &str,
    title: impl AsRef<str>,
) -> String {
    let rules_base = channel.rules_base(base_url);
    let title = html_escape(title.as_ref());

    let is_default = existing.map(|r| r.id == "default").unwrap_or(false);
    let is_edit = existing.is_some();

    // Shadow values: keep all field inputs across mode-switches so the user
    // doesn't lose typing if they flip matcher kind.
    let initial = existing.cloned().unwrap_or_else(|| ReplyRule {
        id: String::new(),
        label: String::new(),
        matcher: ReplyMatcher::StaticText {
            keywords: Vec::new(),
        },
        response: ReplyResponse::Canned {
            text: String::new(),
        },
    });

    let label_val = html_escape(&initial.label);

    let (matcher_kind, keywords_val, description_val, threshold_val) = match &initial.matcher {
        ReplyMatcher::Default => (
            "default",
            String::new(),
            String::new(),
            default_match_threshold(),
        ),
        ReplyMatcher::StaticText { keywords } => (
            "static_text",
            keywords.join(", "),
            String::new(),
            default_match_threshold(),
        ),
        ReplyMatcher::Prompt {
            description,
            threshold,
            ..
        } => ("prompt", String::new(), description.clone(), *threshold),
    };

    let (response_kind, response_text) = match &initial.response {
        ReplyResponse::Canned { text } => ("canned", text.clone()),
        ReplyResponse::Prompt { text } => ("prompt", text.clone()),
    };

    let action_url = if is_default {
        format!("{rules_base}/default")
    } else if let Some(ex) = existing {
        format!("{rules_base}/{}", html_escape(&ex.id))
    } else {
        rules_base.clone()
    };

    let method_attr = if is_edit { "hx-put" } else { "hx-post" };

    // Default rule edit hides the matcher block (matcher is fixed).
    let matcher_block = if is_default {
        String::new()
    } else {
        format!(
            r##"<div class="form-group">
  <label class="eyebrow lbl">Match by</label>
  <div class="row gap-12 mb-12">
    <label class="row gap-6"><input type="radio" name="matcher_kind" value="static_text" x-model="matcherKind"> Keywords</label>
    <label class="row gap-6"><input type="radio" name="matcher_kind" value="prompt" x-model="matcherKind"> Prompt (AI intent)</label>
  </div>

  <div x-show="matcherKind === 'static_text'" x-cloak>
    <label class="eyebrow lbl">Keywords (comma- or newline-separated)</label>
    <textarea class="textarea mono" name="keywords" rows="2" placeholder="hours, open, closed">{keywords_val}</textarea>
    <p class="muted fs-12 mt-4">Case-insensitive. Matches if the inbound message contains any of these.</p>
  </div>

  <div x-show="matcherKind === 'prompt'" x-cloak>
    <label class="eyebrow lbl">Description</label>
    <input class="input" name="description" maxlength="200" value="{description_val}" placeholder="asks about hours">
    <p class="muted fs-12 mt-4">Describe the kind of message that should match. We embed this and compare against incoming messages.</p>
    <label class="eyebrow lbl mt-12">Threshold: <span class="mono" x-text="threshold.toFixed(2)"></span></label>
    <input type="range" min="0.5" max="0.95" step="0.01" name="threshold" x-model.number="threshold" style="width:100%;accent-color:var(--accent)">
    <p class="muted fs-12 mt-4">Higher = stricter match. Default 0.72.</p>
  </div>
</div>"##,
            keywords_val = html_escape(&keywords_val),
            description_val = html_escape(&description_val),
        )
    };

    let body = format!(
        r##"<div class="page-pad" x-data="{x_data}" hx-ext="json-enc">
  <p><a href="{rules_base}" class="btn ghost sm">&larr; All rules</a></p>
  <h1 class="display-sm m-0 mb-16">{title}</h1>

  <form class="card p-22" {method_attr}="{action_url}" hx-target="body" hx-swap="innerHTML">
    <div class="form-group">
      <label class="eyebrow lbl">Label</label>
      <input class="input" name="label" maxlength="80" value="{label_val}" placeholder="Pricing questions" required>
    </div>

    {matcher_block}

    <div class="form-group">
      <label class="eyebrow lbl">Respond with</label>
      <div class="row gap-12 mb-12">
        <label class="row gap-6"><input type="radio" name="response_kind" value="canned" x-model="responseKind"> Canned text (free, no AI)</label>
        <label class="row gap-6"><input type="radio" name="response_kind" value="prompt" x-model="responseKind"> AI prompt (1 credit per reply)</label>
      </div>
      <textarea class="textarea" name="response_text" rows="5" maxlength="2000" placeholder="Hi! Here's what we recommend..." required>{response_text}</textarea>
      <p class="muted fs-12 mt-4" x-show="responseKind === 'prompt'" x-cloak>This text is appended to your persona prompt and sent to the LLM.</p>
    </div>

    <div class="row gap-8 mt-16" style="justify-content:flex-end">
      <a class="btn ghost" href="{rules_base}">Cancel</a>
      <button class="btn primary" type="submit">Save</button>
    </div>
  </form>
</div>"##,
        rules_base = rules_base,
        title = title,
        method_attr = method_attr,
        action_url = action_url,
        label_val = label_val,
        matcher_block = matcher_block,
        response_text = html_escape(&response_text),
        x_data = build_x_data(matcher_kind, response_kind, threshold_val),
    );

    let page = app_shell(&body, "Rules", base_url);
    base_html("Edit rule - Concierge", &page)
}

fn build_x_data(matcher_kind: &str, response_kind: &str, threshold: f32) -> String {
    format!(
        "{{ matcherKind: '{}', responseKind: '{}', threshold: {} }}",
        matcher_kind, response_kind, threshold
    )
}
