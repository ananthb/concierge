//! Discord slash command handlers (/domains, /rules, /status).

use worker::*;

use crate::storage::*;
use crate::types::*;

/// Dispatch a slash command.
pub async fn handle_command(interaction: &DiscordInteraction, env: &Env) -> Result<Response> {
    let name = interaction
        .data
        .as_ref()
        .and_then(|d| d.name.as_deref())
        .unwrap_or("");

    let guild_id = interaction.guild_id.as_deref().unwrap_or("");
    let kv = env.kv("KV")?;

    // Resolve tenant from guild
    let tenant_id = match get_discord_config_by_guild(&kv, guild_id).await? {
        Some(config) => config.tenant_id,
        None => {
            return ephemeral(
                "This server is not linked to a tenant. Configure via the web admin.",
            );
        }
    };

    match name {
        "status" => handle_status(&kv, env, &tenant_id).await,
        "domains" => handle_domains(interaction, &kv, &tenant_id).await,
        "rules" => handle_rules(interaction, &kv, &tenant_id).await,
        _ => ephemeral(&format!("Unknown command: {name}")),
    }
}

async fn handle_status(kv: &kv::KvStore, env: &Env, tenant_id: &str) -> Result<Response> {
    let wa_accounts = list_whatsapp_accounts(kv, tenant_id).await?;
    let ig_accounts = list_instagram_accounts(kv, tenant_id).await?;
    let domains = get_email_domains(kv, tenant_id).await?;

    let db = env.d1("DB")?;
    let metrics = get_email_metrics(&db, tenant_id, None).await?;

    let metrics_str: String = metrics
        .iter()
        .map(|m| {
            let action = m.get("action_type").and_then(|v| v.as_str()).unwrap_or("?");
            let total = m.get("total").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
            format!("  {action}: {total}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    let msg = format!(
        "**Concierge Status**\n\
        \n**WhatsApp**: {} account(s)\n\
        **Instagram**: {} account(s)\n\
        **Email domains**: {}\n\
        \n**Email metrics (7d)**:\n{}",
        wa_accounts.len(),
        ig_accounts.len(),
        domains
            .iter()
            .map(|d| d.domain.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        if metrics_str.is_empty() {
            "  No activity".to_string()
        } else {
            metrics_str
        },
    );

    ephemeral(&msg)
}

async fn handle_domains(
    interaction: &DiscordInteraction,
    kv: &kv::KvStore,
    tenant_id: &str,
) -> Result<Response> {
    let subcommand = interaction
        .data
        .as_ref()
        .and_then(|d| d.options.as_ref())
        .and_then(|opts| opts.first())
        .map(|o| o.name.as_str())
        .unwrap_or("list");

    match subcommand {
        "list" => {
            let domains = get_email_domains(kv, tenant_id).await?;
            if domains.is_empty() {
                return ephemeral("No email domains configured.");
            }
            let list: String = domains
                .iter()
                .map(|d| format!("- **{}**", d.domain))
                .collect::<Vec<_>>()
                .join("\n");
            ephemeral(&format!("**Email Domains**\n{list}"))
        }
        "add" => {
            let domain = interaction
                .data
                .as_ref()
                .and_then(|d| d.options.as_ref())
                .and_then(|opts| opts.first())
                .and_then(|o| o.options.first())
                .and_then(|o| o.value.as_ref())
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if domain.is_empty() {
                return ephemeral("Please provide a domain name.");
            }

            let domain = domain.to_lowercase();
            let mut domains = get_email_domains(kv, tenant_id).await?;

            if domains.iter().any(|d| d.domain == domain) {
                return ephemeral(&format!("Domain `{domain}` already exists."));
            }

            domains.push(EmailDomain {
                domain: domain.clone(),
                tenant_id: tenant_id.to_string(),
                default_action: EmailAction::Drop,
                created_at: crate::helpers::now_iso(),
            });
            save_email_domains(kv, tenant_id, &domains).await?;
            set_email_domain_index(kv, &domain, tenant_id).await?;

            ephemeral(&format!("Domain `{domain}` added."))
        }
        "remove" => {
            let domain = interaction
                .data
                .as_ref()
                .and_then(|d| d.options.as_ref())
                .and_then(|opts| opts.first())
                .and_then(|o| o.options.first())
                .and_then(|o| o.value.as_ref())
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if domain.is_empty() {
                return ephemeral("Please provide a domain name.");
            }

            let mut domains = get_email_domains(kv, tenant_id).await?;
            let before = domains.len();
            domains.retain(|d| d.domain != domain);

            if domains.len() == before {
                return ephemeral(&format!("Domain `{domain}` not found."));
            }

            save_email_domains(kv, tenant_id, &domains).await?;
            delete_email_domain_index(kv, domain).await?;
            save_email_rules(kv, tenant_id, domain, &[]).await?;

            ephemeral(&format!("Domain `{domain}` removed."))
        }
        _ => {
            ephemeral("Usage: `/domains list`, `/domains add <domain>`, `/domains remove <domain>`")
        }
    }
}

async fn handle_rules(
    interaction: &DiscordInteraction,
    kv: &kv::KvStore,
    tenant_id: &str,
) -> Result<Response> {
    let subcommand = interaction
        .data
        .as_ref()
        .and_then(|d| d.options.as_ref())
        .and_then(|opts| opts.first())
        .map(|o| o.name.as_str())
        .unwrap_or("list");

    let domain = interaction
        .data
        .as_ref()
        .and_then(|d| d.options.as_ref())
        .and_then(|opts| opts.first())
        .and_then(|o| o.options.first())
        .and_then(|o| o.value.as_ref())
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if domain.is_empty() {
        return ephemeral("Please provide a domain. Usage: `/rules list <domain>`");
    }

    match subcommand {
        "list" => {
            let rules = get_email_rules(kv, tenant_id, domain).await?;
            if rules.is_empty() {
                return ephemeral(&format!("No rules for `{domain}`."));
            }
            let list: String = rules
                .iter()
                .map(|r| {
                    let status = if r.enabled { "on" } else { "off" };
                    let action = match &r.action {
                        EmailAction::Drop => "drop".into(),
                        EmailAction::Spam { .. } => "spam".into(),
                        EmailAction::ForwardEmail { destination } => {
                            format!("fwd:{destination}")
                        }
                        EmailAction::ForwardDiscord { channel_id } => {
                            format!("discord:<#{channel_id}>")
                        }
                        EmailAction::AiReply { .. } => "ai_reply".into(),
                    };
                    format!(
                        "- [{}] **{}** (p:{}) → {} `{}`",
                        status,
                        r.name,
                        r.priority,
                        action,
                        r.id.chars().take(8).collect::<String>()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            ephemeral(&format!("**Rules for {domain}**\n{list}"))
        }
        _ => ephemeral(
            "Usage: `/rules list <domain>`\nFor complex rule management, use the web admin.",
        ),
    }
}

fn ephemeral(content: &str) -> Result<Response> {
    Response::from_json(&serde_json::json!({
        "type": 4,
        "data": {
            "content": content,
            "flags": 64
        }
    }))
}
