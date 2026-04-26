//! `/admin/approvals/*` routes: list pending AI drafts and act on them.
//!
//! All routes are authenticated and CSRF-protected by the `handle_admin`
//! dispatcher. Approve / edit / reject mutate the D1 row, the KV
//! `ConversationContext`, and (on send) call into the channel layer to push
//! the reply back to the customer.

use worker::*;

use crate::approvals;
use crate::billing;
use crate::channel;
use crate::helpers::{generate_id, now_iso};
use crate::storage::{
    delete_conversation_context, get_conversation_context, get_tenant, save_message,
};
use crate::templates::approvals::{approval_row_html, approvals_list_html, approvals_page_html};
use crate::types::{ApprovalStatus, Channel, ConversationContext, PendingApproval};

pub async fn handle_approvals(
    mut req: Request,
    env: Env,
    path: &str,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;
    let method = req.method();
    let _ = base_url;

    // Strip the "/admin/approvals" prefix; what remains is "" for the
    // index page or "/{id}/{action}" for per-approval actions.
    let rest = path.strip_prefix("/admin/approvals").unwrap_or("");

    match (method, rest) {
        (Method::Get, "" | "/") => {
            let rows = approvals::list_pending(&db, tenant_id).await?;
            Response::from_html(approvals_page_html(&rows, base_url))
        }

        (Method::Get, "/list") => {
            let rows = approvals::list_pending(&db, tenant_id).await?;
            Response::from_html(approvals_list_html(&rows))
        }

        (Method::Post, action_path) if action_path.starts_with('/') => {
            let parts: Vec<&str> = action_path.trim_start_matches('/').split('/').collect();
            let (id, action) = match parts.as_slice() {
                [id, action] => ((*id).to_string(), *action),
                _ => return Response::error("Not Found", 404),
            };

            let row = match approvals::get_row(&db, &id).await? {
                Some(r) if r.tenant_id == tenant_id => r,
                _ => return Response::error("Approval not found", 404),
            };

            if row.status != ApprovalStatus::Pending {
                return resolved_row_html(&row, "Already handled.");
            }

            match action {
                "approve" => approve(&env, &kv, &db, tenant_id, row, None).await,
                "reject" => reject(&kv, &db, tenant_id, row).await,
                "edit" => {
                    let form: serde_json::Value =
                        req.json().await.unwrap_or(serde_json::Value::Null);
                    let edited: String = form
                        .get("draft")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .chars()
                        .take(2000)
                        .collect();
                    if edited.trim().is_empty() {
                        return Response::from_html(
                            r#"<div class="error">Edited reply can't be empty.</div>"#,
                        );
                    }
                    approve(&env, &kv, &db, tenant_id, row, Some(edited)).await
                }
                _ => Response::error("Not Found", 404),
            }
        }

        _ => Response::error("Not Found", 404),
    }
}

/// Send the draft (optionally edited) and mark the row approved.
async fn approve(
    env: &Env,
    kv: &kv::KvStore,
    db: &D1Database,
    tenant_id: &str,
    row: PendingApproval,
    edit: Option<String>,
) -> Result<Response> {
    let ctx = match get_conversation_context(kv, &row.id).await? {
        Some(c) => c,
        None => return Response::from_html(r#"<div class="error">Conversation expired.</div>"#),
    };

    let edited = edit.is_some();
    let draft_text = match edit.clone() {
        Some(text) => {
            if let Err(e) = approvals::save_edited_draft(db, &row.id, &text).await {
                console_log!("Failed to persist edited draft: {e:?}");
            }
            text
        }
        None => row.draft.clone(),
    };

    let subject = email_subject_for(&ctx);

    if let Err(e) = channel::send_reply(
        &ctx.origin_channel,
        env,
        &ctx.reply_metadata,
        &ctx.origin_sender,
        &draft_text,
        subject,
    )
    .await
    {
        console_log!("Approval send error: {e:?}");
        return Response::from_html(format!(r#"<div class="error">Failed to send: {}</div>"#, e));
    }

    let decided_by = format!("web:{}", web_decider(db, tenant_id).await);
    if let Err(e) =
        approvals::mark_decided(db, &row.id, ApprovalStatus::Approved, &decided_by, edited).await
    {
        console_log!("Failed to mark approval row decided: {e:?}");
    }

    let _ = save_message(
        db,
        &generate_id(),
        &ctx.origin_channel,
        "outbound",
        &ctx.origin_recipient,
        &ctx.origin_sender,
        &ctx.tenant_id,
        &ctx.channel_account_id,
        Some("ai_approved"),
    )
    .await;

    let _ = delete_conversation_context(kv, &row.id).await;

    let mut updated = row;
    updated.status = ApprovalStatus::Approved;
    updated.decided_at = Some(now_iso());
    updated.decided_by = Some(decided_by);
    updated.edited = edited;
    if let Some(text) = edit {
        updated.draft = text;
    }
    resolved_row_html(&updated, if edited { "Edited and sent." } else { "Sent." })
}

async fn reject(
    kv: &kv::KvStore,
    db: &D1Database,
    tenant_id: &str,
    row: PendingApproval,
) -> Result<Response> {
    let decided_by = format!("web:{}", web_decider(db, tenant_id).await);
    if let Err(e) =
        approvals::mark_decided(db, &row.id, ApprovalStatus::Rejected, &decided_by, false).await
    {
        console_log!("Failed to mark rejection row decided: {e:?}");
    }

    if let Err(e) = billing::restore_credit(db, &row.tenant_id).await {
        console_log!("Failed to restore credit on rejection: {e:?}");
    }

    let _ = save_message(
        db,
        &generate_id(),
        &row.channel,
        "outbound",
        &row.sender,
        &row.sender,
        &row.tenant_id,
        &row.channel_account_id,
        Some("ai_rejected"),
    )
    .await;

    let _ = delete_conversation_context(kv, &row.id).await;

    let mut updated = row;
    updated.status = ApprovalStatus::Rejected;
    updated.decided_at = Some(now_iso());
    updated.decided_by = Some(decided_by);
    resolved_row_html(&updated, "Rejected. Credit refunded.")
}

/// Render the row in its terminal state with a one-line note. HTMX swaps
/// this in place of the live row, so the user sees what happened without a
/// page reload.
fn resolved_row_html(row: &PendingApproval, note: &str) -> Result<Response> {
    let sender = crate::helpers::html_escape(&row.sender);
    let id = crate::helpers::html_escape(&row.id);
    let html = format!(
        r##"<div class="approval-row" id="approval-{id}" style="padding:14px 18px;border-bottom:1px solid var(--border)">
  <div class="row gap-8" style="align-items:center;flex-wrap:wrap">
    <strong>{sender}</strong>
    <span class="chip ok">{note}</span>
  </div>
</div>"##,
        note = crate::helpers::html_escape(note),
    );
    let _ = approval_row_html; // keep reference to silence unused-import lints
    Response::from_html(html)
}

fn email_subject_for(ctx: &ConversationContext) -> Option<&'static str> {
    if matches!(ctx.origin_channel, Channel::Email) {
        Some("Re: your message")
    } else {
        None
    }
}

/// Best-effort lookup of the tenant's login email for the audit row. Falls
/// back to tenant_id if the lookup fails.
async fn web_decider(db: &D1Database, tenant_id: &str) -> String {
    match get_tenant(db, tenant_id).await {
        Ok(Some(t)) => t.email,
        _ => tenant_id.to_string(),
    }
}
