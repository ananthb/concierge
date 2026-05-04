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
        email_address_extras_purchased: row
            .get("email_address_extras_purchased")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        verified_at: row
            .get("verified_at")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
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
    let verified_val: JsValue = match &tenant.verified_at {
        Some(v) => v.as_str().into(),
        None => JsValue::NULL,
    };
    db.prepare(
        "INSERT INTO tenants (id, email, name, facebook_id, plan, locale, currency, email_address_extras_purchased, verified_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           email = excluded.email,
           name = excluded.name,
           facebook_id = excluded.facebook_id,
           plan = excluded.plan,
           locale = excluded.locale,
           currency = excluded.currency,
           email_address_extras_purchased = excluded.email_address_extras_purchased,
           verified_at = excluded.verified_at,
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
        JsValue::from(tenant.email_address_extras_purchased as f64),
        verified_val,
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
// Persona catalog (D1)
// ============================================================================

/// Default persona slug surfaced when the demo doesn't carry one.
pub const DEMO_DEFAULT_PERSONA_SLUG: &str = "concierge";

/// List rows from the archetypes catalog. `only_approved` filters by
/// `safety_status = 'approved'`. Used for the demo picker and the
/// tenant-onboarding starter set; the management UI passes `false` to
/// see drafts and rejections too.
pub async fn list_archetypes(
    db: &D1Database,
    only_approved: bool,
) -> Result<Vec<crate::types::Archetype>> {
    let stmt = if only_approved {
        "SELECT * FROM archetypes WHERE safety_status = 'approved' ORDER BY label ASC"
    } else {
        "SELECT * FROM archetypes ORDER BY label ASC"
    };
    let results = db.prepare(stmt).all().await?;
    let rows: Vec<serde_json::Value> = results.results()?;
    Ok(rows.iter().filter_map(row_to_archetype).collect())
}

pub async fn get_archetype(db: &D1Database, slug: &str) -> Result<Option<crate::types::Archetype>> {
    let row = db
        .prepare("SELECT * FROM archetypes WHERE slug = ?")
        .bind(&[slug.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(row.as_ref().and_then(row_to_archetype))
}

/// Backstop TTL for the per-slug KV cache. Writers (`upsert_archetype`,
/// `delete_archetype`, `set_archetype_safety` callers) invalidate
/// explicitly; the TTL just bounds drift if an invalidation is ever
/// missed.
const ARCHETYPE_CACHE_TTL_SECS: u64 = 300;

fn archetype_cache_key(slug: &str) -> String {
    format!("archetype:{slug}")
}

/// Hot-path archetype lookup. Reads from KV first; on miss, falls back
/// to D1 and writes the result back to KV with a short TTL. Writers
/// invalidate via [`invalidate_archetype_cache`] so the latest row
/// (including a since-Rejected safety verdict) propagates immediately.
pub async fn get_archetype_cached(
    kv: &kv::KvStore,
    db: &D1Database,
    slug: &str,
) -> Result<Option<crate::types::Archetype>> {
    let key = archetype_cache_key(slug);
    if let Ok(Some(s)) = kv.get(&key).text().await {
        if let Ok(a) = serde_json::from_str::<crate::types::Archetype>(&s) {
            return Ok(Some(a));
        }
    }
    let row = get_archetype(db, slug).await?;
    if let Some(ref a) = row {
        if let Ok(serialized) = serde_json::to_string(a) {
            if let Ok(put) = kv.put(&key, serialized) {
                let _ = put.expiration_ttl(ARCHETYPE_CACHE_TTL_SECS).execute().await;
            }
        }
    }
    Ok(row)
}

/// Drop the cached archetype row for `slug`. Call after every write
/// (`upsert_archetype`, `delete_archetype`, `set_archetype_safety`) so
/// the next reader gets fresh data.
pub async fn invalidate_archetype_cache(kv: &kv::KvStore, slug: &str) -> Result<()> {
    kv.delete(&archetype_cache_key(slug)).await?;
    Ok(())
}

/// Create or update an archetype row. The caller is responsible for
/// resetting the safety verdict to Draft and enqueueing a SafetyJob
/// before calling this. `upsert_archetype` itself is dumb.
pub async fn upsert_archetype(db: &D1Database, row: &crate::types::Archetype) -> Result<()> {
    let safety_status = match row.safety.status {
        crate::types::PersonaSafetyStatus::Approved => "approved",
        crate::types::PersonaSafetyStatus::Rejected => "rejected",
        crate::types::PersonaSafetyStatus::Pending => "draft",
    };
    let checked_at: JsValue = match &row.safety.checked_at {
        Some(s) => s.as_str().into(),
        None => JsValue::NULL,
    };
    let vague_reason: JsValue = match &row.safety.vague_reason {
        Some(s) => s.as_str().into(),
        None => JsValue::NULL,
    };
    let catch_phrases_json = serde_json::to_string(&row.catch_phrases)?;
    let off_topics_json = serde_json::to_string(&row.off_topics)?;
    let handoff_conditions_json = serde_json::to_string(&row.handoff_conditions)?;

    db.prepare(
        "INSERT INTO archetypes (slug, label, description, voice_prompt, greeting, default_rules_json,
                                catch_phrases_json, off_topics_json, never, handoff_conditions_json,
                                safety_status, safety_checked_at, safety_vague_reason,
                                created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
         ON CONFLICT(slug) DO UPDATE SET
           label = excluded.label,
           description = excluded.description,
           voice_prompt = excluded.voice_prompt,
           greeting = excluded.greeting,
           default_rules_json = excluded.default_rules_json,
           catch_phrases_json = excluded.catch_phrases_json,
           off_topics_json = excluded.off_topics_json,
           never = excluded.never,
           handoff_conditions_json = excluded.handoff_conditions_json,
           safety_status = excluded.safety_status,
           safety_checked_at = excluded.safety_checked_at,
           safety_vague_reason = excluded.safety_vague_reason,
           updated_at = datetime('now')",
    )
    .bind(&[
        row.slug.as_str().into(),
        row.label.as_str().into(),
        row.description.as_str().into(),
        row.voice_prompt.as_str().into(),
        row.greeting.as_str().into(),
        row.default_rules_json.as_str().into(),
        catch_phrases_json.into(),
        off_topics_json.into(),
        row.never.as_str().into(),
        handoff_conditions_json.into(),
        safety_status.into(),
        checked_at,
        vague_reason,
    ])?
    .run()
    .await?;
    Ok(())
}

/// Hard-delete an archetype row.
pub async fn delete_archetype(db: &D1Database, slug: &str) -> Result<()> {
    db.prepare("DELETE FROM archetypes WHERE slug = ?")
        .bind(&[slug.into()])?
        .run()
        .await?;
    Ok(())
}

/// Record the safety classifier's verdict for an archetype row. Called by
/// the queue consumer after the classifier returns.
pub async fn set_archetype_safety(
    db: &D1Database,
    slug: &str,
    verdict: &crate::safety::SafetyVerdict,
) -> Result<()> {
    let (status, reason): (&str, JsValue) = match verdict {
        crate::safety::SafetyVerdict::Approved => ("approved", JsValue::NULL),
        crate::safety::SafetyVerdict::Rejected { vague_reason } => {
            ("rejected", vague_reason.as_str().into())
        }
    };
    db.prepare(
        "UPDATE archetypes
            SET safety_status = ?, safety_checked_at = datetime('now'),
                safety_vague_reason = ?, updated_at = datetime('now')
            WHERE slug = ?",
    )
    .bind(&[status.into(), reason, slug.into()])?
    .run()
    .await?;
    Ok(())
}

fn row_to_archetype(row: &serde_json::Value) -> Option<crate::types::Archetype> {
    let slug = row.get("slug")?.as_str()?.to_string();
    let label = row.get("label")?.as_str()?.to_string();
    let description = row.get("description")?.as_str()?.to_string();
    let voice_prompt = row.get("voice_prompt")?.as_str()?.to_string();
    let greeting = row.get("greeting")?.as_str()?.to_string();
    let default_rules_json = row.get("default_rules_json")?.as_str()?.to_string();
    let catch_phrases = row
        .get("catch_phrases_json")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let off_topics = row
        .get("off_topics_json")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let never = row
        .get("never")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let handoff_conditions = row
        .get("handoff_conditions_json")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let safety_status = match row
        .get("safety_status")
        .and_then(|v| v.as_str())
        .unwrap_or("draft")
    {
        "approved" => crate::types::PersonaSafetyStatus::Approved,
        "rejected" => crate::types::PersonaSafetyStatus::Rejected,
        _ => crate::types::PersonaSafetyStatus::Pending,
    };
    let safety = crate::types::PersonaSafety {
        status: safety_status,
        checked_prompt_hash: None,
        checked_at: row
            .get("safety_checked_at")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        vague_reason: row
            .get("safety_vague_reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    };
    Some(crate::types::Archetype {
        slug,
        label,
        description,
        voice_prompt,
        greeting,
        default_rules_json,
        catch_phrases,
        off_topics,
        never,
        handoff_conditions,
        safety,
        created_at: row
            .get("created_at")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        updated_at: row
            .get("updated_at")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
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

    // Delete D1 data: messages + billing. Payments and audit_log are kept for dispute and tax records.
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

    // Nullify tenant_id in payments. The row stays for dispute and tax records.
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

use crate::types::{
    Channel, ConversationContext, DiscordConfig, InboundMessage, MessageAction, MessageDirection,
    OnboardingState,
};

/// Save a unified message to D1. No message content stored: metadata only.
///
/// `conversation_id` ties this row to a `Session.conversation_id`. AI
/// flows always pass `Some(...)`; canned-only or pre-AI inbound flows
/// pass `None` because no conversation exists yet.
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

/// Save a message from an InboundMessage struct.
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

/// Stamp an existing message row with its `conversation_id`. Called
/// from `handle_auto_reply` after we resolve the active conversation:
/// the inbound row was logged at pipeline entry (before we knew the
/// id), so we write it back here. Idempotent: re-running with the
/// same id is harmless.
///
/// Note: in the buffered-DO path, only the *last* inbound's row in a
/// batch gets stamped (it's the one carried as the synthetic
/// `msg.id`). Earlier rows in the same batch stay NULL.
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
// Conversation Sessions (KV)
// ============================================================================
//
// Stable-keyed per (tenant, channel, channel_account, sender). One
// record per customer-thread, holding the most recent inbound
// timestamp (for the idle-gap conversation boundary) and any
// in-progress handoff sub-state. Distinct from `ConversationContext`
// (which is per AI-draft-approval and short-lived).
//
// TTL is set generously above the idle gap so stale records don't
// outlive their purpose: a session that hasn't seen an inbound in
// twice the gap window is conceptually a closed conversation, even
// if the customer eventually returns.

const SESSION_TTL: u64 = 24 * 60 * 60; // 24h, comfortably above the 6h idle gap.

/// Compose the KV key for a thread's conversation-session record.
/// `sender` is hashed to keep keys ASCII-clean; not a privacy claim
/// (KV isn't user-facing).
///
/// Note: deliberately *not* under the `session:` prefix. That's
/// owned by auth sessions (see `save_session` above). Use `convsession:`
/// to keep the two namespaces from colliding.
fn conversation_session_key(
    tenant_id: &str,
    channel: &crate::types::Channel,
    channel_account_id: &str,
    sender: &str,
) -> String {
    let channel_str = match channel {
        crate::types::Channel::WhatsApp => "whatsapp",
        crate::types::Channel::Instagram => "instagram",
        crate::types::Channel::Email => "email",
        crate::types::Channel::Discord => "discord",
    };
    let sender_hash = crate::helpers::sha256_hex(sender);
    format!("convsession:{tenant_id}:{channel_str}:{channel_account_id}:{sender_hash}")
}

pub async fn get_conversation_session(
    kv: &kv::KvStore,
    tenant_id: &str,
    channel: &crate::types::Channel,
    channel_account_id: &str,
    sender: &str,
) -> Result<Option<crate::types::Session>> {
    let key = conversation_session_key(tenant_id, channel, channel_account_id, sender);
    kv.get(&key)
        .json::<crate::types::Session>()
        .await
        .map_err(|e| Error::from(e.to_string()))
}

pub async fn save_conversation_session(
    kv: &kv::KvStore,
    tenant_id: &str,
    channel: &crate::types::Channel,
    channel_account_id: &str,
    sender: &str,
    state: &crate::types::Session,
) -> Result<()> {
    let key = conversation_session_key(tenant_id, channel, channel_account_id, sender);
    let json = serde_json::to_string(state).map_err(|e| Error::from(format!("JSON error: {e}")))?;
    kv.put(&key, json)?
        .expiration_ttl(SESSION_TTL)
        .execute()
        .await?;
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

// ============================================================================
// Pricing config (single-row table)
// ============================================================================

/// What kind of price we're storing. The unit (minor or milli-minor) is
/// determined by the concept and is the same across every currency, so
/// operators can reason about a single number per concept-currency cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PricingConcept {
    /// Per-AI-reply rate, in milli-minor units (1/1000 of paise / cent / etc).
    /// Stored fine-grained so sub-minor prices fit (e.g. ₹0.10 = 10000 mp).
    UnitPriceMilli,
    /// Reply-email pack price per recurring period, in minor units.
    AddressPrice,
    /// Sign-up verification charge, in minor units.
    VerificationAmount,
}

impl PricingConcept {
    pub const ALL: [PricingConcept; 3] = [
        PricingConcept::UnitPriceMilli,
        PricingConcept::AddressPrice,
        PricingConcept::VerificationAmount,
    ];

    pub fn as_wire(self) -> &'static str {
        match self {
            PricingConcept::UnitPriceMilli => "unit_price_milli",
            PricingConcept::AddressPrice => "address_price",
            PricingConcept::VerificationAmount => "verification_amount",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "unit_price_milli" => Some(PricingConcept::UnitPriceMilli),
            "address_price" => Some(PricingConcept::AddressPrice),
            "verification_amount" => Some(PricingConcept::VerificationAmount),
            _ => None,
        }
    }

    /// True when amounts for this concept are stored as 1/1000 of the
    /// currency's minor unit. The slider needs this precision to offer
    /// fractions of a paise / cent. All other concepts are plain minor.
    pub fn is_milli(self) -> bool {
        matches!(self, PricingConcept::UnitPriceMilli)
    }

    /// Human label for the management form.
    pub fn label(self) -> &'static str {
        match self {
            PricingConcept::UnitPriceMilli => "Per-AI-reply rate",
            PricingConcept::AddressPrice => "Reply-email pack price",
            PricingConcept::VerificationAmount => "Sign-up verification charge",
        }
    }

    /// Short caption explaining the unit on the form.
    pub fn unit_caption(self) -> &'static str {
        if self.is_milli() {
            "milli-minor units (1/1000 of a paise / cent / fils)"
        } else {
            "minor units (paise / cents / fils)"
        }
    }
}

/// Operator-controlled pricing snapshot: currency-agnostic config plus a
/// `(concept, currency_code)` map keyed by ISO 4217 code. Every currency
/// uses the same unit per concept (see `PricingConcept::is_milli`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pricing {
    pub email_pack_size: i64,
    pub amounts: std::collections::BTreeMap<(PricingConcept, String), i64>,
}

impl Default for Pricing {
    fn default() -> Self {
        let mut amounts = std::collections::BTreeMap::new();
        // Mirrors the migration's seeded INR + USD rows so callers always
        // have a usable pricing snapshot even on a DB that skipped seeding.
        amounts.insert((PricingConcept::UnitPriceMilli, "INR".into()), 10_000);
        amounts.insert((PricingConcept::UnitPriceMilli, "USD".into()), 100);
        amounts.insert((PricingConcept::AddressPrice, "INR".into()), 9_900);
        amounts.insert((PricingConcept::AddressPrice, "USD".into()), 100);
        amounts.insert((PricingConcept::VerificationAmount, "INR".into()), 100);
        amounts.insert((PricingConcept::VerificationAmount, "USD".into()), 100);
        Self {
            email_pack_size: 5,
            amounts,
        }
    }
}

impl Pricing {
    /// Look up the stored amount for a concept-currency pair. The unit
    /// (minor vs milli-minor) is determined by `concept.is_milli()`.
    pub fn amount(&self, concept: PricingConcept, currency_code: &str) -> Option<i64> {
        self.amounts
            .get(&(concept, currency_code.to_uppercase()))
            .copied()
    }

    /// Per-AI-reply rate (milli-minor units) for a currency. Returns 0 if
    /// the currency hasn't been configured. Razorpay calls will fail
    /// loudly on a 0-amount order, which is the right behavior.
    pub fn unit_price_milli(&self, currency_code: &str) -> i64 {
        self.amount(PricingConcept::UnitPriceMilli, currency_code)
            .unwrap_or(0)
    }

    /// Reply-email pack price (minor units) for a currency.
    pub fn address_price(&self, currency_code: &str) -> i64 {
        self.amount(PricingConcept::AddressPrice, currency_code)
            .unwrap_or(0)
    }

    /// Sign-up verification charge (minor units) for a currency.
    pub fn verification_amount(&self, currency_code: &str) -> i64 {
        self.amount(PricingConcept::VerificationAmount, currency_code)
            .unwrap_or(0)
    }

    /// Sorted list of currency codes that have at least one amount row.
    pub fn currencies(&self) -> Vec<String> {
        let mut seen = std::collections::BTreeSet::new();
        for (_, code) in self.amounts.keys() {
            seen.insert(code.clone());
        }
        seen.into_iter().collect()
    }
}

/// Load the platform pricing snapshot: one singleton row for currency-
/// agnostic settings plus the per-(concept, currency) amount table.
pub async fn get_pricing(db: &D1Database) -> Pricing {
    let mut p = Pricing::default();

    // Currency-agnostic singleton.
    if let Ok(Some(row)) = db
        .prepare("SELECT email_pack_size FROM pricing_config WHERE id = 1")
        .first::<serde_json::Value>(None)
        .await
    {
        if let Some(n) = row.get("email_pack_size").and_then(|v| v.as_i64()) {
            p.email_pack_size = n;
        }
    }

    // Per-currency amounts. We treat the seeded defaults as a fallback so a
    // DB that lost a row still renders sane prices; rows that *do* exist
    // overwrite them.
    if let Ok(rs) = db.prepare("SELECT * FROM pricing_amount").all().await {
        if let Ok(rows) = rs.results::<serde_json::Value>() {
            for row in rows {
                let concept = row
                    .get("concept")
                    .and_then(|v| v.as_str())
                    .and_then(PricingConcept::from_wire);
                let code = row
                    .get("currency_code")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_uppercase());
                let amount = row.get("amount").and_then(|v| v.as_i64());
                if let (Some(c), Some(code), Some(a)) = (concept, code, amount) {
                    p.amounts.insert((c, code), a);
                }
            }
        }
    }

    p
}

/// Persist a single (concept, currency) cell. Used by the management form
/// for incremental edits. We don't update the whole table at once because
/// the operator may have just typed one value.
pub async fn upsert_pricing_amount(
    db: &D1Database,
    concept: PricingConcept,
    currency_code: &str,
    amount: i64,
) -> Result<()> {
    db.prepare(
        "INSERT INTO pricing_amount (concept, currency_code, amount) \
         VALUES (?, ?, ?) \
         ON CONFLICT(concept, currency_code) DO UPDATE SET amount = excluded.amount",
    )
    .bind(&[
        concept.as_wire().into(),
        currency_code.to_uppercase().as_str().into(),
        JsValue::from_f64(amount as f64),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Drop every row for a currency. Used when the operator removes a
/// currency from the pricing form (e.g. they no longer want to sell in EUR).
pub async fn delete_pricing_currency(db: &D1Database, currency_code: &str) -> Result<()> {
    db.prepare("DELETE FROM pricing_amount WHERE currency_code = ?")
        .bind(&[currency_code.to_uppercase().as_str().into()])?
        .run()
        .await?;
    Ok(())
}

/// Persist the currency-agnostic settings (just `email_pack_size` for now).
pub async fn update_pricing_config(db: &D1Database, email_pack_size: i64) -> Result<()> {
    db.prepare(
        "UPDATE pricing_config SET \
           email_pack_size = ?, \
           updated_at = datetime('now') \
         WHERE id = 1",
    )
    .bind(&[JsValue::from_f64(email_pack_size as f64)])?
    .run()
    .await?;
    Ok(())
}

// ============================================================================
// Scheduled grants
// ============================================================================

fn parse_scheduled_grant(row: &serde_json::Value) -> Option<crate::types::ScheduledGrant> {
    use crate::types::{GrantCadence, ScheduledGrant};

    let id = row.get("id")?.as_str()?.to_string();
    let cadence_wire = row.get("cadence")?.as_str()?;
    let cadence = GrantCadence::from_wire(cadence_wire)?;
    let credits = row.get("credits")?.as_i64()?;
    let expires_in_days = row
        .get("expires_in_days")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let last_run_at = row
        .get("last_run_at")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let next_run_at = row.get("next_run_at")?.as_str()?.to_string();
    let active = row
        .get("active")
        .and_then(|v| v.as_i64())
        .map(|n| n != 0)
        .unwrap_or(true);
    let created_at = row
        .get("created_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let updated_at = row
        .get("updated_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Some(ScheduledGrant {
        id,
        cadence,
        credits,
        expires_in_days,
        last_run_at,
        next_run_at,
        active,
        created_at,
        updated_at,
    })
}

pub async fn list_scheduled_grants(db: &D1Database) -> Result<Vec<crate::types::ScheduledGrant>> {
    let res = db
        .prepare("SELECT * FROM scheduled_grants ORDER BY created_at DESC")
        .all()
        .await?;
    let rows: Vec<serde_json::Value> = res.results()?;
    Ok(rows.iter().filter_map(parse_scheduled_grant).collect())
}

pub async fn list_due_scheduled_grants(
    db: &D1Database,
    now_iso: &str,
) -> Result<Vec<crate::types::ScheduledGrant>> {
    let res = db
        .prepare(
            "SELECT * FROM scheduled_grants \
             WHERE active = 1 AND next_run_at <= ? \
             ORDER BY next_run_at ASC",
        )
        .bind(&[now_iso.into()])?
        .all()
        .await?;
    let rows: Vec<serde_json::Value> = res.results()?;
    Ok(rows.iter().filter_map(parse_scheduled_grant).collect())
}

pub async fn insert_scheduled_grant(
    db: &D1Database,
    g: &crate::types::ScheduledGrant,
) -> Result<()> {
    db.prepare(
        "INSERT INTO scheduled_grants \
         (id, cadence, credits, expires_in_days, \
          last_run_at, next_run_at, active, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))",
    )
    .bind(&[
        g.id.as_str().into(),
        g.cadence.as_wire().as_str().into(),
        wasm_bindgen::JsValue::from_f64(g.credits as f64),
        wasm_bindgen::JsValue::from_f64(g.expires_in_days as f64),
        g.last_run_at
            .as_deref()
            .map(wasm_bindgen::JsValue::from_str)
            .unwrap_or(wasm_bindgen::JsValue::null()),
        g.next_run_at.as_str().into(),
        wasm_bindgen::JsValue::from_f64(if g.active { 1.0 } else { 0.0 }),
    ])?
    .run()
    .await?;
    Ok(())
}

pub async fn delete_scheduled_grant(db: &D1Database, id: &str) -> Result<()> {
    db.prepare("DELETE FROM scheduled_grants WHERE id = ?")
        .bind(&[id.into()])?
        .run()
        .await?;
    Ok(())
}

pub async fn record_scheduled_grant_run(
    db: &D1Database,
    id: &str,
    last_run_at: &str,
    next_run_at: &str,
) -> Result<()> {
    db.prepare(
        "UPDATE scheduled_grants \
         SET last_run_at = ?, next_run_at = ?, updated_at = datetime('now') \
         WHERE id = ?",
    )
    .bind(&[last_run_at.into(), next_run_at.into(), id.into()])?
    .run()
    .await?;
    Ok(())
}
