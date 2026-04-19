use worker::*;

use crate::crypto;
use crate::instagram;
use crate::storage::*;

pub async fn handle_scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    console_log!("Scheduled job started");

    // Refresh Instagram tokens
    if let Err(e) = refresh_instagram_tokens(&env).await {
        console_log!("Instagram token refresh error: {:?}", e);
    }

    console_log!("Scheduled job completed");
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

    for tenant_id in &seen_tenants {
        let accounts = list_instagram_accounts(&kv, tenant_id).await?;
        for account in accounts {
            if !account.enabled {
                continue;
            }

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
                    continue;
                }
            };

            if instagram::token_is_expired(&token) {
                console_log!("Token expired for Instagram account {}", account.id);
                continue;
            }

            if instagram::token_needs_refresh(&token) {
                match instagram::refresh_token(&token.access_token, &app_id, &app_secret).await {
                    Ok(new_token) => {
                        let encrypted = crypto::encrypt_token(&new_token, &encryption_key).await?;
                        kv.put(&token_key, encrypted)?.execute().await?;
                        console_log!("Refreshed token for Instagram account {}", account.id);
                    }
                    Err(e) => {
                        console_log!(
                            "Failed to refresh token for account {}: {:?}",
                            account.id,
                            e
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
