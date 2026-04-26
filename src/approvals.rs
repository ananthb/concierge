//! Approval-queue persistence: turn an AI draft into a pending row in D1
//! plus a `ConversationContext` in KV, and (if configured) push a Discord
//! draft post for the tenant. The web routes and the digest cron read from
//! the same D1 row.

use worker::*;

use crate::discord;
use crate::helpers::{generate_id, now_iso};
use crate::storage::{get_discord_config_by_tenant, save_conversation_context};
use crate::types::{ConversationContext, InboundMessage, PendingApproval, QueueReason, ReplyRule};

/// Enqueue an AI draft for human approval. The caller has already paid the
/// AI credit and produced the draft text; this function only persists state
/// and (best-effort) posts to Discord.
///
/// The `id` lives across three places: the KV ConversationContext key, the
/// D1 pending_approvals.id, and the Discord button custom_id. A single
/// token threads decisions across surfaces.
pub async fn enqueue(
    env: &Env,
    msg: &InboundMessage,
    rule: &ReplyRule,
    draft: &str,
    reason: QueueReason,
) -> Result<()> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    let id = generate_id();

    // Discord channel id (if any) is decided up front so the saved
    // ConversationContext records where we posted to. For non-Discord-linked
    // tenants this stays empty: the web queue still works.
    let discord_channel_id = match get_discord_config_by_tenant(&kv, &msg.tenant_id).await? {
        Some(cfg) => cfg.approval_channel_id.unwrap_or_default(),
        None => String::new(),
    };

    let ctx = ConversationContext {
        id: id.clone(),
        discord_message_id: String::new(),
        discord_channel_id: discord_channel_id.clone(),
        origin_channel: msg.channel.clone(),
        origin_sender: msg.sender.clone(),
        origin_recipient: msg.recipient.clone(),
        tenant_id: msg.tenant_id.clone(),
        channel_account_id: msg.channel_account_id.clone(),
        reply_metadata: msg.raw_metadata.clone(),
        ai_draft: Some(draft.to_string()),
        created_at: now_iso(),
    };

    save_conversation_context(&kv, &ctx).await?;

    let inbound_preview = discord::truncate_inbound_preview(&msg.body);

    insert_pending_approval(
        &db,
        &PendingApproval {
            id: id.clone(),
            tenant_id: msg.tenant_id.clone(),
            channel: msg.channel.clone(),
            channel_account_id: msg.channel_account_id.clone(),
            rule_id: rule.id.clone(),
            rule_label: rule.label.clone(),
            sender: msg.sender.clone(),
            sender_name: msg.sender_name.clone(),
            inbound_preview: inbound_preview.clone(),
            draft: draft.to_string(),
            queue_reason: reason,
            status: crate::types::ApprovalStatus::Pending,
            created_at: ctx.created_at.clone(),
            decided_at: None,
            decided_by: None,
            edited: false,
            last_digest_at: None,
        },
    )
    .await?;

    // Discord post is best-effort: if the bot isn't configured or the post
    // fails, the row still lives in D1 and the web queue surfaces it.
    if !discord_channel_id.is_empty() {
        match discord::post_ai_draft(
            env,
            &ctx,
            msg.subject.as_deref(),
            &inbound_preview,
            Some(queue_reason_label(reason)),
            Some(&rule.label),
        )
        .await
        {
            Ok(message_id) => {
                let stamped = ConversationContext {
                    discord_message_id: message_id,
                    ..ctx
                };
                if let Err(e) = save_conversation_context(&kv, &stamped).await {
                    console_log!("Failed to stamp discord_message_id back to KV: {e:?}");
                }
            }
            Err(e) => console_log!(
                "Discord draft post failed for tenant {}: {e:?}",
                msg.tenant_id
            ),
        }
    }

    Ok(())
}

/// Read the current status of a pending approval row, or None if the row
/// doesn't exist (already deleted, never created, expired and pruned).
pub async fn get_status(db: &D1Database, id: &str) -> Result<Option<crate::types::ApprovalStatus>> {
    let stmt = db.prepare("SELECT status FROM pending_approvals WHERE id = ?");
    let row = stmt
        .bind(&[id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    let Some(row) = row else { return Ok(None) };
    let status = row.get("status").and_then(|v| v.as_str()).unwrap_or("");
    Ok(Some(parse_status(status)))
}

/// Fetch a single pending approval row by id, fully populated. Returns None
/// if the row doesn't exist.
pub async fn get_row(db: &D1Database, id: &str) -> Result<Option<PendingApproval>> {
    let stmt = db.prepare("SELECT * FROM pending_approvals WHERE id = ?");
    let row = stmt
        .bind(&[id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.map(|r| from_row(&r)))
}

/// List all pending (status='pending') approvals for a tenant, oldest first.
/// The list page renders these with Approve/Reject/Edit buttons.
pub async fn list_pending(db: &D1Database, tenant_id: &str) -> Result<Vec<PendingApproval>> {
    let stmt = db.prepare(
        "SELECT * FROM pending_approvals
         WHERE tenant_id = ? AND status = 'pending'
         ORDER BY created_at ASC",
    );
    let result = stmt.bind(&[tenant_id.into()])?.all().await?;
    let rows: Vec<serde_json::Value> = result.results()?;
    Ok(rows.iter().map(from_row).collect())
}

/// Update the draft text in place. Used by edit-and-approve to capture the
/// human-edited version so the audit row preserves what was actually sent.
pub async fn save_edited_draft(db: &D1Database, id: &str, draft: &str) -> Result<()> {
    let stmt =
        db.prepare("UPDATE pending_approvals SET draft = ? WHERE id = ? AND status = 'pending'");
    stmt.bind(&[draft.into(), id.into()])?.run().await?;
    Ok(())
}

/// Tenants who have at least one pending approval that hasn't yet been
/// rolled into a digest email. The digest sweep iterates this list, then
/// pulls each tenant's own undigested pending rows.
pub async fn tenants_with_undigested_pending(db: &D1Database) -> Result<Vec<String>> {
    let stmt = db.prepare(
        "SELECT DISTINCT tenant_id FROM pending_approvals
         WHERE status = 'pending' AND last_digest_at IS NULL",
    );
    let result = stmt.bind(&[])?.all().await?;
    let rows: Vec<serde_json::Value> = result.results()?;
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            r.get("tenant_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect())
}

/// Pending rows for one tenant that haven't been rolled into a digest yet.
/// Caller is the digest builder; cap the page length so a runaway tenant
/// doesn't blow up an email body.
pub async fn pending_for_digest(
    db: &D1Database,
    tenant_id: &str,
    limit: u32,
) -> Result<Vec<PendingApproval>> {
    let stmt = db.prepare(
        "SELECT * FROM pending_approvals
         WHERE tenant_id = ? AND status = 'pending' AND last_digest_at IS NULL
         ORDER BY created_at ASC
         LIMIT ?",
    );
    let result = stmt
        .bind(&[tenant_id.into(), wasm_bindgen::JsValue::from(limit as f64)])?
        .all()
        .await?;
    let rows: Vec<serde_json::Value> = result.results()?;
    Ok(rows.iter().map(from_row).collect())
}

/// Stamp a batch of approval ids as digested-now. Idempotent: rows already
/// digested keep their earlier timestamp.
pub async fn mark_digested(db: &D1Database, ids: &[String]) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    // D1 prepare doesn't support variadic IN(?), so emit a fresh statement
    // per id. The list is small (<= digest cap, ~50) so this is fine.
    for id in ids {
        let stmt = db
            .prepare("UPDATE pending_approvals SET last_digest_at = datetime('now') WHERE id = ?");
        stmt.bind(&[id.clone().into()])?.run().await?;
    }
    Ok(())
}

/// Mark pending approvals older than `cutoff_iso` as expired. Returns the
/// rows that were expired so the caller can restore credit per tenant.
pub async fn expire_stale(db: &D1Database, cutoff_iso: &str) -> Result<Vec<PendingApproval>> {
    let select = db.prepare(
        "SELECT * FROM pending_approvals
         WHERE status = 'pending' AND created_at < ?",
    );
    let result = select.bind(&[cutoff_iso.into()])?.all().await?;
    let rows: Vec<serde_json::Value> = result.results()?;
    let approvals: Vec<PendingApproval> = rows.iter().map(from_row).collect();

    if approvals.is_empty() {
        return Ok(approvals);
    }

    for row in &approvals {
        let stmt = db.prepare(
            "UPDATE pending_approvals
             SET status = 'expired',
                 decided_at = datetime('now'),
                 decided_by = 'expired'
             WHERE id = ? AND status = 'pending'",
        );
        if let Err(e) = stmt.bind(&[row.id.clone().into()])?.run().await {
            console_log!("Failed to expire approval {}: {e:?}", row.id);
        }
    }

    Ok(approvals)
}

/// Manually parse a D1 row into a PendingApproval. We can't use serde
/// directly because `Channel`'s wire form differs from `Channel::as_str()`
/// (the DB writes "whatsapp" but serde expects "whats_app"). The columns
/// are stable and known, so a hand-written shim is simpler than a custom
/// Deserialize impl on a project-wide enum.
fn from_row(row: &serde_json::Value) -> PendingApproval {
    use crate::types::{ApprovalStatus, Channel, QueueReason};
    let s = |k: &str| {
        row.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let opt = |k: &str| {
        row.get(k)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };
    let channel = match s("channel").as_str() {
        "whatsapp" => Channel::WhatsApp,
        "instagram" => Channel::Instagram,
        "discord" => Channel::Discord,
        _ => Channel::Email,
    };
    let queue_reason = match s("queue_reason").as_str() {
        "rule_always" => QueueReason::RuleAlways,
        "risk_money_word" => QueueReason::RiskMoneyWord,
        "risk_commitment" => QueueReason::RiskCommitment,
        "risk_persona_drift" => QueueReason::RiskPersonaDrift,
        _ => QueueReason::RiskLength,
    };
    let status = parse_status(&s("status"));
    let edited = row
        .get("edited")
        .and_then(|v| v.as_i64())
        .map(|n| n != 0)
        .unwrap_or(false);

    PendingApproval {
        id: s("id"),
        tenant_id: s("tenant_id"),
        channel,
        channel_account_id: s("channel_account_id"),
        rule_id: s("rule_id"),
        rule_label: s("rule_label"),
        sender: s("sender"),
        sender_name: opt("sender_name"),
        inbound_preview: s("inbound_preview"),
        draft: s("draft"),
        queue_reason,
        status,
        created_at: s("created_at"),
        decided_at: opt("decided_at"),
        decided_by: opt("decided_by"),
        edited,
        last_digest_at: opt("last_digest_at"),
    }
}

/// Update a pending row's terminal status. Idempotent: a row that's already
/// non-pending stays as it was, so the caller's race-loss check handles
/// double-clicks gracefully.
pub async fn mark_decided(
    db: &D1Database,
    id: &str,
    status: crate::types::ApprovalStatus,
    decided_by: &str,
    edited: bool,
) -> Result<()> {
    let stmt = db.prepare(
        "UPDATE pending_approvals
         SET status = ?, decided_at = datetime('now'), decided_by = ?, edited = ?
         WHERE id = ? AND status = 'pending'",
    );
    stmt.bind(&[
        approval_status_wire(status).into(),
        decided_by.into(),
        wasm_bindgen::JsValue::from(if edited { 1.0_f64 } else { 0.0_f64 }),
        id.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

fn parse_status(s: &str) -> crate::types::ApprovalStatus {
    match s {
        "approved" => crate::types::ApprovalStatus::Approved,
        "rejected" => crate::types::ApprovalStatus::Rejected,
        "expired" => crate::types::ApprovalStatus::Expired,
        _ => crate::types::ApprovalStatus::Pending,
    }
}

async fn insert_pending_approval(db: &D1Database, row: &PendingApproval) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO pending_approvals (
             id, tenant_id, channel, channel_account_id, rule_id, rule_label,
             sender, sender_name, inbound_preview, draft, queue_reason,
             status, created_at, edited
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    );
    stmt.bind(&[
        row.id.clone().into(),
        row.tenant_id.clone().into(),
        row.channel.as_str().into(),
        row.channel_account_id.clone().into(),
        row.rule_id.clone().into(),
        row.rule_label.clone().into(),
        row.sender.clone().into(),
        row.sender_name
            .clone()
            .map(wasm_bindgen::JsValue::from)
            .unwrap_or(wasm_bindgen::JsValue::null()),
        row.inbound_preview.clone().into(),
        row.draft.clone().into(),
        queue_reason_wire(row.queue_reason).into(),
        approval_status_wire(row.status).into(),
        row.created_at.clone().into(),
        wasm_bindgen::JsValue::from(if row.edited { 1.0_f64 } else { 0.0_f64 }),
    ])?
    .run()
    .await?;
    Ok(())
}

fn queue_reason_wire(reason: QueueReason) -> &'static str {
    match reason {
        QueueReason::RuleAlways => "rule_always",
        QueueReason::RiskLength => "risk_length",
        QueueReason::RiskMoneyWord => "risk_money_word",
        QueueReason::RiskCommitment => "risk_commitment",
        QueueReason::RiskPersonaDrift => "risk_persona_drift",
    }
}

/// Short human-readable label used in Discord embeds and digest emails.
pub fn queue_reason_label(reason: QueueReason) -> &'static str {
    match reason {
        QueueReason::RuleAlways => "Rule asks for approval",
        QueueReason::RiskLength => "Unusual length",
        QueueReason::RiskMoneyWord => "Mentions money",
        QueueReason::RiskCommitment => "Makes a commitment",
        QueueReason::RiskPersonaDrift => "Off-topic for persona",
    }
}

fn approval_status_wire(status: crate::types::ApprovalStatus) -> &'static str {
    match status {
        crate::types::ApprovalStatus::Pending => "pending",
        crate::types::ApprovalStatus::Approved => "approved",
        crate::types::ApprovalStatus::Rejected => "rejected",
        crate::types::ApprovalStatus::Expired => "expired",
    }
}
