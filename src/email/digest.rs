//! Approval-queue digest emails.
//!
//! Once every 15 minutes the cron triggers `sweep`, which:
//! 1. Expires pending rows older than 24h and refunds the credit. The
//!    customer never sees a stale reply, and the tenant isn't charged.
//! 2. Lists tenants with undigested pending approvals, checks each tenant's
//!    `DigestCadence` against the current hour:minute, and emails the
//!    tenant a digest with deep links into `/admin/approvals`.
//!
//! The email is a notification with deep links: tenants click through to
//! the web page to actually approve. Reply-to-approve is out of scope.

use worker::*;

use crate::approvals;
use crate::approvals::queue_reason_label;
use crate::billing;
use crate::email::send::{send_outbound, OutboundEmail};
use crate::helpers::{generate_id, html_escape};
use crate::storage::{get_onboarding, get_tenant, save_message};
use crate::types::{MessageAction, MessageDirection, PendingApproval};

/// Hard expiry for pending approvals: anything older than 24h is dropped.
/// 24h is short enough that a queued draft can't be sent against a customer
/// who's long since moved on.
const EXPIRY_HOURS: i64 = 24;

/// Cap on how many rows go into a single digest email body. Anything beyond
/// this stays pending and lands in the next digest.
const DIGEST_PAGE_SIZE: u32 = 50;

/// Run one tick of the 15-minute approval-digest cron.
pub async fn sweep(env: &Env) -> Result<()> {
    let db = env.d1("DB")?;
    let kv = env.kv("KV")?;

    expire_pass(&db).await;

    let now = js_sys::Date::new_0();
    let hour = now.get_utc_hours();
    let minute = now.get_utc_minutes();

    let tenants = approvals::tenants_with_undigested_pending(&db).await?;
    if tenants.is_empty() {
        return Ok(());
    }

    let base_url = env
        .var("PUBLIC_BASE_URL")
        .ok()
        .map(|v| v.to_string())
        .filter(|s| !s.is_empty());
    let email_domain = env
        .var("EMAIL_DOMAIN")
        .ok()
        .map(|v| v.to_string())
        .filter(|s| !s.is_empty());
    let (Some(base_url), Some(email_domain)) = (base_url, email_domain) else {
        console_log!("Digest sweep skipped: PUBLIC_BASE_URL and EMAIL_DOMAIN must both be set");
        return Ok(());
    };
    let from_addr = format!("noreply@{email_domain}");

    for tenant_id in tenants {
        if let Err(e) = sweep_one(
            env, &db, &kv, &tenant_id, hour, minute, &base_url, &from_addr,
        )
        .await
        {
            console_log!("Digest sweep for tenant {tenant_id} failed: {e:?}");
        }
    }

    Ok(())
}

async fn sweep_one(
    env: &Env,
    db: &D1Database,
    kv: &kv::KvStore,
    tenant_id: &str,
    hour: u32,
    minute: u32,
    base_url: &str,
    from_addr: &str,
) -> Result<()> {
    let onboarding = get_onboarding(kv, tenant_id).await?;
    let notif = onboarding.notifications;
    if !notif.approval_email {
        return Ok(());
    }
    if !notif.approval_email_cadence.is_due_at(hour, minute) {
        return Ok(());
    }

    let rows = approvals::pending_for_digest(db, tenant_id, DIGEST_PAGE_SIZE).await?;
    if rows.is_empty() {
        return Ok(());
    }

    let recipient = match get_tenant(db, tenant_id).await? {
        Some(t) => t.email,
        None => {
            console_log!("Digest sweep: tenant {tenant_id} not found, skipping");
            return Ok(());
        }
    };

    let outbound = build_digest_email(&rows, &recipient, base_url, from_addr);
    send_outbound(env, &outbound).await?;

    let ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
    approvals::mark_digested(db, &ids).await?;

    Ok(())
}

async fn expire_pass(db: &D1Database) {
    let cutoff = expiry_cutoff_iso();
    match approvals::expire_stale(db, &cutoff).await {
        Ok(rows) => {
            for row in rows {
                if let Err(e) = billing::restore_credit(db, &row.tenant_id).await {
                    console_log!("Failed to restore credit on expiry: {e:?}");
                }
                let _ = save_message(
                    db,
                    &generate_id(),
                    &row.channel,
                    MessageDirection::Outbound,
                    &row.sender,
                    &row.sender,
                    &row.tenant_id,
                    &row.channel_account_id,
                    Some(MessageAction::AiExpired),
                    None,
                )
                .await;
                console_log!(
                    "Expired stale approval {} for tenant {}",
                    row.id,
                    row.tenant_id
                );
            }
        }
        Err(e) => console_log!("expire_stale failed: {e:?}"),
    }
}

fn expiry_cutoff_iso() -> String {
    let now = js_sys::Date::new_0();
    let cutoff_ms = now.get_time() - (EXPIRY_HOURS as f64) * 3600.0 * 1000.0;
    let cutoff = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(cutoff_ms));
    cutoff
        .to_iso_string()
        .as_string()
        .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string())
}

fn build_digest_email(
    rows: &[PendingApproval],
    recipient: &str,
    base_url: &str,
    from_addr: &str,
) -> OutboundEmail {
    let count = rows.len();
    let plural = if count == 1 { "" } else { "s" };
    let subject = format!("{count} reply draft{plural} waiting for review");

    let html_items: String = rows
        .iter()
        .map(|r| {
            format!(
                r#"<li style="margin-bottom:18px">
  <div><strong>From {sender}</strong> via {channel} &middot; {reason}</div>
  <div style="color:#777;font-size:13px;margin:4px 0">{preview}</div>
  <div style="font-family:monospace;font-size:13px;white-space:pre-wrap;margin:6px 0">{draft}</div>
  <a href="{base_url}/admin/approvals/{id}" style="display:inline-block;padding:8px 14px;background:#5865F2;color:#fff;border-radius:4px;text-decoration:none">Review draft</a>
</li>"#,
                sender = html_escape(&r.sender),
                channel = r.channel.label(),
                reason = html_escape(queue_reason_label(r.queue_reason)),
                preview = html_escape(&r.inbound_preview),
                draft = html_escape(&r.draft),
                base_url = base_url,
                id = html_escape(&r.id),
            )
        })
        .collect();
    let html = format!(
        r#"<!doctype html>
<html><body style="font-family:-apple-system,BlinkMacSystemFont,sans-serif;max-width:640px;margin:0 auto;padding:24px">
<h1 style="font-size:22px;margin:0 0 16px">{count} reply draft{plural} waiting</h1>
<p style="color:#555">These AI replies paused for your review. Click through to approve, edit, or reject.</p>
<ol style="padding-left:20px">{html_items}</ol>
<p style="color:#999;font-size:12px;margin-top:32px">You're getting this because email approvals are turned on for your tenant. <a href="{base_url}/admin/wizard/notifications">Change cadence</a>.</p>
</body></html>"#,
        count = count,
        plural = plural,
    );

    let text_items: String = rows
        .iter()
        .map(|r| {
            format!(
                "- From {sender} via {channel} ({reason})\n  {preview}\n  Draft: {draft}\n  Review: {base_url}/admin/approvals/{id}\n",
                sender = r.sender,
                channel = r.channel.label(),
                reason = queue_reason_label(r.queue_reason),
                preview = r.inbound_preview,
                draft = r.draft,
                base_url = base_url,
                id = r.id,
            )
        })
        .collect();
    let text = format!(
        "{count} reply draft{plural} waiting for review.\n\n{text_items}\nChange cadence: {base_url}/admin/wizard/notifications\n",
        count = count,
        plural = plural,
    );

    OutboundEmail {
        from: from_addr.to_string(),
        to: recipient.to_string(),
        subject,
        text: Some(text),
        html: Some(html),
        reply_to: None,
        cc: vec![],
        bcc: vec![],
        headers: vec![],
    }
}
