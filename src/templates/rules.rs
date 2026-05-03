//! Templates for `/admin/rules/{channel}/{id}`: rules list page and the
//! shared add/edit form. The default rule is rendered at the top of the
//! list (always present, mandatory) and uses a separate edit URL since its
//! matcher is fixed to `Default`.

use crate::handlers::admin_rules::ChannelRef;
use crate::helpers::html_escape;
use crate::i18n::t;
use crate::locale::Locale;
use crate::types::{
    default_match_threshold, ApprovalPolicy, ReplyConfig, ReplyMatcher, ReplyResponse, ReplyRule,
};

use super::base::{app_shell, base_html};
use super::HASH;

pub fn rules_list_html(
    cfg: &ReplyConfig,
    channel: &ChannelRef<'_>,
    base_url: &str,
    locale: &Locale,
) -> String {
    let rules_base = channel.rules_base(base_url);
    let back = channel.back_url(base_url);
    let channel_label = html_escape(channel.label());

    let last_idx = cfg.rules.len().saturating_sub(1);
    let rule_rows: String = cfg
        .rules
        .iter()
        .enumerate()
        .map(|(i, rule)| rule_row_html(rule, i, last_idx, &rules_base, locale))
        .collect();

    let empty_note = if cfg.rules.is_empty() {
        format!(
            r#"<p class="muted ta-center" style="padding:18px">{}</p>"#,
            t(locale, "admin-rules-list-empty"),
        )
    } else {
        String::new()
    };

    let default_summary = render_default_summary(&cfg.default_rule, &rules_base, locale);

    let body = format!(
        r##"<div class="page-pad">
  <p><a href="{back}" class="btn ghost sm">{back_prefix} {channel_label}</a></p>
  <h1 class="display-sm m-0 mb-4">{h1}</h1>
  <p class="muted mb-16">{lead}</p>

  <h2 class="display-xs mb-8">{routing_h2}</h2>
  <div class="card p-0 mb-12" style="overflow:hidden">
    {rule_rows}{empty_note}
  </div>
  <div class="row gap-8 mb-24">
    <a class="btn primary" href="{rules_base}/new">{add}</a>
  </div>

  <h2 class="display-xs mb-8">{default_h2}</h2>
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
        back_prefix = t(locale, "admin-rules-list-back-prefix"),
        h1 = t(locale, "admin-rules-list-h1"),
        lead = t(locale, "admin-rules-list-lead"),
        routing_h2 = t(locale, "admin-rules-list-routing-h2"),
        add = t(locale, "admin-rules-list-add"),
        default_h2 = t(locale, "admin-rules-list-default-h2"),
    );

    let page = app_shell(&body, "Rules", base_url, locale);
    base_html(&t(locale, "admin-rules-title"), &page, locale)
}

fn rule_row_html(
    rule: &ReplyRule,
    idx: usize,
    last_idx: usize,
    rules_base: &str,
    locale: &Locale,
) -> String {
    let label = html_escape(&rule.label);
    let chip_default = t(locale, "admin-rules-chip-default");
    let chip_keywords = t(locale, "admin-rules-chip-keywords");
    let chip_prompt = t(locale, "admin-rules-chip-prompt");
    let chip_canned = t(locale, "admin-rules-chip-canned");
    let chip_ai = t(locale, "admin-rules-chip-ai");
    let chip_auto = t(locale, "admin-rules-chip-auto");
    let chip_always = t(locale, "admin-rules-chip-always-asks");
    let chip_no_gate = t(locale, "admin-rules-chip-no-gate");
    let matcher_chip = match &rule.matcher {
        ReplyMatcher::Default => format!(r#"<span class="chip">{chip_default}</span>"#),
        ReplyMatcher::Keyword { keywords } => format!(
            r#"<span class="chip">{chip_keywords}</span> <span class="muted fs-13">{}</span>"#,
            html_escape(&keywords.join(", "))
        ),
        ReplyMatcher::Prompt { description, .. } => format!(
            r#"<span class="chip">{chip_prompt}</span> <span class="muted fs-13">{}</span>"#,
            html_escape(description)
        ),
    };
    let response_chip = match &rule.response {
        ReplyResponse::Canned { .. } => format!(r#"<span class="chip">{chip_canned}</span>"#),
        ReplyResponse::Prompt { .. } => format!(r#"<span class="chip ok">{chip_ai}</span>"#),
    };
    let approval_chip = match (&rule.response, &rule.approval) {
        // Approval policy is irrelevant for canned text (no AI draft).
        (ReplyResponse::Canned { .. }, _) => String::new(),
        (ReplyResponse::Prompt { .. }, ApprovalPolicy::Auto) => {
            format!(r#"<span class="chip">{chip_auto}</span>"#)
        }
        (ReplyResponse::Prompt { .. }, ApprovalPolicy::Always) => {
            format!(r#"<span class="chip warn">{chip_always}</span>"#)
        }
        (ReplyResponse::Prompt { .. }, ApprovalPolicy::NoGate { .. }) => {
            format!(r#"<span class="chip warn">{chip_no_gate}</span>"#)
        }
    };

    let id = html_escape(&rule.id);
    let up_aria = html_escape(&t(locale, "admin-rules-row-move-up"));
    let down_aria = html_escape(&t(locale, "admin-rules-row-move-down"));
    let up_btn = if idx > 0 {
        format!(
            r#"<button class="btn ghost icon" hx-post="{rules_base}/{id}/move/up" hx-swap="none" title="{up_aria}" aria-label="{up_aria}">&uarr;</button>"#
        )
    } else {
        r#"<span class="btn ghost icon" style="visibility:hidden" aria-hidden="true">&uarr;</span>"#
            .to_string()
    };
    let down_btn = if idx < last_idx {
        format!(
            r#"<button class="btn ghost icon" hx-post="{rules_base}/{id}/move/down" hx-swap="none" title="{down_aria}" aria-label="{down_aria}">&darr;</button>"#
        )
    } else {
        r#"<span class="btn ghost icon" style="visibility:hidden" aria-hidden="true">&darr;</span>"#
            .to_string()
    };

    let edit_label = t(locale, "admin-rules-row-edit");
    let delete_label = t(locale, "admin-rules-row-delete");
    let delete_confirm = html_escape(&t(locale, "admin-rules-row-delete-confirm"));
    format!(
        r##"<div class="rule-row" id="rule-{id}" style="display:grid;grid-template-columns:auto 1fr auto;gap:12px;align-items:center;padding:14px 18px;border-bottom:1px solid var(--border)">
  <div class="row gap-2">{up_btn}{down_btn}</div>
  <div>
    <div class="row gap-8" style="align-items:center;flex-wrap:wrap">
      <strong>{label}</strong>
      {response_chip}
      {approval_chip}
    </div>
    <div class="mt-4">{matcher_chip}</div>
  </div>
  <div class="row gap-6">
    <a class="btn ghost sm" href="{rules_base}/{id}">{edit}</a>
    <button class="btn ghost sm text-warn"
      hx-delete="{rules_base}/{id}"
      hx-confirm="{confirm}"
      hx-target="{HASH}rule-{id}" hx-swap="outerHTML">{delete}</button>
  </div>
</div>"##,
        rules_base = rules_base,
        id = id,
        label = label,
        matcher_chip = matcher_chip,
        response_chip = response_chip,
        approval_chip = approval_chip,
        up_btn = up_btn,
        down_btn = down_btn,
        HASH = HASH,
        edit = edit_label,
        delete = delete_label,
        confirm = delete_confirm,
    )
}

fn render_default_summary(default_rule: &ReplyRule, rules_base: &str, locale: &Locale) -> String {
    let label = html_escape(&default_rule.label);
    let (kind_chip, text) = match &default_rule.response {
        ReplyResponse::Canned { text } => (t(locale, "admin-rules-chip-canned"), text.as_str()),
        ReplyResponse::Prompt { text } => (t(locale, "admin-rules-chip-ai"), text.as_str()),
    };
    format!(
        r#"<div class="row gap-8 mb-8" style="align-items:center;flex-wrap:wrap">
  <strong>{label}</strong>
  <span class="chip">{kind_chip}</span>
</div>
<pre class="mono fs-12 m-0 mb-12" style="white-space:pre-wrap">{text}</pre>
<a class="btn ghost sm" href="{rules_base}/default">{edit_default}</a>"#,
        label = label,
        kind_chip = kind_chip,
        text = html_escape(text),
        rules_base = rules_base,
        edit_default = t(locale, "admin-rules-default-edit"),
    )
}

/// Title shown on the edit form. The default rule has fixed text since its
/// matcher can't change; regular rules use their current label. Falls back
/// to localized strings via Cow so the caller can use `&` against either
/// shape.
pub fn rule_form_title<'a>(rule: &'a ReplyRule, locale: &Locale) -> std::borrow::Cow<'a, str> {
    if rule.id == "default" {
        std::borrow::Cow::Owned(t(locale, "admin-rules-form-title-default"))
    } else if rule.label.is_empty() {
        std::borrow::Cow::Owned(t(locale, "admin-rules-form-title-edit-fallback"))
    } else {
        std::borrow::Cow::Borrowed(&rule.label)
    }
}

pub fn rule_form_html(
    channel: &ChannelRef<'_>,
    existing: Option<&ReplyRule>,
    base_url: &str,
    title: impl AsRef<str>,
    allow_no_gate: bool,
    locale: &Locale,
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
        matcher: ReplyMatcher::Keyword {
            keywords: Vec::new(),
        },
        response: ReplyResponse::Canned {
            text: String::new(),
        },
        approval: crate::types::ApprovalPolicy::default(),
    });

    let label_val = html_escape(&initial.label);
    let approval_kind = match &initial.approval {
        ApprovalPolicy::Auto => "auto",
        ApprovalPolicy::Always => "always",
        ApprovalPolicy::NoGate { .. } => "no_gate",
    };
    // If a rule was previously saved as `NoGate`, the acceptance is already
    // on file: the modal must NOT re-prompt unless the user changes the
    // radio away and back.
    let already_accepted = matches!(initial.approval, ApprovalPolicy::NoGate { .. });

    let (matcher_kind, keywords_val, description_val, threshold_val) = match &initial.matcher {
        ReplyMatcher::Default => (
            "default",
            String::new(),
            String::new(),
            default_match_threshold(),
        ),
        ReplyMatcher::Keyword { keywords } => (
            "keyword",
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
  <label class="eyebrow lbl" id="rule-match-by-label">{match_by}</label>
  <div class="row gap-12 mb-12" role="radiogroup" aria-labelledby="rule-match-by-label">
    <label class="row gap-6"><input type="radio" name="matcher_kind" value="keyword" x-model="matcherKind"> {match_keyword}</label>
    <label class="row gap-6"><input type="radio" name="matcher_kind" value="prompt" x-model="matcherKind"> {match_prompt}</label>
  </div>

  <div x-show="matcherKind === 'keyword'" x-cloak :aria-hidden="matcherKind !== 'keyword'">
    <label for="rule-keywords" class="eyebrow lbl">{kw_label}</label>
    <textarea id="rule-keywords" class="textarea mono" name="keywords" rows="2" placeholder="{kw_ph}">{keywords_val}</textarea>
    <p class="muted fs-12 mt-4">{kw_help}</p>
  </div>

  <div x-show="matcherKind === 'prompt'" x-cloak :aria-hidden="matcherKind !== 'prompt'">
    <label for="rule-description" class="eyebrow lbl">{desc_label}</label>
    <input id="rule-description" class="input" name="description" maxlength="200" value="{description_val}" placeholder="{desc_ph}">
    <p class="muted fs-12 mt-4">{desc_help}</p>
    <label for="rule-threshold" class="eyebrow lbl mt-12">{th_prefix} <span class="mono" x-text="threshold.toFixed(2)"></span></label>
    <input id="rule-threshold" type="range" min="0.5" max="0.95" step="0.01" name="threshold" x-model.number="threshold" style="width:100%;accent-color:var(--accent)">
    <p class="muted fs-12 mt-4">{th_help}</p>
  </div>
</div>"##,
            keywords_val = html_escape(&keywords_val),
            description_val = html_escape(&description_val),
            match_by = t(locale, "admin-rules-form-match-by"),
            match_keyword = t(locale, "admin-rules-form-match-keyword"),
            match_prompt = t(locale, "admin-rules-form-match-prompt"),
            kw_label = t(locale, "admin-rules-form-keywords"),
            kw_ph = t(locale, "admin-rules-form-keywords-placeholder"),
            kw_help = t(locale, "admin-rules-form-keywords-help"),
            desc_label = t(locale, "admin-rules-form-description"),
            desc_ph = t(locale, "admin-rules-form-description-placeholder"),
            desc_help = t(locale, "admin-rules-form-description-help"),
            th_prefix = t(locale, "admin-rules-form-threshold-prefix"),
            th_help = t(locale, "admin-rules-form-threshold-help"),
        )
    };

    let approval_block = approval_block_html(approval_kind, allow_no_gate, locale);
    let no_gate_modal = no_gate_modal_html(locale);

    let body = format!(
        r##"<div class="page-pad" x-data="{x_data}" hx-ext="json-enc">
  <p><a href="{rules_base}" class="btn ghost sm">{back}</a></p>
  <h1 class="display-sm m-0 mb-16">{title}</h1>

  <form class="card p-22" {method_attr}="{action_url}" hx-target="body" hx-swap="innerHTML" x-ref="form">
    <div class="form-group">
      <label for="rule-label" class="eyebrow lbl">{lbl}</label>
      <input id="rule-label" class="input" name="label" maxlength="80" value="{label_val}" placeholder="{lbl_ph}" required aria-required="true">
    </div>

    {matcher_block}

    <div class="form-group">
      <label class="eyebrow lbl" id="rule-respond-with-label">{respond_with}</label>
      <div class="row gap-12 mb-12" role="radiogroup" aria-labelledby="rule-respond-with-label">
        <label class="row gap-6"><input type="radio" name="response_kind" value="canned" x-model="responseKind"> {resp_canned}</label>
        <label class="row gap-6"><input type="radio" name="response_kind" value="prompt" x-model="responseKind"> {resp_prompt}</label>
      </div>
      <label for="rule-response-text" class="sr-only">{resp_sr}</label>
      <textarea id="rule-response-text" class="textarea" name="response_text" rows="5" maxlength="2000" placeholder="{resp_ph}" required aria-required="true">{response_text}</textarea>
      <p class="muted fs-12 mt-4" x-show="responseKind === 'prompt'" x-cloak :aria-hidden="responseKind !== 'prompt'">{resp_help}</p>
      <p class="muted fs-12 mt-4" x-show="responseKind === 'prompt'" x-cloak :aria-hidden="responseKind !== 'prompt'">{rule_envelope_note}</p>
    </div>

    {approval_block}

    <input type="hidden" name="approval_kind" :value="approvalKind">
    <input type="hidden" name="no_gate_acceptance" :value="noGateConfirmed ? 'true' : 'false'">

    <div class="row gap-8 mt-16" style="justify-content:flex-end">
      <a class="btn ghost" href="{rules_base}">{cancel}</a>
      <button class="btn primary" type="submit"
        @click="if (responseKind === 'prompt' && approvalKind === 'no_gate' && !noGateConfirmed) {{ $event.preventDefault(); noGateModalOpen = true; }}">{save}</button>
    </div>
  </form>

  {no_gate_modal}
</div>"##,
        rules_base = rules_base,
        title = title,
        method_attr = method_attr,
        action_url = action_url,
        label_val = label_val,
        matcher_block = matcher_block,
        response_text = html_escape(&response_text),
        approval_block = approval_block,
        no_gate_modal = no_gate_modal,
        x_data = build_x_data(
            matcher_kind,
            response_kind,
            threshold_val,
            approval_kind,
            already_accepted,
        ),
        back = t(locale, "admin-rules-form-back"),
        lbl = t(locale, "admin-rules-form-label"),
        lbl_ph = t(locale, "admin-rules-form-label-placeholder"),
        respond_with = t(locale, "admin-rules-form-respond-with"),
        resp_canned = t(locale, "admin-rules-form-response-canned"),
        resp_prompt = t(locale, "admin-rules-form-response-prompt"),
        resp_sr = t(locale, "admin-rules-form-response-text-sr"),
        resp_ph = t(locale, "admin-rules-form-response-placeholder"),
        resp_help = t(locale, "admin-rules-form-response-help"),
        rule_envelope_note = t(locale, "admin-rules-form-envelope-note"),
        cancel = t(locale, "admin-rules-form-cancel"),
        save = t(locale, "admin-rules-form-save"),
    );

    let page = app_shell(&body, "Rules", base_url, locale);
    base_html(&t(locale, "admin-rules-edit-title"), &page, locale)
}

fn approval_block_html(approval_kind: &str, allow_no_gate: bool, locale: &Locale) -> String {
    let no_gate_radio = if allow_no_gate {
        format!(
            r##"<label class="row gap-6" style="opacity:0.85">
          <input type="radio" name="approval_kind_visible" value="no_gate" x-model="approvalKind"
            @change="if (approvalKind === 'no_gate' && !noGateConfirmed) {{ noGateModalOpen = true }}">
          <span><strong>{lbl}</strong><br><span class="muted fs-12">{detail}</span></span>
        </label>"##,
            lbl = t(locale, "admin-rules-approval-no-gate"),
            detail = t(locale, "admin-rules-approval-no-gate-detail"),
        )
    } else {
        String::new()
    };

    let _ = approval_kind;
    format!(
        r##"<div class="form-group" x-show="responseKind === 'prompt'" x-cloak :aria-hidden="responseKind !== 'prompt'">
  <label class="eyebrow lbl" id="rule-approval-label">{eyebrow}</label>
  <div class="col gap-8 mb-4" role="radiogroup" aria-labelledby="rule-approval-label">
    <label class="row gap-6">
      <input type="radio" name="approval_kind_visible" value="auto" x-model="approvalKind">
      <span><strong>{auto_lbl}</strong><br><span class="muted fs-12">{auto_detail}</span></span>
    </label>
    <label class="row gap-6">
      <input type="radio" name="approval_kind_visible" value="always" x-model="approvalKind">
      <span><strong>{always_lbl}</strong><br><span class="muted fs-12">{always_detail}</span></span>
    </label>
    {no_gate_radio}
  </div>
</div>"##,
        eyebrow = t(locale, "admin-rules-approval-eyebrow"),
        auto_lbl = t(locale, "admin-rules-approval-auto"),
        auto_detail = t(locale, "admin-rules-approval-auto-detail"),
        always_lbl = t(locale, "admin-rules-approval-always"),
        always_detail = t(locale, "admin-rules-approval-always-detail"),
    )
}

fn no_gate_modal_html(locale: &Locale) -> String {
    format!(
        r##"<div x-show="noGateModalOpen" x-cloak
  role="dialog" aria-modal="true" aria-labelledby="no-gate-title"
  x-trap.noscroll.inert="noGateModalOpen"
  @keydown.escape.window="noGateModalOpen = false; noGateAck1 = false; noGateAck2 = false; approvalKind = 'auto'"
  style="position:fixed;inset:0;background:rgba(0,0,0,0.4);display:flex;align-items:center;justify-content:center;z-index:1000">
  <div class="card p-22" style="max-width:560px;width:90%">
    <h2 id="no-gate-title" class="display-xs mb-8">{h2}</h2>
    <p class="muted fs-13 mb-12">{lead}</p>
    <ul class="muted fs-13 mb-12" style="margin-left:18px">
      <li>{li_1}</li>
      <li>{li_2}</li>
      <li>{li_3}</li>
      <li>{li_4}</li>
    </ul>
    <p class="fs-13 mb-12">{disclaimer}</p>
    <label class="row gap-6 mb-8"><input type="checkbox" x-model="noGateAck1"> {ack_1}</label>
    <label class="row gap-6 mb-12"><input type="checkbox" x-model="noGateAck2"> {ack_2}</label>
    <div class="row gap-8" style="justify-content:flex-end">
      <button type="button" class="btn ghost"
        @click="noGateModalOpen = false; noGateAck1 = false; noGateAck2 = false; approvalKind = 'auto'">{cancel}</button>
      <button type="button" class="btn primary"
        :disabled="!(noGateAck1 && noGateAck2)"
        @click="if (noGateAck1 && noGateAck2) {{ noGateConfirmed = true; noGateModalOpen = false; $refs.form.requestSubmit(); }}">{confirm}</button>
    </div>
  </div>
</div>"##,
        h2 = t(locale, "admin-rules-modal-h2"),
        lead = t(locale, "admin-rules-modal-lead"),
        li_1 = t(locale, "admin-rules-modal-li-1"),
        li_2 = t(locale, "admin-rules-modal-li-2"),
        li_3 = t(locale, "admin-rules-modal-li-3"),
        li_4 = t(locale, "admin-rules-modal-li-4"),
        disclaimer = t(locale, "admin-rules-modal-disclaimer"),
        ack_1 = t(locale, "admin-rules-modal-ack-1"),
        ack_2 = t(locale, "admin-rules-modal-ack-2"),
        cancel = t(locale, "admin-rules-modal-cancel"),
        confirm = t(locale, "admin-rules-modal-confirm"),
    )
}

fn build_x_data(
    matcher_kind: &str,
    response_kind: &str,
    threshold: f32,
    approval_kind: &str,
    already_accepted: bool,
) -> String {
    format!(
        "{{ matcherKind: '{}', responseKind: '{}', threshold: {}, approvalKind: '{}', noGateConfirmed: {}, noGateModalOpen: false, noGateAck1: {}, noGateAck2: {} }}",
        matcher_kind,
        response_kind,
        threshold,
        approval_kind,
        already_accepted,
        already_accepted,
        already_accepted,
    )
}
