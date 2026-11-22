use wasm_bindgen::JsValue;
use worker::*;

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
    let row = db
        .prepare("SELECT * FROM tenants WHERE email = ? COLLATE NOCASE")
        .bind(&[email.into()])?
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
        tenant.email.as_str().into(),
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
// Session KV Operations
// ============================================================================

pub async fn save_session(
    kv: &kv::KvStore,
    token: &str,
    tenant_id: &str,
    ttl_seconds: u64,
) -> Result<()> {
    kv.put(&format!("session:{}", token), tenant_id)?
        .expiration_ttl(ttl_seconds)
        .execute()
        .await?;
    Ok(())
}

pub async fn get_session(kv: &kv::KvStore, token: &str) -> Result<Option<String>> {
    kv.get(&format!("session:{}", token))
        .text()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

pub async fn delete_session(kv: &kv::KvStore, token: &str) -> Result<()> {
    kv.delete(&format!("session:{}", token)).await?;
    Ok(())
}

// ============================================================================
// WhatsApp Account KV Operations
// ============================================================================

pub async fn get_whatsapp_account(kv: &kv::KvStore, id: &str) -> Result<Option<WhatsAppAccount>> {
    kv.get(&format!("whatsapp:{}", id))
        .json::<WhatsAppAccount>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

pub async fn save_whatsapp_account(kv: &kv::KvStore, account: &WhatsAppAccount) -> Result<()> {
    kv.put(&format!("whatsapp:{}", account.id), account)?
        .execute()
        .await?;
    if !account.tenant_id.is_empty() {
        kv.put(
            &format!("tenant:{}:whatsapp:{}", account.tenant_id, account.id),
            "",
        )?
        .execute()
        .await?;
    }
    // Reverse index: phone_number_id -> whatsapp account id (for webhook routing)
    if !account.phone_number_id.is_empty() {
        kv.put(
            &format!("wa_phone:{}", account.phone_number_id),
            &account.id,
        )?
        .execute()
        .await?;
    }
    Ok(())
}

pub async fn delete_whatsapp_account(kv: &kv::KvStore, tenant_id: &str, id: &str) -> Result<()> {
    if let Some(account) = get_whatsapp_account(kv, id).await? {
        if !account.phone_number_id.is_empty() {
            kv.delete(&format!("wa_phone:{}", account.phone_number_id))
                .await?;
        }
    }
    kv.delete(&format!("whatsapp:{}", id)).await?;
    if !tenant_id.is_empty() {
        kv.delete(&format!("tenant:{}:whatsapp:{}", tenant_id, id))
            .await?;
    }
    Ok(())
}

pub async fn list_whatsapp_accounts(
    kv: &kv::KvStore,
    tenant_id: &str,
) -> Result<Vec<WhatsAppAccount>> {
    let prefix = format!("tenant:{}:whatsapp:", tenant_id);
    let list = kv
        .list()
        .prefix(prefix.clone())
        .execute()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let mut accounts = Vec::new();
    for key in list.keys {
        let account_id = key.name.strip_prefix(&prefix).unwrap_or("").to_string();
        if account_id.is_empty() {
            continue;
        }
        if let Some(account) = get_whatsapp_account(kv, &account_id).await? {
            accounts.push(account);
        }
    }
    accounts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(accounts)
}

pub async fn get_whatsapp_account_by_phone(
    kv: &kv::KvStore,
    phone_number_id: &str,
) -> Result<Option<String>> {
    kv.get(&format!("wa_phone:{}", phone_number_id))
        .text()
        .await
        .map_err(|e| Error::from(e.to_string()))
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
    kv: &kv::KvStore,
    channel: &Channel,
    account_id: &str,
) -> Result<Option<ChannelReplyInfo>> {
    match channel {
        Channel::WhatsApp => {
            Ok(get_whatsapp_account(kv, account_id)
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
    kv: &kv::KvStore,
    channel: &Channel,
    account_id: &str,
) -> Result<Option<ReplyConfig>> {
    Ok(get_channel_reply_info(kv, channel, account_id)
        .await?
        .map(|info| info.config))
}

// ============================================================================
// Delete All Tenant Data
// ============================================================================

pub async fn delete_tenant_data(kv: &kv::KvStore, db: &D1Database, tenant_id: &str) -> Result<()> {
    let wa_accounts = list_whatsapp_accounts(kv, tenant_id).await?;
    for account in &wa_accounts {
        delete_whatsapp_account(kv, tenant_id, &account.id).await?;
    }

    for table in &["messages", "tenant_billing"] {
        let query = format!("DELETE FROM {} WHERE tenant_id = ?", table);
        let stmt = db.prepare(&query);
        if let Err(e) = stmt.bind(&[tenant_id.into()])?.run().await {
            console_log!("Failed to delete from {}: {:?}", table, e);
        }
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

    kv.delete(&format!("onboarding:{}", tenant_id)).await?;
    kv.delete(&format!("tenant:{}:credentials", tenant_id))
        .await?;
    kv.delete(&format!("csrf:{}", tenant_id)).await?;

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
        action_taken
            .map(|a| JsValue::from(a.as_str()))
            .unwrap_or(JsValue::null()),
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
// Conversation Sessions (KV)
// ============================================================================

const SESSION_TTL: u64 = 24 * 60 * 60;

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
    kv: &kv::KvStore,
    tenant_id: &str,
    channel: &Channel,
    channel_account_id: &str,
    conversation_key: &str,
) -> Result<Option<Session>> {
    let key = conversation_session_key(tenant_id, channel, channel_account_id, conversation_key);
    kv.get(&key)
        .json::<Session>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

pub async fn save_conversation_session(
    kv: &kv::KvStore,
    tenant_id: &str,
    channel: &Channel,
    channel_account_id: &str,
    conversation_key: &str,
    state: &Session,
) -> Result<()> {
    let key = conversation_session_key(tenant_id, channel, channel_account_id, conversation_key);
    let json = serde_json::to_string(state).map_err(|e| Error::from(format!("JSON error: {e}")))?;
    kv.put(&key, json)?
        .expiration_ttl(SESSION_TTL)
        .execute()
        .await?;
    Ok(())
}

// ============================================================================
// Onboarding State (KV)
// ============================================================================

pub async fn get_onboarding(kv: &kv::KvStore, tenant_id: &str) -> Result<OnboardingState> {
    let key = format!("onboarding:{tenant_id}");
    kv.get(&key)
        .json::<OnboardingState>()
        .await
        .map_err(|e| Error::from(e.to_string()))
        .map(|opt| opt.unwrap_or_default())
}

pub async fn save_onboarding(
    kv: &kv::KvStore,
    tenant_id: &str,
    state: &OnboardingState,
) -> Result<()> {
    let key = format!("onboarding:{tenant_id}");
    let json = serde_json::to_string(state).map_err(|e| Error::from(format!("JSON error: {e}")))?;
    kv.put(&key, json)?.execute().await?;
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
