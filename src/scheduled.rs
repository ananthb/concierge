use worker::*;

use crate::crypto;
use crate::instagram;
use crate::storage::*;

pub async fn handle_scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    console_log!("Scheduled job started");

    // Verify email domain apex DNS records are correct
    let zone_id = env
        .var("EMAIL_ZONE_ID")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let base_domain = env
        .var("EMAIL_BASE_DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let api_token = env
        .secret("CF_DNS_API_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_default();
    if !zone_id.is_empty() && !api_token.is_empty() && !base_domain.is_empty() {
        if let Err(e) =
            crate::cloudflare::dns::verify_apex_web_records(&zone_id, &base_domain, &api_token)
                .await
        {
            console_log!("Email apex DNS verification failed: {:?}", e);
        }
    }

    // Check subscription health for email subdomains
    if let Err(e) = check_email_subscriptions(&env).await {
        console_log!("Email subscription check error: {:?}", e);
    }

    // Refresh Instagram tokens
    if let Err(e) = refresh_instagram_tokens(&env).await {
        console_log!("Instagram token refresh error: {:?}", e);
    }

    console_log!("Scheduled job completed");
}

/// Check all active email subdomain subscriptions against Razorpay.
/// Suspends any that have gone halted/cancelled (catches missed webhooks).
async fn check_email_subscriptions(env: &Env) -> Result<()> {
    let key_id = env
        .secret("RAZORPAY_KEY_ID")
        .map(|s| s.to_string())
        .unwrap_or_default();
    let key_secret = env
        .secret("RAZORPAY_KEY_SECRET")
        .map(|s| s.to_string())
        .unwrap_or_default();

    if key_id.is_empty() || key_secret.is_empty() {
        return Ok(());
    }

    let kv = env.kv("KV")?;
    let zone_id = env
        .var("EMAIL_ZONE_ID")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let base_domain = env
        .var("EMAIL_BASE_DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let api_token = env
        .secret("CF_DNS_API_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Iterate all tenants with email subdomains via KV list
    // KV list prefix "email_domains:" gives us all tenant email configs
    let list = kv
        .list()
        .prefix("email_domains:".to_string())
        .execute()
        .await?;

    for key in &list.keys {
        let tenant_id = key.name.strip_prefix("email_domains:").unwrap_or("");
        if tenant_id.is_empty() {
            continue;
        }

        let mut subdomains = get_email_subdomains(&kv, tenant_id).await?;
        let mut changed = false;

        for sub in subdomains.iter_mut() {
            // Only check active subdomains with a subscription
            if sub.status != crate::types::SubdomainStatus::Active {
                continue;
            }
            let sub_id = match &sub.subscription_id {
                Some(id) => id.clone(),
                None => continue,
            };

            let status =
                crate::billing::razorpay::get_subscription_status(&key_id, &key_secret, &sub_id)
                    .await
                    .unwrap_or_else(|_| "unknown".into());

            if status == "halted" || status == "cancelled" || status == "expired" {
                console_log!(
                    "Subscription {} for {}.{} is {} — suspending",
                    sub_id,
                    sub.label,
                    base_domain,
                    status
                );

                sub.status = crate::types::SubdomainStatus::Suspended;
                sub.updated_at = crate::helpers::now_iso();

                // Remove MX + web records
                if !sub.dns_record_ids.is_empty() && !zone_id.is_empty() && !api_token.is_empty() {
                    let _ = crate::cloudflare::dns::delete_dns_records(
                        &zone_id,
                        &sub.dns_record_ids,
                        &api_token,
                    )
                    .await;
                    sub.dns_record_ids.clear();
                }

                // Remove domain index
                let _ = delete_email_domain_index(&kv, &sub.domain).await;
                changed = true;
            }
        }

        if changed {
            save_email_subdomains(&kv, tenant_id, &subdomains).await?;
        }
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
