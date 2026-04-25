use wasm_bindgen::JsValue;
use worker::*;

use crate::types::{
    CreditEntry, InstagramAccount, LeadCaptureForm, Tenant, TenantBilling, WhatsAppAccount,
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
            .unwrap_or("free")
            .to_string(),
        currency: row
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("INR")
            .to_string(),
        email_address_packs_purchased: row
            .get("email_address_packs_purchased")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
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
        "INSERT INTO tenants (id, email, name, facebook_id, plan, currency, email_address_packs_purchased, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           email = excluded.email,
           name = excluded.name,
           facebook_id = excluded.facebook_id,
           plan = excluded.plan,
           currency = excluded.currency,
           email_address_packs_purchased = excluded.email_address_packs_purchased,
           updated_at = excluded.updated_at",
    )
    .bind(&[
        tenant.id.as_str().into(),
        tenant.email.as_str().into(),
        name_val,
        fb_val,
        tenant.plan.as_str().into(),
        tenant.currency.as_str().into(),
        JsValue::from(tenant.email_address_packs_purchased as f64),
        tenant.created_at.as_str().into(),
        tenant.updated_at.as_str().into(),
    ])?
    .run()
    .await?;
    Ok(())
}

pub async fn list_tenants(db: &D1Database) -> Result<Vec<Tenant>> {
    let results = db
        .prepare("SELECT * FROM tenants ORDER BY created_at DESC")
        .all()
        .await?;
    let rows: Vec<serde_json::Value> = results.results()?;
    Ok(rows.iter().map(row_to_tenant).collect())
}

pub async fn count_tenants(db: &D1Database) -> Result<usize> {
    let row = db
        .prepare("SELECT COUNT(*) as cnt FROM tenants")
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row
        .and_then(|r| r.get("cnt").and_then(|v| v.as_u64()))
        .unwrap_or(0) as usize)
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
// CSRF Token KV Operations
// ============================================================================

pub async fn save_csrf_token(
    kv: &kv::KvStore,
    tenant_id: &str,
    token: &str,
    ttl_seconds: u64,
) -> Result<()> {
    kv.put(&format!("csrf:{}", tenant_id), token)?
        .expiration_ttl(ttl_seconds)
        .execute()
        .await?;
    Ok(())
}

pub async fn get_csrf_token(kv: &kv::KvStore, tenant_id: &str) -> Result<Option<String>> {
    kv.get(&format!("csrf:{}", tenant_id))
        .text()
        .await
        .map_err(|e| Error::from(e.to_string()))
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
    // Load account first to clean up phone index
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
// Instagram Account KV Operations
// ============================================================================

pub async fn get_instagram_account(kv: &kv::KvStore, id: &str) -> Result<Option<InstagramAccount>> {
    kv.get(&format!("instagram:{}", id))
        .json::<InstagramAccount>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

pub async fn save_instagram_account(kv: &kv::KvStore, account: &InstagramAccount) -> Result<()> {
    kv.put(&format!("instagram:{}", account.id), account)?
        .execute()
        .await?;
    if !account.tenant_id.is_empty() {
        kv.put(
            &format!("tenant:{}:instagram:{}", account.tenant_id, account.id),
            "",
        )?
        .execute()
        .await?;
    }
    // Reverse index: page_id -> instagram account id (for webhook routing)
    if !account.page_id.is_empty() {
        kv.put(&format!("ig_page:{}", account.page_id), &account.id)?
            .execute()
            .await?;
    }
    Ok(())
}

pub async fn delete_instagram_account(kv: &kv::KvStore, tenant_id: &str, id: &str) -> Result<()> {
    if let Some(account) = get_instagram_account(kv, id).await? {
        if !account.page_id.is_empty() {
            kv.delete(&format!("ig_page:{}", account.page_id)).await?;
        }
    }
    kv.delete(&format!("instagram:{}", id)).await?;
    if !tenant_id.is_empty() {
        kv.delete(&format!("tenant:{}:instagram:{}", tenant_id, id))
            .await?;
    }
    kv.delete(&format!("instagram_token:{}", id)).await?;
    Ok(())
}

pub async fn list_instagram_accounts(
    kv: &kv::KvStore,
    tenant_id: &str,
) -> Result<Vec<InstagramAccount>> {
    let prefix = format!("tenant:{}:instagram:", tenant_id);
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
        if let Some(account) = get_instagram_account(kv, &account_id).await? {
            accounts.push(account);
        }
    }
    accounts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(accounts)
}

pub async fn get_instagram_account_by_page(
    kv: &kv::KvStore,
    page_id: &str,
) -> Result<Option<String>> {
    kv.get(&format!("ig_page:{}", page_id))
        .text()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

// ============================================================================
// Lead Form KV Operations
// ============================================================================

pub async fn get_lead_form(kv: &kv::KvStore, id: &str) -> Result<Option<LeadCaptureForm>> {
    kv.get(&format!("lead_form:{}", id))
        .json::<LeadCaptureForm>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

pub async fn save_lead_form(kv: &kv::KvStore, form: &LeadCaptureForm) -> Result<()> {
    kv.put(&format!("lead_form:{}", form.id), form)?
        .execute()
        .await?;
    if !form.tenant_id.is_empty() {
        kv.put(
            &format!("tenant:{}:lead_form:{}", form.tenant_id, form.id),
            "",
        )?
        .execute()
        .await?;
    }
    Ok(())
}

pub async fn delete_lead_form(kv: &kv::KvStore, tenant_id: &str, id: &str) -> Result<()> {
    kv.delete(&format!("lead_form:{}", id)).await?;
    if !tenant_id.is_empty() {
        kv.delete(&format!("tenant:{}:lead_form:{}", tenant_id, id))
            .await?;
    }
    Ok(())
}

pub async fn list_lead_forms(kv: &kv::KvStore, tenant_id: &str) -> Result<Vec<LeadCaptureForm>> {
    let prefix = format!("tenant:{}:lead_form:", tenant_id);
    let list = kv
        .list()
        .prefix(prefix.clone())
        .execute()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let mut forms = Vec::new();
    for key in list.keys {
        let form_id = key.name.strip_prefix(&prefix).unwrap_or("").to_string();
        if form_id.is_empty() {
            continue;
        }
        if let Some(form) = get_lead_form(kv, &form_id).await? {
            forms.push(form);
        }
    }
    forms.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(forms)
}

// ============================================================================
// Delete All Tenant Data
// ============================================================================

pub async fn delete_tenant_data(kv: &kv::KvStore, db: &D1Database, tenant_id: &str) -> Result<()> {
    // Delete WhatsApp accounts + phone indexes
    let wa_accounts = list_whatsapp_accounts(kv, tenant_id).await?;
    for account in &wa_accounts {
        delete_whatsapp_account(kv, tenant_id, &account.id).await?;
    }

    // Delete Instagram accounts + page indexes + tokens
    let ig_accounts = list_instagram_accounts(kv, tenant_id).await?;
    for account in &ig_accounts {
        delete_instagram_account(kv, tenant_id, &account.id).await?;
    }

    // Delete lead forms
    let forms = list_lead_forms(kv, tenant_id).await?;
    for form in &forms {
        delete_lead_form(kv, tenant_id, &form.id).await?;
    }

    // Delete D1 data: messages + billing (but preserve payments and audit_log for compliance)
    for table in &[
        "whatsapp_messages",
        "lead_form_submissions",
        "instagram_messages",
        "email_messages",
        "email_metrics",
        "messages",
        "tenant_billing",
    ] {
        let query = format!("DELETE FROM {} WHERE tenant_id = ?", table);
        let stmt = db.prepare(&query);
        if let Err(e) = stmt.bind(&[tenant_id.into()])?.run().await {
            console_log!("Failed to delete from {}: {:?}", table, e);
        }
    }

    // Nullify tenant_id in payments (preserve records for compliance)
    if let Err(e) = db
        .prepare("UPDATE payments SET tenant_id = NULL WHERE tenant_id = ?")
        .bind(&[tenant_id.into()])?
        .run()
        .await
    {
        console_log!("Failed to nullify tenant in payments: {:?}", e);
    }

    // Delete email addresses + indices (KV)
    if let Ok(addrs) = get_email_addresses(kv, tenant_id).await {
        for a in &addrs {
            let _ = delete_email_address_index(kv, &a.local_part).await;
        }
        let _ = save_email_addresses(kv, tenant_id, &[]).await;
    }

    // Delete discord config (KV)
    if let Ok(Some(config)) = get_discord_config_by_tenant(kv, tenant_id).await {
        kv.delete(&format!("discord_guild:{}", config.guild_id))
            .await?;
        kv.delete(&format!("discord_config:{}", tenant_id)).await?;
    }

    // Delete onboarding state and credentials (KV)
    kv.delete(&format!("onboarding:{}", tenant_id)).await?;
    kv.delete(&format!("tenant:{}:credentials", tenant_id))
        .await?;
    // Delete CSRF token (KV)
    kv.delete(&format!("csrf:{}", tenant_id)).await?;

    // Delete tenant record (D1)
    db.prepare("DELETE FROM tenants WHERE id = ?")
        .bind(&[tenant_id.into()])?
        .run()
        .await?;

    Ok(())
}

// ============================================================================
// D1 Operations (WhatsApp Messages)
// ============================================================================

pub async fn save_whatsapp_message(
    db: &D1Database,
    id: &str,
    whatsapp_account_id: &str,
    direction: &str,
    from_number: &str,
    to_number: &str,
    tenant_id: &str,
) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO whatsapp_messages (id, whatsapp_account_id, direction, from_number, to_number, tenant_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
    );
    stmt.bind(&[
        id.into(),
        whatsapp_account_id.into(),
        direction.into(),
        from_number.into(),
        to_number.into(),
        tenant_id.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

// ============================================================================
// D1 Operations (Lead Form Submissions)
// ============================================================================

#[allow(clippy::too_many_arguments)]
pub async fn save_lead_form_submission(
    db: &D1Database,
    id: &str,
    lead_form_id: &str,
    phone_number: &str,
    whatsapp_account_id: &str,
    message_sent: &str,
    reply_mode: &str,
    tenant_id: &str,
) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO lead_form_submissions (id, lead_form_id, phone_number, whatsapp_account_id, message_sent, reply_mode, tenant_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))",
    );
    stmt.bind(&[
        id.into(),
        lead_form_id.into(),
        phone_number.into(),
        whatsapp_account_id.into(),
        message_sent.into(),
        reply_mode.into(),
        tenant_id.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

// ============================================================================
// D1 Operations (Instagram Messages)
// ============================================================================

pub async fn save_instagram_message(
    db: &D1Database,
    id: &str,
    instagram_account_id: &str,
    direction: &str,
    sender_id: &str,
    recipient_id: &str,
    tenant_id: &str,
) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO instagram_messages (id, instagram_account_id, direction, sender_id, recipient_id, tenant_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
    );
    stmt.bind(&[
        id.into(),
        instagram_account_id.into(),
        direction.into(),
        sender_id.into(),
        recipient_id.into(),
        tenant_id.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

// ============================================================================
// Email Address Storage
// ============================================================================

use crate::types::{EmailAddress, EmailReverseAlias};

/// Get all email addresses owned by a tenant.
pub async fn get_email_addresses(kv: &kv::KvStore, tenant_id: &str) -> Result<Vec<EmailAddress>> {
    let key = format!("email_addrs:{tenant_id}");
    match kv
        .get(&key)
        .json::<Vec<EmailAddress>>()
        .await
        .map_err(|e| Error::from(e.to_string()))?
    {
        Some(addrs) => Ok(addrs),
        None => Ok(vec![]),
    }
}

/// Save the full address list for a tenant.
pub async fn save_email_addresses(
    kv: &kv::KvStore,
    tenant_id: &str,
    addrs: &[EmailAddress],
) -> Result<()> {
    let key = format!("email_addrs:{tenant_id}");
    kv.put(&key, serde_json::to_string(addrs)?)?
        .execute()
        .await?;
    Ok(())
}

/// Look up a single address by local-part within a tenant. Returns None if
/// the tenant doesn't own that local-part.
pub async fn get_email_address(
    kv: &kv::KvStore,
    tenant_id: &str,
    local_part: &str,
) -> Result<Option<EmailAddress>> {
    let addrs = get_email_addresses(kv, tenant_id).await?;
    Ok(addrs.into_iter().find(|a| a.local_part == local_part))
}

/// Insert-or-replace one address by local-part. Persists the full list.
pub async fn save_email_address(
    kv: &kv::KvStore,
    tenant_id: &str,
    addr: &EmailAddress,
) -> Result<()> {
    let mut addrs = get_email_addresses(kv, tenant_id).await?;
    if let Some(existing) = addrs.iter_mut().find(|a| a.local_part == addr.local_part) {
        *existing = addr.clone();
    } else {
        addrs.push(addr.clone());
    }
    save_email_addresses(kv, tenant_id, &addrs).await
}

/// Drop an address from the tenant's list. Returns true if it existed.
pub async fn delete_email_address(
    kv: &kv::KvStore,
    tenant_id: &str,
    local_part: &str,
) -> Result<bool> {
    let mut addrs = get_email_addresses(kv, tenant_id).await?;
    let before = addrs.len();
    addrs.retain(|a| a.local_part != local_part);
    if addrs.len() == before {
        return Ok(false);
    }
    save_email_addresses(kv, tenant_id, &addrs).await?;
    Ok(true)
}

/// Set the local-part → tenant_id reverse index. Local-parts are unique
/// across the platform since every tenant shares one email domain.
pub async fn set_email_address_index(
    kv: &kv::KvStore,
    local_part: &str,
    tenant_id: &str,
) -> Result<()> {
    let key = format!("email_addr:{local_part}");
    kv.put(&key, tenant_id)?.execute().await?;
    Ok(())
}

/// Look up tenant_id by local-part.
pub async fn get_tenant_by_address(kv: &kv::KvStore, local_part: &str) -> Result<Option<String>> {
    let key = format!("email_addr:{local_part}");
    kv.get(&key)
        .text()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

/// Delete the local-part → tenant index.
pub async fn delete_email_address_index(kv: &kv::KvStore, local_part: &str) -> Result<()> {
    let key = format!("email_addr:{local_part}");
    kv.delete(&key).await?;
    Ok(())
}

// --- Verification tokens for notification recipients --------------------

/// Payload stored under each verification token. The recipient_id locates
/// the row inside the address's notification_recipients vec.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct EmailVerificationPayload {
    pub tenant_id: String,
    pub local_part: String,
    pub recipient_id: String,
}

const EMAIL_VERIFICATION_TTL: u64 = 7 * 24 * 60 * 60; // 7 days

pub async fn set_email_verification_token(
    kv: &kv::KvStore,
    token: &str,
    payload: &EmailVerificationPayload,
) -> Result<()> {
    let key = format!("email_verify:{token}");
    kv.put(&key, serde_json::to_string(payload)?)?
        .expiration_ttl(EMAIL_VERIFICATION_TTL)
        .execute()
        .await?;
    Ok(())
}

pub async fn get_email_verification_token(
    kv: &kv::KvStore,
    token: &str,
) -> Result<Option<EmailVerificationPayload>> {
    let key = format!("email_verify:{token}");
    kv.get(&key)
        .json::<EmailVerificationPayload>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

pub async fn delete_email_verification_token(kv: &kv::KvStore, token: &str) -> Result<()> {
    let key = format!("email_verify:{token}");
    kv.delete(&key).await?;
    Ok(())
}

// --- Reverse aliases (unchanged behavior; domain field now the platform) ---

/// Get a reverse alias mapping.
pub async fn get_email_reverse_alias(
    kv: &kv::KvStore,
    reverse_address: &str,
) -> Result<Option<EmailReverseAlias>> {
    let key = format!("email_reverse:{reverse_address}");
    kv.get(&key)
        .json::<EmailReverseAlias>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

/// Save a reverse alias mapping (30-day TTL).
pub async fn save_email_reverse_alias(
    kv: &kv::KvStore,
    reverse_address: &str,
    alias: &EmailReverseAlias,
) -> Result<()> {
    let key = format!("email_reverse:{reverse_address}");
    kv.put(&key, serde_json::to_string(alias)?)?
        .expiration_ttl(30 * 24 * 60 * 60)
        .execute()
        .await?;
    Ok(())
}

// ============================================================================
// Unified Message Storage
// ============================================================================

use crate::types::{Channel, ConversationContext, DiscordConfig, InboundMessage, OnboardingState};

/// Save a unified message to D1. No message content stored: metadata only.
pub async fn save_message(
    db: &D1Database,
    id: &str,
    channel: &Channel,
    direction: &str,
    sender: &str,
    recipient: &str,
    tenant_id: &str,
    channel_account_id: &str,
    action_taken: Option<&str>,
) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO messages (id, channel, direction, sender, recipient, tenant_id, channel_account_id, action_taken)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    );
    stmt.bind(&[
        id.into(),
        channel.as_str().into(),
        direction.into(),
        sender.into(),
        recipient.into(),
        tenant_id.into(),
        channel_account_id.into(),
        action_taken.map(JsValue::from).unwrap_or(JsValue::null()),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Save a message from an InboundMessage struct.
pub async fn save_inbound_message(
    db: &D1Database,
    msg: &InboundMessage,
    action_taken: Option<&str>,
) -> Result<()> {
    save_message(
        db,
        &msg.id,
        &msg.channel,
        "inbound",
        &msg.sender,
        &msg.recipient,
        &msg.tenant_id,
        &msg.channel_account_id,
        action_taken,
    )
    .await
}

/// Get recent unified messages for a tenant.
pub async fn get_messages(
    db: &D1Database,
    tenant_id: &str,
    channel: Option<&Channel>,
    limit: u32,
) -> Result<Vec<serde_json::Value>> {
    if let Some(ch) = channel {
        let stmt = db.prepare(
            "SELECT * FROM messages WHERE tenant_id = ? AND channel = ? ORDER BY created_at DESC LIMIT ?",
        );
        let result = stmt
            .bind(&[
                tenant_id.into(),
                ch.as_str().into(),
                JsValue::from(limit as f64),
            ])?
            .all()
            .await?;
        result.results::<serde_json::Value>()
    } else {
        let stmt = db
            .prepare("SELECT * FROM messages WHERE tenant_id = ? ORDER BY created_at DESC LIMIT ?");
        let result = stmt
            .bind(&[tenant_id.into(), JsValue::from(limit as f64)])?
            .all()
            .await?;
        result.results::<serde_json::Value>()
    }
}

// ============================================================================
// Conversation Context (KV)
// ============================================================================

const CONVERSATION_TTL: u64 = 7 * 24 * 60 * 60; // 7 days

pub async fn save_conversation_context(kv: &kv::KvStore, ctx: &ConversationContext) -> Result<()> {
    let key = format!("conv:{}", ctx.id);
    let json = serde_json::to_string(ctx).map_err(|e| Error::from(format!("JSON error: {e}")))?;
    kv.put(&key, json)?
        .expiration_ttl(CONVERSATION_TTL)
        .execute()
        .await?;
    Ok(())
}

pub async fn get_conversation_context(
    kv: &kv::KvStore,
    id: &str,
) -> Result<Option<ConversationContext>> {
    let key = format!("conv:{id}");
    kv.get(&key)
        .json::<ConversationContext>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

pub async fn delete_conversation_context(kv: &kv::KvStore, id: &str) -> Result<()> {
    let key = format!("conv:{id}");
    kv.delete(&key).await?;
    Ok(())
}

// ============================================================================
// Discord Config (KV)
// ============================================================================

pub async fn get_discord_config_by_guild(
    kv: &kv::KvStore,
    guild_id: &str,
) -> Result<Option<DiscordConfig>> {
    let key = format!("discord_guild:{guild_id}");
    kv.get(&key)
        .json::<DiscordConfig>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

pub async fn save_discord_config(kv: &kv::KvStore, config: &DiscordConfig) -> Result<()> {
    let key = format!("discord_guild:{}", config.guild_id);
    let json =
        serde_json::to_string(config).map_err(|e| Error::from(format!("JSON error: {e}")))?;
    kv.put(&key, json)?.execute().await?;

    // Also store reverse mapping
    let rev_key = format!("discord_config:{}", config.tenant_id);
    let rev_json =
        serde_json::to_string(config).map_err(|e| Error::from(format!("JSON error: {e}")))?;
    kv.put(&rev_key, rev_json)?.execute().await?;
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

pub async fn get_discord_config_by_tenant(
    kv: &kv::KvStore,
    tenant_id: &str,
) -> Result<Option<DiscordConfig>> {
    let key = format!("discord_config:{tenant_id}");
    kv.get(&key)
        .json::<DiscordConfig>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

// ============================================================================
// Billing Storage
// ============================================================================

pub async fn get_tenant_billing(db: &D1Database, tenant_id: &str) -> Result<TenantBilling> {
    let stmt = db.prepare(
        "SELECT credits_json, free_month, replies_used FROM tenant_billing WHERE tenant_id = ?",
    );
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
            let free_month = row
                .get("free_month")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let replies_used = row
                .get("replies_used")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            Ok(TenantBilling {
                credits,
                free_month,
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
    let free_month_val: wasm_bindgen::JsValue = match &billing.free_month {
        Some(m) => m.as_str().into(),
        None => wasm_bindgen::JsValue::NULL,
    };
    let stmt = db.prepare(
        "INSERT INTO tenant_billing (tenant_id, credits_json, free_month, replies_used, updated_at)
         VALUES (?, ?, ?, ?, datetime('now'))
         ON CONFLICT(tenant_id) DO UPDATE SET
           credits_json = excluded.credits_json,
           free_month = excluded.free_month,
           replies_used = excluded.replies_used,
           updated_at = datetime('now')",
    );
    stmt.bind(&[
        tenant_id.into(),
        credits_json.as_str().into(),
        free_month_val,
        (billing.replies_used as f64).into(),
    ])?
    .run()
    .await?;
    Ok(())
}
