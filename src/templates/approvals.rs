//! `/admin/approvals`: list of pending AI drafts awaiting human approval.
//!
//! The list refreshes via HTMX polling (`hx-trigger="every 5s"`); fine for
//! Phase 2. A future Phase 2.5 may swap the polling block for an SSE
//! endpoint backed by a per-tenant Durable Object.

use crate::approvals::queue_reason_label;
use crate::helpers::html_escape;
use crate::locale::Locale;
use crate::types::{PendingApproval, QueueReason};

use super::base::{app_shell, base_html};
use super::HASH;

pub fn approvals_page_html(rows: &[PendingApproval], base_url: &str, locale: &Locale) -> String {
    let list = approvals_list_inner_html(rows);
    // SSE listener fires on every push; HTMX refetches /list when it
    // hears the event. Polling every 30s is the belt-and-suspenders
    // fallback if the SSE connection drops between reconnect retries.
    let body = format!(
        r##"<div class="page-pad" hx-ext="sse" sse-connect="/admin/approvals/stream">
  <h1 class="display-sm m-0 mb-4">Approvals</h1>
  <p class="muted mb-16">AI drafts that paused for review. Approve, reject, or edit and send.</p>

  <div id="approvals-list"
    role="region"
    aria-live="polite"
    aria-atomic="false"
    hx-get="/admin/approvals/list"
    hx-trigger="sse:approval-changed, every 30s"
    hx-swap="outerHTML">
    {list}
  </div>
</div>"##,
        list = list,
    );
    let page = app_shell(&body, "Approvals", base_url, locale);
    base_html("Approvals - Concierge", &page, locale)
}

/// Render just the inner list. Returned by GET `/admin/approvals/list`,
/// which the SSE event triggers (with a 30s polling fallback). The
/// `outerHTML` swap replaces the wrapping div so the next tick rebinds.
pub fn approvals_list_html(rows: &[PendingApproval]) -> String {
    let inner = approvals_list_inner_html(rows);
    format!(
        r##"<div id="approvals-list"
  role="region"
  aria-live="polite"
  aria-atomic="false"
  hx-get="/admin/approvals/list"
  hx-trigger="sse:approval-changed, every 30s"
  hx-swap="outerHTML">
  {inner}
</div>"##
    )
}

fn approvals_list_inner_html(rows: &[PendingApproval]) -> String {
    if rows.is_empty() {
        return r##"<div class="card p-22 ta-center">
  <p class="muted m-0">Nothing waiting on you. We'll surface drafts here when an AI reply needs your review.</p>
</div>"##
            .to_string();
    }

    let rendered: String = rows.iter().map(approval_row_html).collect();
    format!(r##"<div class="card p-0" style="overflow:hidden">{rendered}</div>"##)
}

pub fn approval_row_html(row: &PendingApproval) -> String {
    let id = html_escape(&row.id);
    let sender = html_escape(&row.sender);
    let rule_label = html_escape(&row.rule_label);
    let inbound = html_escape(&row.inbound_preview);
    let draft = html_escape(&row.draft);
    let channel = row.channel.label();
    let reason_chip = reason_chip(row.queue_reason);
    let created = html_escape(short_date(&row.created_at));

    format!(
        r##"<div class="approval-row" id="approval-{id}" x-data="{{ editing: false, draft: '' }}"
  style="padding:18px;border-bottom:1px solid var(--border)">
  <div class="row gap-8 mb-4" style="align-items:center;flex-wrap:wrap">
    <strong>{sender}</strong>
    <span class="chip">{channel}</span>
    {reason_chip}
    <span class="muted fs-12">{created}</span>
  </div>
  <div class="muted fs-13 mb-8" style="white-space:pre-wrap">From rule: {rule_label}</div>

  <details class="mb-8">
    <summary class="muted fs-12">Original message</summary>
    <pre class="mono fs-12 mt-4" style="white-space:pre-wrap">{inbound}</pre>
  </details>

  <div x-show="!editing">
    <pre class="mono fs-13 m-0 mb-12" style="white-space:pre-wrap">{draft}</pre>
    <div class="row gap-8">
      <button class="btn primary"
        hx-post="/admin/approvals/{id}/approve"
        hx-target="{HASH}approval-{id}"
        hx-swap="outerHTML">Approve and send</button>
      <button class="btn ghost"
        hx-post="/admin/approvals/{id}/reject"
        hx-target="{HASH}approval-{id}"
        hx-swap="outerHTML"
        hx-confirm="Reject this draft? The credit will be refunded.">Reject</button>
      <button class="btn ghost" type="button"
        @click="editing = true; draft = $el.parentElement.previousElementSibling.textContent">Edit</button>
    </div>
  </div>

  <div x-show="editing" x-cloak hx-ext="json-enc">
    <textarea class="textarea" name="draft" rows="5" maxlength="2000" x-model="draft"></textarea>
    <div class="row gap-8 mt-8">
      <button class="btn primary" type="button"
        hx-post="/admin/approvals/{id}/edit"
        hx-vals='js:{{draft: document.querySelector("#approval-{id} textarea").value}}'
        hx-target="{HASH}approval-{id}"
        hx-swap="outerHTML">Send edit</button>
      <button class="btn ghost" type="button" @click="editing = false">Cancel</button>
    </div>
  </div>
</div>"##,
        HASH = HASH,
    )
}

fn reason_chip(reason: QueueReason) -> String {
    let label = queue_reason_label(reason);
    match reason {
        QueueReason::RuleAlways => format!(r##"<span class="chip">{label}</span>"##),
        _ => format!(r##"<span class="chip warn">{label}</span>"##),
    }
}

/// Short rendering of an ISO timestamp: just the date+time prefix. Good
/// enough for an approval list where age matters more than precise seconds.
fn short_date(iso: &str) -> &str {
    iso.get(..16).unwrap_or(iso)
}
