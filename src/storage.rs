use wasm_bindgen::JsValue;
use worker::*;

use crate::helpers::{now_epoch_secs, now_iso};
use crate::types::{
    Channel, CreditEntry, InboundMessage, MessageAction, MessageDirection, OnboardingState,
    ReplyConfig, SafetyState, Session, Tenant, TenantBilling, WhatsAppAccount,
};

// ============================================================================
// Tenant D1 Operations
// ============================================================================

fn row_to_tenant(row: &serde_json::Value) -> Tenant {
    Tenant {
        id: row
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        email: row
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        name: row
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        facebook_id: row
            .get("facebook_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        plan: row
            .get("plan")
            .and_then(|v| v.as_str())
            .and_then(crate::types::Plan::from_wire)
            .unwrap_or_default(),
        locale: row
            .get("locale")
            .and_then(|v| v.as_str())
            .unwrap_or("en-IN")
            .to_string(),
        currency: row
            .get("currency")
            .and_then(|v| v.as_str())
            .map(crate::locale::Currency::parse)
            .unwrap_or_default(),
        created_at: row
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        updated_at: row
            .get("updated_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    }
}

pub async fn get_tenant(db: &D1Database, id: &str) -> Result<Option<Tenant>> {
    let row = db
        .prepare("SELECT * FROM tenants WHERE id = ?")
        .bind(&[id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.as_ref().map(row_to_tenant))
}

pub async fn get_tenant_by_email(db: &D1Database, email: &str) -> Result<Option<Tenant>> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let row = db
        .prepare("SELECT * FROM tenants WHERE email = ? COLLATE NOCASE")
        .bind(&[trimmed.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.as_ref().map(row_to_tenant))
}

pub async fn get_tenant_by_facebook_id(
    db: &D1Database,
    facebook_id: &str,
) -> Result<Option<Tenant>> {
    let row = db
        .prepare("SELECT * FROM tenants WHERE facebook_id = ?")
        .bind(&[facebook_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.as_ref().map(row_to_tenant))
}

pub async fn save_tenant(db: &D1Database, tenant: &Tenant) -> Result<()> {
    let email_val: JsValue = if tenant.email.trim().is_empty() {
        JsValue::NULL
    } else {
        tenant.email.trim().into()
    };
    let name_val: JsValue = match &tenant.name {
        Some(n) => n.as_str().into(),
        None => JsValue::NULL,
    };
    let fb_val: JsValue = match &tenant.facebook_id {
        Some(fb) => fb.as_str().into(),
        None => JsValue::NULL,
    };
    db.prepare(
        "INSERT INTO tenants (id, email, name, facebook_id, plan, locale, currency, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           email = excluded.email,
           name = excluded.name,
           facebook_id = excluded.facebook_id,
           plan = excluded.plan,
           locale = excluded.locale,
           currency = excluded.currency,
           updated_at = excluded.updated_at",
    )
    .bind(&[
        tenant.id.as_str().into(),
        email_val,
        name_val,
        fb_val,
        tenant.plan.as_str().into(),
        tenant.locale.as_str().into(),
        tenant.currency.as_str().into(),
        tenant.created_at.as_str().into(),
        tenant.updated_at.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Persist the Meta Facebook user ID on a tenant so `/data-deletion` can
/// locate and delete their data on Meta callback.
pub async fn set_tenant_facebook_id(
    db: &D1Database,
    tenant_id: &str,
    facebook_id: &str,
) -> Result<()> {
    db.prepare("UPDATE tenants SET facebook_id = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(&[facebook_id.into(), tenant_id.into()])?
        .run()
        .await?;
    Ok(())
}

// ============================================================================
// Session D1 Operations
// ============================================================================

pub async fn save_session(
    db: &D1Database,
    token: &str,
    tenant_id: &str,
    ttl_seconds: u64,
) -> Result<()> {
    let now = now_epoch_secs();
    let expires_at = now + (ttl_seconds as i64);
    let created_at = now_iso();
    let _ = db
        .prepare("DELETE FROM sessions WHERE expires_at <= ?")
        .bind(&[JsValue::from(now as f64)])?
        .run()
        .await;
    db.prepare(
        "INSERT INTO sessions (token, tenant_id, expires_at, created_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(token) DO UPDATE SET
           tenant_id = excluded.tenant_id,
           expires_at = excluded.expires_at,
           created_at = excluded.created_at",
    )
    .bind(&[
        token.into(),
        tenant_id.into(),
        JsValue::from(expires_at as f64),
        created_at.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

pub async fn get_session(db: &D1Database, token: &str) -> Result<Option<String>> {
    let now = now_epoch_secs();
    let row = db
        .prepare("SELECT tenant_id FROM sessions WHERE token = ? AND expires_at > ?")
        .bind(&[token.into(), JsValue::from(now as f64)])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.and_then(|r| {
        r.get("tenant_id")
            .and_then(|v| v.as_str())
            .map(String::from)
    }))
}

pub async fn delete_session(db: &D1Database, token: &str) -> Result<()> {
    db.prepare("DELETE FROM sessions WHERE token = ?")
        .bind(&[token.into()])?
        .run()
        .await?;
    Ok(())
}

// ============================================================================
// WhatsApp Signup State D1 Operations
// ============================================================================

pub async fn save_signup_state(
    db: &D1Database,
    state: &str,
    tenant_id: &str,
    ttl_seconds: u64,
) -> Result<()> {
    let now = now_epoch_secs();
    let expires_at = now + (ttl_seconds as i64);
    let _ = db
        .prepare("DELETE FROM signup_states WHERE expires_at <= ?")
        .bind(&[JsValue::from(now as f64)])?
        .run()
        .await;
    db.prepare(
        "INSERT INTO signup_states (state, tenant_id, expires_at)
         VALUES (?, ?, ?)
         ON CONFLICT(state) DO UPDATE SET
           tenant_id = excluded.tenant_id,
           expires_at = excluded.expires_at",
    )
    .bind(&[
        state.into(),
        tenant_id.into(),
        JsValue::from(expires_at as f64),
    ])?
    .run()
    .await?;
    Ok(())
}

pub async fn take_signup_state(db: &D1Database, state: &str) -> Result<Option<String>> {
    let now = now_epoch_secs();
    let row = db
        .prepare("SELECT tenant_id FROM signup_states WHERE state = ? AND expires_at > ?")
        .bind(&[state.into(), JsValue::from(now as f64)])?
        .first::<serde_json::Value>(None)
        .await?;
    if row.is_some() {
        let _ = db
            .prepare("DELETE FROM signup_states WHERE state = ?")
            .bind(&[state.into()])?
            .run()
            .await;
    }
    Ok(row.and_then(|r| {
        r.get("tenant_id")
            .and_then(|v| v.as_str())
            .map(String::from)
    }))
}

// ============================================================================
// WhatsApp Account D1 Operations
// ============================================================================

fn row_to_whatsapp_account(row: &serde_json::Value) -> WhatsAppAccount {
    let created_at = row
        .get("created_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let reply_json = row
        .get("reply_json")
        .and_then(|v| v.as_str())
        .unwrap_or("{}");
    let auto_reply: ReplyConfig = serde_json::from_str(reply_json).unwrap_or_default();
    let canned_safety_json = row
        .get("canned_safety_json")
        .and_then(|v| v.as_str())
        .unwrap_or("{}");
    let canned_safety: SafetyState = serde_json::from_str(canned_safety_json).unwrap_or_default();

    WhatsAppAccount {
        id: row
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        tenant_id: row
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        name: row
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        phone_number: row
            .get("display_phone_number")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        phone_number_id: row
            .get("phone_number_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        waba_id: row
            .get("waba_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from),
        auto_reply,
        canned_safety,
        created_at: created_at.clone(),
        updated_at: created_at,
    }
}

pub async fn get_whatsapp_account(db: &D1Database, id: &str) -> Result<Option<WhatsAppAccount>> {
    let row = db
        .prepare("SELECT * FROM whatsapp_accounts WHERE id = ?")
        .bind(&[id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.as_ref().map(row_to_whatsapp_account))
}

pub async fn save_whatsapp_account(db: &D1Database, account: &WhatsAppAccount) -> Result<()> {
    let reply_json = serde_json::to_string(&account.auto_reply)
        .map_err(|e| Error::from(format!("JSON error: {e}")))?;
    let canned_safety_json = serde_json::to_string(&account.canned_safety)
        .map_err(|e| Error::from(format!("JSON error: {e}")))?;
    let waba_val: JsValue = match &account.waba_id {
        Some(w) if !w.is_empty() => w.as_str().into(),
        _ => JsValue::NULL,
    };
    let created_at = if account.created_at.is_empty() {
        now_iso()
    } else {
        account.created_at.clone()
    };

    db.prepare(
        "INSERT INTO whatsapp_accounts (id, tenant_id, name, phone_number_id, display_phone_number, waba_id, reply_json, canned_safety_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           tenant_id = excluded.tenant_id,
           name = excluded.name,
           phone_number_id = excluded.phone_number_id,
           display_phone_number = excluded.display_phone_number,
           waba_id = excluded.waba_id,
           reply_json = excluded.reply_json,
           canned_safety_json = excluded.canned_safety_json",
    )
    .bind(&[
        account.id.as_str().into(),
        account.tenant_id.as_str().into(),
        account.name.as_str().into(),
        account.phone_number_id.as_str().into(),
        account.phone_number.as_str().into(),
        waba_val,
        reply_json.as_str().into(),
        canned_safety_json.as_str().into(),
        created_at.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

pub async fn delete_whatsapp_account(db: &D1Database, tenant_id: &str, id: &str) -> Result<()> {
    db.prepare("DELETE FROM whatsapp_accounts WHERE id = ? AND tenant_id = ?")
        .bind(&[id.into(), tenant_id.into()])?
        .run()
        .await?;
    Ok(())
}

pub async fn list_whatsapp_accounts(
    db: &D1Database,
    tenant_id: &str,
) -> Result<Vec<WhatsAppAccount>> {
    let rows = db
        .prepare("SELECT * FROM whatsapp_accounts WHERE tenant_id = ? ORDER BY created_at DESC")
        .bind(&[tenant_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;
    Ok(rows.iter().map(row_to_whatsapp_account).collect())
}

pub async fn get_whatsapp_account_by_phone(
    db: &D1Database,
    phone_number_id: &str,
) -> Result<Option<String>> {
    let row = db
        .prepare("SELECT id FROM whatsapp_accounts WHERE phone_number_id = ?")
        .bind(&[phone_number_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.and_then(|r| r.get("id").and_then(|v| v.as_str()).map(String::from)))
}

pub async fn get_whatsapp_account_by_waba(
    db: &D1Database,
    waba_id: &str,
) -> Result<Option<String>> {
    if waba_id.trim().is_empty() {
        return Ok(None);
    }
    let row = db
        .prepare(
            "SELECT id FROM whatsapp_accounts WHERE waba_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(&[waba_id.trim().into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.and_then(|r| r.get("id").and_then(|v| v.as_str()).map(String::from)))
}

// ============================================================================
// Channel Reply Config Abstraction
// ============================================================================

/// Reply config + safety verdict for any channel account.
pub struct ChannelReplyInfo {
    pub config: ReplyConfig,
    pub canned_safety: SafetyState,
}

/// Look up a channel account's `ReplyConfig` and canned-reply safety verdict.
pub async fn get_channel_reply_info(
    db: &D1Database,
    channel: &Channel,
    account_id: &str,
) -> Result<Option<ChannelReplyInfo>> {
    match channel {
        Channel::WhatsApp => {
            Ok(get_whatsapp_account(db, account_id)
                .await?
                .map(|a| ChannelReplyInfo {
                    config: a.auto_reply,
                    canned_safety: a.canned_safety,
                }))
        }
    }
}

/// Look up a channel account's `ReplyConfig` without leaking channel-specific
/// storage lookups into the pipeline.
pub async fn get_channel_reply_config(
    db: &D1Database,
    channel: &Channel,
    account_id: &str,
) -> Result<Option<ReplyConfig>> {
    Ok(get_channel_reply_info(db, channel, account_id)
        .await?
        .map(|info| info.config))
}

// ============================================================================
// Delete All Tenant Data
// ============================================================================

pub async fn delete_tenant_data(db: &D1Database, tenant_id: &str) -> Result<()> {
    for table in &[
        "whatsapp_accounts",
        "tenant_onboarding",
        "sessions",
        "signup_states",
        "messages",
        "tenant_billing",
    ] {
        let query = format!("DELETE FROM {} WHERE tenant_id = ?", table);
        let stmt = db.prepare(&query);
        if let Err(e) = stmt.bind(&[tenant_id.into()])?.run().await {
            console_log!("Failed to delete from {}: {:?}", table, e);
        }
    }

    let prefix = format!("convsession:{tenant_id}:%");
    if let Err(e) = db
        .prepare("DELETE FROM conversation_sessions WHERE session_key LIKE ?")
        .bind(&[prefix.as_str().into()])?
        .run()
        .await
    {
        console_log!("Failed to delete from conversation_sessions: {:?}", e);
    }

    // Nullify tenant_id in payments. The row stays for dispute and tax records.
    if let Err(e) = db
        .prepare("UPDATE payments SET tenant_id = NULL WHERE tenant_id = ?")
        .bind(&[tenant_id.into()])?
        .run()
        .await
    {
        console_log!("Failed to nullify tenant in payments: {:?}", e);
    }

    db.prepare("DELETE FROM tenants WHERE id = ?")
        .bind(&[tenant_id.into()])?
        .run()
        .await?;

    Ok(())
}

// ============================================================================
// Unified Message Storage
// ============================================================================

/// Save a unified message to D1. No message content stored: metadata only.
#[allow(clippy::too_many_arguments)]
pub async fn save_message(
    db: &D1Database,
    id: &str,
    channel: &Channel,
    direction: MessageDirection,
    sender: &str,
    recipient: &str,
    tenant_id: &str,
    channel_account_id: &str,
    action_taken: Option<MessageAction>,
    conversation_id: Option<&str>,
) -> Result<()> {
    save_message_raw_action(
        db,
        id,
        channel,
        direction,
        sender,
        recipient,
        tenant_id,
        channel_account_id,
        action_taken.map(|a| a.as_str()),
        conversation_id,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn save_message_raw_action(
    db: &D1Database,
    id: &str,
    channel: &Channel,
    direction: MessageDirection,
    sender: &str,
    recipient: &str,
    tenant_id: &str,
    channel_account_id: &str,
    action_taken: Option<&str>,
    conversation_id: Option<&str>,
) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO messages (id, channel, direction, sender, recipient, tenant_id, channel_account_id, action_taken, conversation_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    );
    stmt.bind(&[
        id.into(),
        channel.as_str().into(),
        direction.as_str().into(),
        sender.into(),
        recipient.into(),
        tenant_id.into(),
        channel_account_id.into(),
        action_taken.map(JsValue::from).unwrap_or(JsValue::null()),
        conversation_id
            .map(JsValue::from)
            .unwrap_or(JsValue::null()),
    ])?
    .run()
    .await?;
    Ok(())
}

pub async fn save_inbound_message(
    db: &D1Database,
    msg: &InboundMessage,
    action_taken: Option<MessageAction>,
) -> Result<()> {
    save_message(
        db,
        &msg.id,
        &msg.channel,
        MessageDirection::Inbound,
        &msg.sender,
        &msg.recipient,
        &msg.tenant_id,
        &msg.channel_account_id,
        action_taken,
        None,
    )
    .await
}

pub async fn update_message_conversation_id(
    db: &D1Database,
    message_id: &str,
    conversation_id: &str,
) -> Result<()> {
    let stmt = db.prepare("UPDATE messages SET conversation_id = ? WHERE id = ?");
    stmt.bind(&[conversation_id.into(), message_id.into()])?
        .run()
        .await?;
    Ok(())
}

// ============================================================================
// Conversation Sessions (D1)
// ============================================================================

const SESSION_TTL: i64 = 24 * 60 * 60;

fn conversation_session_key(
    tenant_id: &str,
    channel: &Channel,
    channel_account_id: &str,
    conversation_key: &str,
) -> String {
    let channel_str = channel.as_str();
    let key_hash = crate::helpers::sha256_hex(conversation_key);
    format!("convsession:{tenant_id}:{channel_str}:{channel_account_id}:{key_hash}")
}

pub async fn get_conversation_session(
    db: &D1Database,
    tenant_id: &str,
    channel: &Channel,
    channel_account_id: &str,
    conversation_key: &str,
) -> Result<Option<Session>> {
    let key = conversation_session_key(tenant_id, channel, channel_account_id, conversation_key);
    let now = now_epoch_secs();
    let row = db
        .prepare("SELECT session_json FROM conversation_sessions WHERE session_key = ? AND expires_at > ?")
        .bind(&[key.as_str().into(), JsValue::from(now as f64)])?
        .first::<serde_json::Value>(None)
        .await?;
    let Some(json_str) = row
        .as_ref()
        .and_then(|r| r.get("session_json"))
        .and_then(|v| v.as_str())
    else {
        return Ok(None);
    };
    let session = serde_json::from_str::<Session>(json_str)
        .map_err(|e| Error::from(format!("JSON error: {e}")))?;
    Ok(Some(session))
}

pub async fn save_conversation_session(
    db: &D1Database,
    tenant_id: &str,
    channel: &Channel,
    channel_account_id: &str,
    conversation_key: &str,
    state: &Session,
) -> Result<()> {
    let key = conversation_session_key(tenant_id, channel, channel_account_id, conversation_key);
    let json = serde_json::to_string(state).map_err(|e| Error::from(format!("JSON error: {e}")))?;
    let now = now_epoch_secs();
    let expires_at = now + SESSION_TTL;
    let updated_at = now_iso();
    let _ = db
        .prepare("DELETE FROM conversation_sessions WHERE expires_at <= ?")
        .bind(&[JsValue::from(now as f64)])?
        .run()
        .await;
    db.prepare(
        "INSERT INTO conversation_sessions (session_key, session_json, expires_at, updated_at)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(session_key) DO UPDATE SET
           session_json = excluded.session_json,
           expires_at = excluded.expires_at,
           updated_at = excluded.updated_at",
    )
    .bind(&[
        key.as_str().into(),
        json.as_str().into(),
        JsValue::from(expires_at as f64),
        updated_at.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

// ============================================================================
// Onboarding State (D1)
// ============================================================================

pub async fn get_onboarding(db: &D1Database, tenant_id: &str) -> Result<OnboardingState> {
    let row = db
        .prepare("SELECT state_json FROM tenant_onboarding WHERE tenant_id = ?")
        .bind(&[tenant_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    match row
        .as_ref()
        .and_then(|r| r.get("state_json"))
        .and_then(|v| v.as_str())
    {
        Some(json_str) => serde_json::from_str::<OnboardingState>(json_str)
            .map_err(|e| Error::from(format!("JSON error: {e}"))),
        None => Ok(OnboardingState::default()),
    }
}

pub async fn save_onboarding(
    db: &D1Database,
    tenant_id: &str,
    state: &OnboardingState,
) -> Result<()> {
    let json = serde_json::to_string(state).map_err(|e| Error::from(format!("JSON error: {e}")))?;
    let updated_at = now_iso();
    db.prepare(
        "INSERT INTO tenant_onboarding (tenant_id, state_json, updated_at)
         VALUES (?, ?, ?)
         ON CONFLICT(tenant_id) DO UPDATE SET
           state_json = excluded.state_json,
           updated_at = excluded.updated_at",
    )
    .bind(&[
        tenant_id.into(),
        json.as_str().into(),
        updated_at.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

// ============================================================================
// Rate Limits (D1)
// ============================================================================

pub async fn get_rate_limit(db: &D1Database, bucket_key: &str) -> Result<i64> {
    let now = now_epoch_secs();
    let row = db
        .prepare("SELECT count FROM rate_limits WHERE bucket_key = ? AND expires_at > ?")
        .bind(&[bucket_key.into(), JsValue::from(now as f64)])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row
        .and_then(|r| r.get("count").and_then(|v| v.as_i64()))
        .unwrap_or(0))
}

pub async fn put_rate_limit(
    db: &D1Database,
    bucket_key: &str,
    count: i64,
    ttl_seconds: u64,
) -> Result<()> {
    let now = now_epoch_secs();
    let expires_at = now + (ttl_seconds as i64);
    let _ = db
        .prepare("DELETE FROM rate_limits WHERE expires_at <= ?")
        .bind(&[JsValue::from(now as f64)])?
        .run()
        .await;
    db.prepare(
        "INSERT INTO rate_limits (bucket_key, count, expires_at)
         VALUES (?, ?, ?)
         ON CONFLICT(bucket_key) DO UPDATE SET
           count = excluded.count,
           expires_at = excluded.expires_at",
    )
    .bind(&[
        bucket_key.into(),
        JsValue::from(count as f64),
        JsValue::from(expires_at as f64),
    ])?
    .run()
    .await?;
    Ok(())
}

// ============================================================================
// Billing Storage
// ============================================================================

pub async fn get_tenant_billing(db: &D1Database, tenant_id: &str) -> Result<TenantBilling> {
    let stmt =
        db.prepare("SELECT credits_json, replies_used FROM tenant_billing WHERE tenant_id = ?");
    let result = stmt
        .bind(&[tenant_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    match result {
        Some(row) => {
            let credits_json = row
                .get("credits_json")
                .and_then(|v| v.as_str())
                .unwrap_or("[]");
            let credits: Vec<CreditEntry> = serde_json::from_str(credits_json).unwrap_or_default();
            let replies_used = row
                .get("replies_used")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            Ok(TenantBilling {
                credits,
                replies_used,
            })
        }
        None => Ok(TenantBilling::default()),
    }
}

pub async fn save_tenant_billing(
    db: &D1Database,
    tenant_id: &str,
    billing: &TenantBilling,
) -> Result<()> {
    let credits_json = serde_json::to_string(&billing.credits)
        .map_err(|e| Error::from(format!("JSON error: {e}")))?;
    let stmt = db.prepare(
        "INSERT INTO tenant_billing (tenant_id, credits_json, replies_used, updated_at)
         VALUES (?, ?, ?, datetime('now'))
         ON CONFLICT(tenant_id) DO UPDATE SET
           credits_json = excluded.credits_json,
           replies_used = excluded.replies_used,
           updated_at = datetime('now')",
    );
    stmt.bind(&[
        tenant_id.into(),
        credits_json.as_str().into(),
        (billing.replies_used as f64).into(),
    ])?
    .run()
    .await?;
    Ok(())
}
