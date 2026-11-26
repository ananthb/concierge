//! `/api/whatsapp/*` — the tenant's connected WhatsApp numbers.

use serde::Serialize;
use worker::*;

use crate::helpers::{generate_token, now_iso, sha256_hex, truncate};
use crate::safety;
use crate::storage::*;
use crate::types::*;

#[derive(Serialize)]
pub struct AccountView {
    pub id: String,
    pub name: String,
    pub phone_number: String,
    pub phone_number_id: String,
    pub waba_id: Option<String>,
    pub reply: ReplyView,
    pub canned_safety_status: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ReplyView {
    pub enabled: bool,
    pub bot_prompt: String,
    pub rules: Vec<StaticRule>,
    pub wait_seconds: u32,
}

#[derive(Serialize)]
pub struct BotPresetView {
    pub id: &'static str,
    pub label: &'static str,
    pub tag: &'static str,
    pub business_name: &'static str,
    pub prompt: &'static str,
    pub rules: &'static [crate::prompt::PresetRule],
    pub opener: &'static str,
}

#[derive(Serialize)]
pub struct SignupView {
    pub app_id: String,
    pub config_id: String,
    pub state: String,
}

impl SignupView {
    pub async fn resolve(
        env: &Env,
        db: &D1Database,
        tenant_id: &str,
    ) -> Result<Option<SignupView>> {
        let app_id = env
            .var("META_APP_ID")
            .map(|v| v.to_string())
            .unwrap_or_default()
            .trim()
            .to_string();
        let config_id = env
            .var("WHATSAPP_SIGNUP_CONFIG_ID")
            .map(|v| v.to_string())
            .unwrap_or_default()
            .trim()
            .to_string();
        if app_id.is_empty() {
            return Ok(None);
        }

        let state = generate_token()?;
        save_signup_state(db, &state, tenant_id, 600).await?;

        Ok(Some(SignupView {
            app_id,
            config_id,
            state,
        }))
    }
}

impl From<&WhatsAppAccount> for AccountView {
    fn from(a: &WhatsAppAccount) -> Self {
        AccountView {
            id: a.id.clone(),
            name: a.name.clone(),
            phone_number: a.phone_number.clone(),
            phone_number_id: a.phone_number_id.clone(),
            waba_id: a.waba_id.clone(),
            reply: ReplyView {
                enabled: a.auto_reply.enabled,
                bot_prompt: a.auto_reply.bot_prompt.clone(),
                rules: a.auto_reply.effective_rules(),
                wait_seconds: a.auto_reply.wait_seconds,
            },
            canned_safety_status: a.canned_safety.status.as_str().to_string(),
            created_at: a.created_at.clone(),
        }
    }
}

#[derive(Serialize)]
struct ListView {
    accounts: Vec<AccountView>,
    signup: Option<SignupView>,
    bot_presets: Vec<BotPresetView>,
}

fn bot_presets() -> Vec<BotPresetView> {
    crate::prompt::BOT_PERSONAS
        .iter()
        .map(|p| BotPresetView {
            id: p.id,
            label: p.label,
            tag: p.tag,
            business_name: p.business_name,
            prompt: p.prompt,
            rules: p.rules,
            opener: p.opener,
        })
        .collect()
}

pub async fn handle(mut req: Request, env: Env, rest: &[&str]) -> Result<Response> {
    let tenant_id = match super::require_tenant(&req, &env).await {
        Ok(t) => t,
        Err(r) => return r,
    };
    let db = env.d1("DB")?;
    let method = req.method();

    match (method, rest) {
        (Method::Get, []) => {
            let accounts = list_whatsapp_accounts(&db, &tenant_id).await?;
            super::json(&ListView {
                accounts: accounts.iter().map(AccountView::from).collect(),
                signup: SignupView::resolve(&env, &db, &tenant_id).await?,
                bot_presets: bot_presets(),
            })
        }

        (Method::Put, [id]) => {
            let Some(mut account) = owned(&db, &tenant_id, id).await? else {
                return super::not_found("WhatsApp number");
            };
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };

            if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
                account.name = truncate(name.trim(), 200);
            }

            // Accept flat `{ name, enabled, bot_prompt, rules, wait_seconds }`
            // (and fall back to nested `reply` object if provided).
            let reply_src = body.get("reply").filter(|v| v.is_object()).unwrap_or(&body);

            if let Some(enabled) = reply_src.get("enabled").and_then(|v| v.as_bool()) {
                account.auto_reply.enabled = enabled;
            }
            if let Some(n) = super::field_i64(reply_src, "wait_seconds") {
                account.auto_reply.wait_seconds = n.clamp(0, 30) as u32;
            }

            let mut safety_dirty = false;
            if let Some(raw_prompt) = reply_src.get("bot_prompt").and_then(|v| v.as_str()) {
                let bp = raw_prompt.trim().to_string();
                if bp.chars().count() > crate::prompt::MAX_BOT_PROMPT {
                    return super::invalid("Bot prompt must be 600 characters or fewer.");
                }
                account.auto_reply.bot_prompt = if bp.is_empty() {
                    crate::prompt::default_bot_prompt()
                } else {
                    bp
                };
                safety_dirty = true;
            }

            if let Some(raw_rules_val) = reply_src.get("rules") {
                let raw_rules: Vec<StaticRule> = match serde_json::from_value(raw_rules_val.clone())
                {
                    Ok(r) => r,
                    Err(_) => {
                        return super::invalid(
                            "Rules must be an array of { match_rule, reply_text }.",
                        )
                    }
                };
                let cleaned = match crate::prompt::validate_static_rules(&raw_rules) {
                    Ok(c) => c,
                    Err(msg) => return super::invalid(msg),
                };
                account.auto_reply.rules = cleaned;
                account.auto_reply.response = None;
                account.auto_reply.match_rule.clear();
                safety_dirty = true;
            }

            if safety_dirty {
                let combined = account.auto_reply.safety_text();
                let content_hash = sha256_hex(&combined);
                let verdict = safety::classify_text(&env, &combined).await;
                account.canned_safety = safety::verdict_to_state(verdict, content_hash);
            }

            account.updated_at = now_iso();
            save_whatsapp_account(&db, &account).await?;
            super::json(&AccountView::from(&account))
        }

        (Method::Delete, [id]) => {
            if owned(&db, &tenant_id, id).await?.is_none() {
                return super::not_found("WhatsApp number");
            }
            delete_whatsapp_account(&db, &tenant_id, id).await?;
            super::no_content()
        }

        _ => super::not_found("Endpoint"),
    }
}

async fn owned(db: &D1Database, tenant_id: &str, id: &str) -> Result<Option<WhatsAppAccount>> {
    Ok(get_whatsapp_account(db, id)
        .await?
        .filter(|a| a.tenant_id == tenant_id))
}
