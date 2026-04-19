use wasm_bindgen::JsValue;
use worker::*;

use crate::types::{
    CreditPackRow, InstagramAccount, LeadCaptureForm, Tenant, TenantBilling, WhatsAppAccount,
};

// ============================================================================
// Tenant KV Operations
// ============================================================================

pub async fn get_tenant(kv: &kv::KvStore, id: &str) -> Result<Option<Tenant>> {
    kv.get(&format!("tenant:{}", id))
        .json::<Tenant>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

pub async fn get_tenant_by_email(kv: &kv::KvStore, email: &str) -> Result<Option<Tenant>> {
    let tenant_id = kv
        .get(&format!("tenant_email:{}", email.to_lowercase()))
        .text()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    match tenant_id {
        Some(id) => get_tenant(kv, &id).await,
        None => Ok(None),
    }
}

pub async fn get_tenant_by_facebook_id(
    kv: &kv::KvStore,
    facebook_id: &str,
) -> Result<Option<Tenant>> {
    let tenant_id = kv
        .get(&format!("tenant_facebook:{}", facebook_id))
        .text()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    match tenant_id {
        Some(id) => get_tenant(kv, &id).await,
        None => Ok(None),
    }
}

pub async fn save_tenant(kv: &kv::KvStore, tenant: &Tenant) -> Result<()> {
    kv.put(&format!("tenant:{}", tenant.id), tenant)?
        .execute()
        .await?;
    if !tenant.email.is_empty() {
        kv.put(
            &format!("tenant_email:{}", tenant.email.to_lowercase()),
            &tenant.id,
        )?
        .execute()
        .await?;
    }
    if let Some(ref fb_id) = tenant.facebook_id {
        kv.put(&format!("tenant_facebook:{}", fb_id), &tenant.id)?
            .execute()
            .await?;
    }
    Ok(())
}

pub async fn delete_tenant_facebook_index(kv: &kv::KvStore, facebook_id: &str) -> Result<()> {
    kv.delete(&format!("tenant_facebook:{}", facebook_id))
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

    // Delete ALL D1 data for this tenant
    for table in &[
        "whatsapp_messages",
        "lead_form_submissions",
        "instagram_messages",
        "email_messages",
        "email_metrics",
        "messages",
        "payments",
        "audit_log",
    ] {
        let query = format!("DELETE FROM {} WHERE tenant_id = ?", table);
        let stmt = db.prepare(&query);
        if let Err(e) = stmt.bind(&[tenant_id.into()])?.run().await {
            console_log!("Failed to delete from {}: {:?}", table, e);
        }
    }

    // Delete KV billing data
    kv.delete(&format!("billing:{}", tenant_id)).await?;

    // Delete email domains and rules
    if let Ok(domains) = get_email_domains(kv, tenant_id).await {
        for domain in &domains {
            let _ = delete_email_domain_index(kv, &domain.domain).await;
            let _ = save_email_rules(kv, tenant_id, &domain.domain, &[]).await;
        }
        let _ = save_email_domains(kv, tenant_id, &[]).await;
    }

    // Delete discord config
    if let Ok(Some(config)) = get_discord_config_by_tenant(kv, tenant_id).await {
        kv.delete(&format!("discord_guild:{}", config.guild_id))
            .await?;
        kv.delete(&format!("discord_config:{}", tenant_id)).await?;
    }

    // Delete onboarding state
    kv.delete(&format!("onboarding:{}", tenant_id)).await?;

    // Delete tenant credentials and tenant record
    kv.delete(&format!("tenant:{}:credentials", tenant_id))
        .await?;

    if let Some(tenant) = get_tenant(kv, tenant_id).await? {
        if !tenant.email.is_empty() {
            kv.delete(&format!("tenant_email:{}", tenant.email.to_lowercase()))
                .await?;
        }
        if let Some(ref fb_id) = tenant.facebook_id {
            kv.delete(&format!("tenant_facebook:{}", fb_id)).await?;
        }
    }
    kv.delete(&format!("tenant:{}", tenant_id)).await?;

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
// Email Routing Storage
// ============================================================================

use crate::types::{EmailDomain, EmailReverseAlias, RoutingRule};

/// Get all domains for a tenant.
pub async fn get_email_domains(kv: &kv::KvStore, tenant_id: &str) -> Result<Vec<EmailDomain>> {
    let key = format!("email_domains:{tenant_id}");
    match kv
        .get(&key)
        .json::<Vec<EmailDomain>>()
        .await
        .map_err(|e| Error::from(e.to_string()))?
    {
        Some(domains) => Ok(domains),
        None => Ok(vec![]),
    }
}

/// Save all domains for a tenant.
pub async fn save_email_domains(
    kv: &kv::KvStore,
    tenant_id: &str,
    domains: &[EmailDomain],
) -> Result<()> {
    let key = format!("email_domains:{tenant_id}");
    kv.put(&key, serde_json::to_string(domains)?)?
        .execute()
        .await?;
    Ok(())
}

/// Set the domain→tenant reverse index.
pub async fn set_email_domain_index(kv: &kv::KvStore, domain: &str, tenant_id: &str) -> Result<()> {
    let key = format!("email_domain:{domain}");
    kv.put(&key, tenant_id)?.execute().await?;
    Ok(())
}

/// Look up tenant_id by domain.
pub async fn get_tenant_by_domain(kv: &kv::KvStore, domain: &str) -> Result<Option<String>> {
    let key = format!("email_domain:{domain}");
    kv.get(&key)
        .text()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

/// Delete the domain→tenant reverse index.
pub async fn delete_email_domain_index(kv: &kv::KvStore, domain: &str) -> Result<()> {
    let key = format!("email_domain:{domain}");
    kv.delete(&key).await?;
    Ok(())
}

/// Get routing rules for a domain.
pub async fn get_email_rules(
    kv: &kv::KvStore,
    tenant_id: &str,
    domain: &str,
) -> Result<Vec<RoutingRule>> {
    let key = format!("email_rules:{tenant_id}:{domain}");
    match kv
        .get(&key)
        .json::<Vec<RoutingRule>>()
        .await
        .map_err(|e| Error::from(e.to_string()))?
    {
        Some(rules) => Ok(rules),
        None => Ok(vec![]),
    }
}

/// Save routing rules for a domain (sorted by priority ascending).
pub async fn save_email_rules(
    kv: &kv::KvStore,
    tenant_id: &str,
    domain: &str,
    rules: &[RoutingRule],
) -> Result<()> {
    let key = format!("email_rules:{tenant_id}:{domain}");
    kv.put(&key, serde_json::to_string(rules)?)?
        .execute()
        .await?;
    Ok(())
}

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

/// Save a reverse alias mapping.
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

/// Get discord bot token for a tenant.
pub async fn get_discord_bot_token(kv: &kv::KvStore, tenant_id: &str) -> Result<Option<String>> {
    let key = format!("discord_bot_token:{tenant_id}");
    kv.get(&key)
        .text()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

/// Save discord bot token for a tenant.
pub async fn save_discord_bot_token(kv: &kv::KvStore, tenant_id: &str, token: &str) -> Result<()> {
    let key = format!("discord_bot_token:{tenant_id}");
    kv.put(&key, token)?.execute().await?;
    Ok(())
}

/// Log an email message to D1.
pub struct EmailLogEntry<'a> {
    pub id: &'a str,
    pub tenant_id: &'a str,
    pub domain: &'a str,
    pub rule_id: Option<&'a str>,
    pub direction: &'a str,
    pub from_email: &'a str,
    pub to_email: &'a str,
    pub action_taken: &'a str,
    pub error_msg: Option<&'a str>,
}

pub async fn save_email_message(db: &D1Database, entry: &EmailLogEntry<'_>) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO email_messages (id, tenant_id, domain, rule_id, direction, from_email, to_email, action_taken, error_msg)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    );
    stmt.bind(&[
        entry.id.into(),
        entry.tenant_id.into(),
        entry.domain.into(),
        entry.rule_id.map(JsValue::from).unwrap_or(JsValue::null()),
        entry.direction.into(),
        entry.from_email.into(),
        entry.to_email.into(),
        entry.action_taken.into(),
        entry
            .error_msg
            .map(JsValue::from)
            .unwrap_or(JsValue::null()),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Increment email metrics counter.
pub async fn increment_email_metric(
    db: &D1Database,
    domain: &str,
    rule_id: Option<&str>,
    action_type: &str,
    tenant_id: &str,
) -> Result<()> {
    let date = js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default();
    let date = &date[..10]; // YYYY-MM-DD

    let stmt = db.prepare(
        "INSERT INTO email_metrics (domain, rule_id, date, action_type, count, tenant_id)
         VALUES (?, ?, ?, ?, 1, ?)
         ON CONFLICT(domain, rule_id, date, action_type)
         DO UPDATE SET count = count + 1",
    );
    stmt.bind(&[
        domain.into(),
        rule_id.map(JsValue::from).unwrap_or(JsValue::null()),
        date.into(),
        action_type.into(),
        tenant_id.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Get recent email log entries for a tenant.
pub async fn get_email_log(
    db: &D1Database,
    tenant_id: &str,
    limit: u32,
) -> Result<Vec<serde_json::Value>> {
    let stmt = db.prepare(
        "SELECT * FROM email_messages WHERE tenant_id = ? ORDER BY created_at DESC LIMIT ?",
    );
    let result = stmt
        .bind(&[tenant_id.into(), JsValue::from(limit as f64)])?
        .all()
        .await?;
    result.results::<serde_json::Value>()
}

/// Get email metrics for a domain on a given date.
pub async fn get_email_metrics(
    db: &D1Database,
    tenant_id: &str,
    domain: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    let query = if let Some(d) = domain {
        let stmt = db.prepare(
            "SELECT action_type, SUM(count) as total FROM email_metrics WHERE tenant_id = ? AND domain = ? AND date >= date('now', '-7 days') GROUP BY action_type",
        );
        stmt.bind(&[tenant_id.into(), d.into()])?.all().await?
    } else {
        let stmt = db.prepare(
            "SELECT action_type, SUM(count) as total FROM email_metrics WHERE tenant_id = ? AND date >= date('now', '-7 days') GROUP BY action_type",
        );
        stmt.bind(&[tenant_id.into()])?.all().await?
    };
    query.results::<serde_json::Value>()
}

// ============================================================================
// Unified Message Storage
// ============================================================================

use crate::types::{Channel, ConversationContext, DiscordConfig, InboundMessage, OnboardingState};

/// Save a unified message to D1. No message content stored — metadata only.
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

pub async fn get_tenant_billing(kv: &kv::KvStore, tenant_id: &str) -> Result<TenantBilling> {
    let key = format!("billing:{tenant_id}");
    kv.get(&key)
        .json::<TenantBilling>()
        .await
        .map_err(|e| Error::from(e.to_string()))
        .map(|opt| opt.unwrap_or_default())
}

pub async fn save_tenant_billing(
    kv: &kv::KvStore,
    tenant_id: &str,
    billing: &TenantBilling,
) -> Result<()> {
    let key = format!("billing:{tenant_id}");
    let json =
        serde_json::to_string(billing).map_err(|e| Error::from(format!("JSON error: {e}")))?;
    kv.put(&key, json)?.execute().await?;
    Ok(())
}

// ============================================================================
// Credit Packs (D1)
// ============================================================================

pub async fn get_active_credit_packs(db: &D1Database) -> Result<Vec<CreditPackRow>> {
    let stmt = db.prepare("SELECT * FROM credit_packs WHERE active = 1 ORDER BY sort_order ASC");
    let result = stmt.all().await?;
    result.results()
}

pub async fn get_all_credit_packs(db: &D1Database) -> Result<Vec<CreditPackRow>> {
    let stmt = db.prepare("SELECT * FROM credit_packs ORDER BY sort_order ASC");
    let result = stmt.all().await?;
    result.results()
}

pub async fn get_credit_pack(db: &D1Database, id: i64) -> Result<Option<CreditPackRow>> {
    let stmt = db.prepare("SELECT * FROM credit_packs WHERE id = ?");
    stmt.bind(&[JsValue::from(id as f64)])?
        .first::<CreditPackRow>(None)
        .await
}

pub async fn save_credit_pack(
    db: &D1Database,
    name: &str,
    replies: i64,
    price_inr: i64,
    price_usd: i64,
    sort_order: i32,
) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO credit_packs (name, replies, price_inr, price_usd, sort_order)
         VALUES (?, ?, ?, ?, ?)",
    );
    stmt.bind(&[
        name.into(),
        JsValue::from(replies as f64),
        JsValue::from(price_inr as f64),
        JsValue::from(price_usd as f64),
        JsValue::from(sort_order as f64),
    ])?
    .run()
    .await?;
    Ok(())
}

pub async fn update_credit_pack(
    db: &D1Database,
    id: i64,
    name: &str,
    replies: i64,
    price_inr: i64,
    price_usd: i64,
    active: bool,
    sort_order: i32,
) -> Result<()> {
    let stmt = db.prepare(
        "UPDATE credit_packs SET name=?, replies=?, price_inr=?, price_usd=?, active=?, sort_order=? WHERE id=?",
    );
    stmt.bind(&[
        name.into(),
        JsValue::from(replies as f64),
        JsValue::from(price_inr as f64),
        JsValue::from(price_usd as f64),
        JsValue::from(if active { 1.0 } else { 0.0 }),
        JsValue::from(sort_order as f64),
        JsValue::from(id as f64),
    ])?
    .run()
    .await?;
    Ok(())
}

pub async fn delete_credit_pack(db: &D1Database, id: i64) -> Result<()> {
    let stmt = db.prepare("DELETE FROM credit_packs WHERE id = ?");
    stmt.bind(&[JsValue::from(id as f64)])?.run().await?;
    Ok(())
}
