//! Discord OAuth install + per-tenant management.
//!
//! Flow:
//!   1. User clicks Connect on wizard Channels step or Settings.
//!   2. GET /admin/discord/install generates a one-shot state token in KV
//!      and 302s to Discord's `oauth2/authorize` with `scope=bot
//!      applications.commands` and a permissions bitfield.
//!   3. Discord redirects to /auth/discord/callback with ?code, ?state,
//!      ?guild_id. We verify state, record `guild_id -> tenant_id` via
//!      `save_discord_config`, then 302 to /admin/discord for the channel
//!      picker.
//!   4. Picker is a small form on /admin/discord. PUT /admin/discord/config
//!      saves approval/digest/relay channel IDs back to `DiscordConfig`.
//!   5. DELETE /admin/discord uninstalls: removes KV entries and asks the
//!      bot to leave the guild.

use worker::*;

use crate::helpers::*;
use crate::storage::*;
use crate::types::*;

const DISCORD_AUTHORIZE_URL: &str = "https://discord.com/api/oauth2/authorize";
/// SEND_MESSAGES | VIEW_CHANNEL | READ_MESSAGE_HISTORY | ADD_REACTIONS | MANAGE_MESSAGES
const BOT_PERMISSIONS: u64 = 76928;
const OAUTH_STATE_TTL: u64 = 600;

/// Handle /admin/discord/*: tenant-scoped Discord management.
pub async fn handle_discord_admin(
    mut req: Request,
    env: Env,
    path: &str,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let kv = env.kv("KV")?;
    let sub = path
        .strip_prefix("/admin/discord")
        .unwrap_or("")
        .trim_start_matches('/');
    let method = req.method();

    match (method, sub) {
        // Overview / channel picker.
        (Method::Get, "" | "/") => show_manage_page(&req, &env, &kv, base_url, tenant_id).await,

        // Kick off OAuth install.
        (Method::Get, "install") => start_install(&req, &env, &kv, base_url, tenant_id).await,

        // Save channel-id selections.
        (Method::Put, "config") => save_channels(&mut req, &kv, tenant_id).await,

        // Uninstall: remove from KV and ask the bot to leave the guild.
        (Method::Delete, "" | "/") => uninstall(&env, &kv, base_url, tenant_id).await,

        _ => Response::error("Not Found", 404),
    }
}

/// Public callback at /auth/discord/callback (inside the /auth/* dispatcher).
/// Relies on the session cookie to identify the tenant, plus a one-shot state
/// token for CSRF protection.
pub async fn handle_discord_callback(req: Request, env: Env) -> Result<Response> {
    let kv = env.kv("KV")?;
    let tenant_id = match super::auth::resolve_tenant_id(&req, &kv).await {
        Some(id) => id,
        None => {
            let headers = Headers::new();
            headers.set("Location", "/auth/login")?;
            return Ok(Response::empty()?.with_status(302).with_headers(headers));
        }
    };
    let base_url = super::get_base_url(&req);

    let url = req.url()?;
    let query: std::collections::HashMap<_, _> = url.query_pairs().collect();

    if let Some(err) = query.get("error") {
        return Response::error(format!("Discord returned error: {err}"), 400);
    }

    let state = query
        .get("state")
        .ok_or_else(|| Error::from("Missing state"))?
        .to_string();
    let guild_id = query
        .get("guild_id")
        .ok_or_else(|| Error::from("Missing guild_id (did the user pick a server?)"))?
        .to_string();

    let state_key = format!("discord_oauth_state:{state}");
    let state_raw = kv
        .get(&state_key)
        .text()
        .await?
        .ok_or_else(|| Error::from("Invalid or expired state"))?;
    kv.delete(&state_key).await?;

    let state_data: serde_json::Value = serde_json::from_str(&state_raw)?;
    let state_tenant = state_data
        .get("tenant_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if state_tenant != tenant_id {
        return Response::error("Session / state tenant mismatch", 403);
    }
    let from = state_data
        .get("from")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let bot_token = env
        .secret("DISCORD_BOT_TOKEN")
        .map(|s| s.to_string())
        .unwrap_or_default();
    let guild_name = if bot_token.is_empty() {
        None
    } else {
        let bot = botrelay::discord::DiscordBot::new(&bot_token, "", "");
        bot.fetch_guild(&guild_id).await.and_then(|g| g.name)
    };

    let config = DiscordConfig {
        guild_id: guild_id.clone(),
        tenant_id: tenant_id.clone(),
        guild_name,
        approval_channel_id: None,
        digest_channel_id: None,
        relay_channel_id: None,
        inbound_mentions: false,
        inbound_channel_ids: Vec::new(),
        auto_reply: AutoReplyConfig::default(),
    };
    save_discord_config(&kv, &config).await?;

    // Land on the manage/picker page so the user can pick channels right away.
    let dest = if from.is_empty() {
        format!("{base_url}/admin/discord")
    } else {
        format!(
            "{base_url}/admin/discord?from={}",
            urlencoding::encode(&from)
        )
    };
    let headers = Headers::new();
    headers.set("Location", &dest)?;
    Ok(Response::empty()?.with_status(302).with_headers(headers))
}

// ----------------------------------------------------------------------------
// Admin handler helpers
// ----------------------------------------------------------------------------

async fn start_install(
    req: &Request,
    env: &Env,
    kv: &kv::KvStore,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let app_id = env
        .secret("DISCORD_APPLICATION_ID")
        .map(|s| s.to_string())
        .unwrap_or_default();
    if app_id.is_empty() {
        return Response::error("Discord integration not configured", 500);
    }

    // Round-trip ?from through state so we can send the user back to the
    // right wizard step after install + picker.
    let url = req.url()?;
    let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let from = query.get("from").map(|s| s.to_string()).unwrap_or_default();

    let state = generate_token()?;
    let state_data = serde_json::json!({ "tenant_id": tenant_id, "from": from }).to_string();
    kv.put(&format!("discord_oauth_state:{state}"), state_data)?
        .expiration_ttl(OAUTH_STATE_TTL)
        .execute()
        .await?;

    let redirect_uri = format!("{base_url}/auth/discord/callback");
    let authorize = format!(
        "{DISCORD_AUTHORIZE_URL}?client_id={client_id}&scope={scope}&permissions={perms}&redirect_uri={redirect}&response_type=code&state={state}",
        client_id = urlencoding::encode(&app_id),
        scope = urlencoding::encode("bot applications.commands"),
        perms = BOT_PERMISSIONS,
        redirect = urlencoding::encode(&redirect_uri),
        state = urlencoding::encode(&state),
    );

    let headers = Headers::new();
    headers.set("Location", &authorize)?;
    Ok(Response::empty()?.with_status(302).with_headers(headers))
}

async fn show_manage_page(
    req: &Request,
    env: &Env,
    kv: &kv::KvStore,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    let cfg = get_discord_config_by_tenant(kv, tenant_id).await?;
    let url = req.url()?;
    let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let from = query.get("from").map(|s| s.to_string()).unwrap_or_default();

    match cfg {
        None => {
            // Not installed: render a simple CTA that kicks off the install.
            Response::from_html(crate::templates::discord::install_cta_html(&from, base_url))
        }
        Some(cfg) => {
            let bot_token = env
                .secret("DISCORD_BOT_TOKEN")
                .map(|s| s.to_string())
                .unwrap_or_default();
            let channels = if bot_token.is_empty() {
                Vec::new()
            } else {
                let bot = botrelay::discord::DiscordBot::new(&bot_token, "", "");
                bot.list_guild_text_channels(&cfg.guild_id)
                    .await
                    .unwrap_or_default()
            };
            Response::from_html(crate::templates::discord::manage_html(
                &cfg, &channels, &from, base_url,
            ))
        }
    }
}

async fn save_channels(req: &mut Request, kv: &kv::KvStore, tenant_id: &str) -> Result<Response> {
    let form: serde_json::Value = req.json().await?;
    let mut cfg = match get_discord_config_by_tenant(kv, tenant_id).await? {
        Some(c) => c,
        None => return Response::error("Discord not installed", 400),
    };

    let opt_str = |key: &str| {
        form.get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    cfg.approval_channel_id = opt_str("approval_channel_id");
    cfg.digest_channel_id = opt_str("digest_channel_id");
    cfg.relay_channel_id = opt_str("relay_channel_id");

    // Inbound triggers + AI auto-reply config
    cfg.inbound_mentions = form
        .get("inbound_mentions")
        .map(|v| v.as_bool().unwrap_or_else(|| v.as_str() == Some("true")))
        .unwrap_or(false);
    cfg.inbound_channel_ids = match form.get("inbound_channel_ids") {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        Some(serde_json::Value::String(s)) if !s.is_empty() => vec![s.clone()],
        _ => Vec::new(),
    };
    cfg.auto_reply.enabled = form
        .get("auto_reply_enabled")
        .map(|v| v.as_bool().unwrap_or_else(|| v.as_str() == Some("true")))
        .unwrap_or(false);
    if let Some(mode) = form.get("auto_reply_mode").and_then(|v| v.as_str()) {
        cfg.auto_reply.mode = match mode {
            "ai" => AutoReplyMode::Ai,
            _ => AutoReplyMode::Static,
        };
    }
    if let Some(p) = form.get("auto_reply_prompt").and_then(|v| v.as_str()) {
        cfg.auto_reply.prompt = p.chars().take(2000).collect();
    }
    if let Some(n) = form.get("wait_seconds").and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    }) {
        cfg.auto_reply.wait_seconds = (n as u32).min(30);
    }

    save_discord_config(kv, &cfg).await?;
    Response::from_html(r#"<div class="success">Channels saved.</div>"#.to_string())
}

async fn uninstall(
    env: &Env,
    kv: &kv::KvStore,
    base_url: &str,
    tenant_id: &str,
) -> Result<Response> {
    if let Some(cfg) = get_discord_config_by_tenant(kv, tenant_id).await? {
        kv.delete(&format!("discord_guild:{}", cfg.guild_id))
            .await?;
        kv.delete(&format!("discord_config:{tenant_id}")).await?;
        // Best-effort: have the bot leave the guild.
        let bot_token = env
            .secret("DISCORD_BOT_TOKEN")
            .map(|s| s.to_string())
            .unwrap_or_default();
        if !bot_token.is_empty() {
            let bot = botrelay::discord::DiscordBot::new(&bot_token, "", "");
            let _ = bot.leave_guild(&cfg.guild_id).await;
        }
    }

    let headers = Headers::new();
    headers.set("HX-Redirect", &format!("{base_url}/admin/discord"))?;
    Ok(Response::empty()?.with_status(200).with_headers(headers))
}
