//! Discord slash command handlers. Currently just `/status`. The
//! domain/rule commands disappeared with the rule-engine rewrite.

use botrelay::discord::{Interaction, InteractionResponse};
use worker::*;

use crate::storage::*;

/// Dispatch a slash command.
pub async fn handle_command(interaction: &Interaction, env: &Env) -> Result<Response> {
    let name = interaction
        .data
        .as_ref()
        .and_then(|d| d.name.as_deref())
        .unwrap_or("");

    // Require Manage Server (0x20) permission
    if !interaction.member_has_permissions(0x20) {
        return ephemeral("You need the Manage Server permission to use this command.");
    }

    let guild_id = interaction.guild_id.as_deref().unwrap_or("");
    let kv = env.kv("KV")?;

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
        _ => ephemeral(&format!("Unknown command: {name}")),
    }
}

async fn handle_status(kv: &kv::KvStore, env: &Env, tenant_id: &str) -> Result<Response> {
    let wa_accounts = list_whatsapp_accounts(kv, tenant_id).await?;
    let ig_accounts = list_instagram_accounts(kv, tenant_id).await?;
    let addrs = get_email_addresses(kv, tenant_id).await?;
    let base_domain = env
        .var("EMAIL_DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_default();

    let addr_list: String = if addrs.is_empty() {
        "  (none)".to_string()
    } else {
        addrs
            .iter()
            .map(|a| format!("  - {}@{}", a.local_part, base_domain))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let msg = format!(
        "**Concierge Status**\n\n**WhatsApp**: {} account(s)\n**Instagram**: {} account(s)\n**Email addresses**:\n{}",
        wa_accounts.len(),
        ig_accounts.len(),
        addr_list,
    );
    ephemeral(&msg)
}

fn ephemeral(content: &str) -> Result<Response> {
    Response::from_json(&InteractionResponse::ephemeral_message(content))
}
