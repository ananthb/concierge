use worker::*;

use crate::crypto;
use crate::email::digest;
use crate::instagram;
use crate::storage::*;

/// Approval-digest sweep + 24h expiry. Mirror this string in `wrangler.toml`
/// and `.github/workflows/deploy.yml` so the deploy registers the trigger.
pub const CRON_DIGEST_SWEEP: &str = "*/15 * * * *";

/// Daily Instagram long-lived-token refresh. Same wrangler/workflow contract.
pub const CRON_INSTAGRAM_REFRESH: &str = "0 6 * * *";

/// Hourly scheduled-grant processor. Picks rows from `scheduled_grants`
/// whose next_run_at has passed and credits the targeted tenants.
pub const CRON_SCHEDULED_GRANTS: &str = "0 * * * *";

pub async fn handle_scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let cron = event.cron();
    console_log!("Scheduled job started: {cron}");

    match cron.as_str() {
        CRON_DIGEST_SWEEP => {
            if let Err(e) = digest::sweep(&env).await {
                console_log!("Digest sweep error: {:?}", e);
            }
        }
        CRON_INSTAGRAM_REFRESH => {
            if let Err(e) = refresh_instagram_tokens(&env).await {
                console_log!("Instagram token refresh error: {:?}", e);
            }
        }
        CRON_SCHEDULED_GRANTS => {
            if let Err(e) = process_scheduled_grants(&env).await {
                console_log!("Scheduled-grants processor error: {:?}", e);
            }
        }
        other => console_log!("Unknown cron schedule: {other}"),
    }

    console_log!("Scheduled job completed: {cron}");
}

/// Process every `scheduled_grants` row whose next_run_at has elapsed.
/// For each row: grant credits to the configured audience, log an audit
/// row per beneficiary, advance next_run_at by the cadence.
async fn process_scheduled_grants(env: &Env) -> Result<()> {
    let db = env.d1("DB")?;
    let now = crate::helpers::now_iso();
    let due = list_due_scheduled_grants(&db, &now).await?;
    if due.is_empty() {
        return Ok(());
    }
    console_log!("Processing {} due scheduled grant(s)", due.len());

    for g in due {
        let tenant_ids = match &g.audience {
            crate::types::GrantAudience::Everyone => list_tenants(&db)
                .await?
                .into_iter()
                .map(|t| t.id)
                .collect::<Vec<_>>(),
            crate::types::GrantAudience::Emails(emails) => {
                let mut ids = Vec::with_capacity(emails.len());
                for em in emails {
                    if let Some(t) = get_tenant_by_email(&db, em).await? {
                        ids.push(t.id);
                    } else {
                        console_log!("Scheduled grant {}: skipping unknown email {em}", g.id);
                    }
                }
                ids
            }
        };

        let mut granted_to = 0u32;
        for tid in &tenant_ids {
            let res = if g.expires_in_days <= 0 {
                crate::billing::grant_purchased(&db, tid, g.credits).await
            } else {
                crate::billing::grant_with_expiry(&db, tid, g.credits, g.expires_in_days).await
            };
            match res {
                Ok(_) => granted_to += 1,
                Err(e) => {
                    console_log!("Scheduled grant {}: tenant {tid} grant failed: {e:?}", g.id)
                }
            }
        }

        let next = crate::billing::cadence::next_run_after(&now, g.cadence);
        record_scheduled_grant_run(&db, &g.id, &now, &next).await?;
        let details = serde_json::json!({
            "credits": g.credits,
            "expires_in_days": g.expires_in_days,
            "tenant_count": granted_to,
            "next_run_at": next,
        });
        // System actor for cron-driven audit entries.
        crate::management::audit::log_action(
            &db,
            "system:cron",
            "scheduled_grant_run",
            "billing",
            Some(&g.id),
            Some(&details),
        )
        .await?;
        console_log!(
            "Scheduled grant {}: granted {granted_to} tenant(s) {credits} credit(s); next run {next}",
            g.id,
            credits = g.credits,
        );
    }
    Ok(())
}

async fn refresh_instagram_tokens(env: &Env) -> Result<()> {
    let kv = env.kv("KV")?;

    let encryption_key = env
        .secret("ENCRYPTION_KEY")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if encryption_key.is_empty() {
        return Ok(());
    }

    let app_id = env
        .secret("META_APP_ID")
        .map(|s| s.to_string())
        .unwrap_or_default();
    let app_secret = env
        .secret("META_APP_SECRET")
        .map(|s| s.to_string())
        .unwrap_or_default();

    // List all tenants by scanning kv prefix
    let tenant_list = kv
        .list()
        .prefix("tenant:".to_string())
        .execute()
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let mut seen_tenants = std::collections::HashSet::new();
    for key in &tenant_list.keys {
        // Extract tenant_id from "tenant:{id}:instagram:{account_id}" keys
        let parts: Vec<&str> = key.name.split(':').collect();
        if parts.len() >= 4 && parts[2] == "instagram" {
            seen_tenants.insert(parts[1].to_string());
        }
    }

    let mut total_accounts = 0u32;
    let mut refreshed = 0u32;
    let mut failures = 0u32;

    for tenant_id in &seen_tenants {
        let accounts = list_instagram_accounts(&kv, tenant_id).await?;
        for account in accounts {
            if !account.enabled {
                continue;
            }
            total_accounts += 1;

            let token_key = format!("instagram_token:{}", account.id);
            let encrypted_token = match kv.get(&token_key).text().await? {
                Some(t) => t,
                None => continue,
            };

            let token = match crypto::decrypt_token(&encrypted_token, &encryption_key).await {
                Ok(t) => t,
                Err(e) => {
                    console_log!(
                        "Failed to decrypt token for account {}: {:?}",
                        account.id,
                        e
                    );
                    failures += 1;
                    continue;
                }
            };

            if instagram::token_is_expired(&token) {
                console_log!("Token expired for Instagram account {}", account.id);
                failures += 1;
                continue;
            }

            if instagram::token_needs_refresh(&token) {
                match instagram::refresh_token(&token.access_token, &app_id, &app_secret).await {
                    Ok(new_token) => {
                        let encrypted = crypto::encrypt_token(&new_token, &encryption_key).await?;
                        kv.put(&token_key, encrypted)?.execute().await?;
                        refreshed += 1;
                    }
                    Err(e) => {
                        console_log!(
                            "Failed to refresh token for account {}: {:?}",
                            account.id,
                            e
                        );
                        failures += 1;
                    }
                }
            }
        }
    }

    console_log!(
        "Instagram token refresh: {} accounts, {} refreshed, {} failures",
        total_accounts,
        refreshed,
        failures
    );

    Ok(())
}
