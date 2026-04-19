//! Admin handlers for email routing: domain + rule CRUD

use worker::*;

use crate::helpers::*;
use crate::storage::*;
use crate::templates::admin_email::*;
use crate::types::*;

/// Handle /admin/email routes
pub async fn handle_email_admin(
    mut req: Request,
    env: Env,
    path: &str,
    _method: Method,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    let path_parts: Vec<&str> = path
        .strip_prefix("/admin/email")
        .unwrap_or("")
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let method = req.method();

    match (method, path_parts.as_slice()) {
        // Dashboard: list domains + metrics
        (Method::Get, []) => {
            let domains = get_email_domains(&kv, tenant_id).await?;
            let metrics = get_email_metrics(&db, tenant_id, None).await?;
            Response::from_html(email_dashboard_html(&domains, &metrics, base_url))
        }

        // Log viewer
        (Method::Get, ["log"]) => {
            let log = get_email_log(&db, tenant_id, 100).await?;
            Response::from_html(email_log_html(&log, base_url))
        }

        // Settings (discord bot token)
        (Method::Get, ["settings"]) => {
            let token = get_discord_bot_token(&kv, tenant_id).await?;
            Response::from_html(email_settings_html(token.as_deref(), base_url))
        }

        // Save discord bot token
        (Method::Put, ["settings"]) => {
            let form: serde_json::Value = req.json().await?;
            if let Some(token) = form.get("discord_bot_token").and_then(|v| v.as_str()) {
                save_discord_bot_token(&kv, tenant_id, token).await?;
            }
            Response::from_html("<div class=\"success\">Settings saved</div>".to_string())
        }

        // Add domain
        (Method::Post, ["domains"]) => {
            let form: serde_json::Value = req.json().await?;
            let domain = form
                .get("domain")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();

            if domain.is_empty() {
                return Response::from_html(
                    "<div class=\"error\">Domain cannot be empty</div>".to_string(),
                );
            }

            let mut domains = get_email_domains(&kv, tenant_id).await?;
            if domains.iter().any(|d| d.domain == domain) {
                return Response::from_html(
                    "<div class=\"error\">Domain already exists</div>".to_string(),
                );
            }

            let email_domain = EmailDomain {
                domain: domain.clone(),
                tenant_id: tenant_id.to_string(),
                default_action: EmailAction::Drop,
                created_at: now_iso(),
            };
            domains.push(email_domain);
            save_email_domains(&kv, tenant_id, &domains).await?;
            set_email_domain_index(&kv, &domain, tenant_id).await?;

            let headers = Headers::new();
            headers.set("HX-Redirect", &format!("{base_url}/admin/email"))?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Delete domain
        (Method::Delete, ["domains", domain]) => {
            let mut domains = get_email_domains(&kv, tenant_id).await?;
            domains.retain(|d| d.domain != *domain);
            save_email_domains(&kv, tenant_id, &domains).await?;
            delete_email_domain_index(&kv, domain).await?;
            // Clear rules for this domain too
            save_email_rules(&kv, tenant_id, domain, &[]).await?;

            Ok(Response::empty()?.with_status(200))
        }

        // List rules for a domain
        (Method::Get, ["domains", domain, "rules"]) => {
            let rules = get_email_rules(&kv, tenant_id, domain).await?;
            Response::from_html(email_rules_html(domain, &rules, base_url))
        }

        // Add rule
        (Method::Post, ["domains", domain, "rules"]) => {
            let form: serde_json::Value = req.json().await?;
            let mut rules = get_email_rules(&kv, tenant_id, domain).await?;

            let rule = parse_rule_from_json(domain, &form)?;
            rules.push(rule);
            rules.sort_by_key(|r| r.priority);
            save_email_rules(&kv, tenant_id, domain, &rules).await?;

            let headers = Headers::new();
            headers.set(
                "HX-Redirect",
                &format!("{base_url}/admin/email/domains/{domain}/rules"),
            )?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        // Edit rule page
        (Method::Get, ["domains", domain, "rules", rule_id]) => {
            let rules = get_email_rules(&kv, tenant_id, domain).await?;
            match rules.iter().find(|r| r.id == *rule_id) {
                Some(rule) => Response::from_html(email_rule_edit_html(domain, rule, base_url)),
                None => Response::error("Rule not found", 404),
            }
        }

        // Update rule
        (Method::Put, ["domains", domain, "rules", rule_id]) => {
            let form: serde_json::Value = req.json().await?;
            let mut rules = get_email_rules(&kv, tenant_id, domain).await?;

            if let Some(existing) = rules.iter_mut().find(|r| r.id == *rule_id) {
                let updated = parse_rule_from_json(domain, &form)?;
                existing.name = updated.name;
                existing.priority = updated.priority;
                existing.enabled = updated.enabled;
                existing.criteria = updated.criteria;
                existing.action = updated.action;
                existing.updated_at = now_iso();
            }

            rules.sort_by_key(|r| r.priority);
            save_email_rules(&kv, tenant_id, domain, &rules).await?;

            Response::from_html("<div class=\"success\">Rule updated</div>".to_string())
        }

        // Delete rule
        (Method::Delete, ["domains", domain, "rules", rule_id]) => {
            let mut rules = get_email_rules(&kv, tenant_id, domain).await?;
            rules.retain(|r| r.id != *rule_id);
            save_email_rules(&kv, tenant_id, domain, &rules).await?;

            Ok(Response::empty()?.with_status(200))
        }

        // Toggle rule enabled/disabled
        (Method::Post, ["domains", domain, "rules", rule_id, "toggle"]) => {
            let mut rules = get_email_rules(&kv, tenant_id, domain).await?;
            if let Some(rule) = rules.iter_mut().find(|r| r.id == *rule_id) {
                rule.enabled = !rule.enabled;
                rule.updated_at = now_iso();
            }
            save_email_rules(&kv, tenant_id, domain, &rules).await?;

            let headers = Headers::new();
            headers.set(
                "HX-Redirect",
                &format!("{base_url}/admin/email/domains/{domain}/rules"),
            )?;
            Ok(Response::empty()?.with_status(200).with_headers(headers))
        }

        _ => Response::error("Not Found", 404),
    }
}

/// Parse a routing rule from JSON form data.
fn parse_rule_from_json(domain: &str, form: &serde_json::Value) -> Result<RoutingRule> {
    let name = form
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unnamed rule")
        .to_string();
    let priority = form
        .get("priority")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let enabled = form.get("enabled").and_then(|v| v.as_str()) == Some("true");

    let criteria = MatchCriteria {
        from_pattern: non_empty_str(form, "from_pattern"),
        to_pattern: non_empty_str(form, "to_pattern"),
        subject_pattern: non_empty_str(form, "subject_pattern"),
        has_attachment: form
            .get("has_attachment")
            .and_then(|v| v.as_str())
            .map(|v| v == "true"),
        body_pattern: non_empty_str(form, "body_pattern"),
    };

    let action_type = form
        .get("action_type")
        .and_then(|v| v.as_str())
        .unwrap_or("drop");

    let action = match action_type {
        "drop" => EmailAction::Drop,
        "spam" => EmailAction::Spam {
            message: non_empty_str(form, "spam_message"),
        },
        "forward_email" => EmailAction::ForwardEmail {
            destination: form
                .get("destination")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        },
        "forward_discord" => EmailAction::ForwardDiscord {
            channel_id: form
                .get("channel_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        },
        "ai_reply" => EmailAction::AiReply {
            system_prompt: non_empty_str(form, "system_prompt"),
            approval_channel_id: non_empty_str(form, "approval_channel_id"),
            approval_email: non_empty_str(form, "approval_email"),
        },
        _ => EmailAction::Drop,
    };

    let now = now_iso();
    Ok(RoutingRule {
        id: generate_id(),
        domain: domain.to_string(),
        name,
        priority,
        enabled,
        criteria,
        action,
        created_at: now.clone(),
        updated_at: now,
    })
}

fn non_empty_str(form: &serde_json::Value, key: &str) -> Option<String> {
    form.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
}
