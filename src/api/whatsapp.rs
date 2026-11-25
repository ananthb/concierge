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
    pub mode: &'static str,
    pub text: String,
    pub match_rule: String,
    pub wait_seconds: u32,
    pub bot_prompt: String,
}

#[derive(Serialize)]
pub struct BotPresetView {
    pub id: &'static str,
    pub label: &'static str,
    pub tag: &'static str,
    pub mode: &'static str,
    pub prompt: &'static str,
    pub match_rule: &'static str,
    pub canned_text: &'static str,
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
        use crate::handlers::health::Component;
        if !crate::handlers::health::WhatsAppSignup.ready(env) {
            return Ok(None);
        }
        let app_id = env
            .secret("META_APP_ID")
            .map(|s| s.to_string())
            .unwrap_or_default();
        let config_id = env
            .var("WHATSAPP_SIGNUP_CONFIG_ID")
            .map(|v| v.to_string())
            .unwrap_or_default();

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
        let (mode, text) = match &a.auto_reply.response {
            ReplyResponse::Ai => ("ai", String::new()),
            ReplyResponse::Canned { text } => ("canned", text.clone()),
        };
        AccountView {
            id: a.id.clone(),
            name: a.name.clone(),
            phone_number: a.phone_number.clone(),
            phone_number_id: a.phone_number_id.clone(),
            waba_id: a.waba_id.clone(),
            reply: ReplyView {
                enabled: a.auto_reply.enabled,
                mode,
                text,
                match_rule: a.auto_reply.match_rule.clone(),
                wait_seconds: a.auto_reply.wait_seconds,
                bot_prompt: a.auto_reply.bot_prompt.clone(),
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
            mode: p.mode,
            prompt: p.prompt,
            match_rule: p.match_rule,
            canned_text: p.canned_text,
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

            // Accept flat `{ name, enabled, mode, text, wait_seconds, bot_prompt }`
            // (and fall back to nested `reply` object if provided).
            let reply_src = body.get("reply").filter(|v| v.is_object()).unwrap_or(&body);

            if let Some(enabled) = reply_src.get("enabled").and_then(|v| v.as_bool()) {
                account.auto_reply.enabled = enabled;
            }
            if let Some(n) = super::field_i64(reply_src, "wait_seconds") {
                account.auto_reply.wait_seconds = n.clamp(0, 30) as u32;
            }

            let mut bot_prompt_updated = false;
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
                bot_prompt_updated = true;
            }

            if let Some(raw_rule) = reply_src.get("match_rule").and_then(|v| v.as_str()) {
                let rule = raw_rule.trim().to_string();
                if rule.chars().count() > crate::prompt::MAX_MATCH_RULE {
                    return super::invalid("Match rule must be 300 characters or fewer.");
                }
                account.auto_reply.match_rule = rule;
            }

            let mut need_ai_safety = false;
            if reply_src.get("mode").is_some() {
                let mode = super::field(reply_src, "mode");
                match mode.as_str() {
                    "ai" => {
                        account.auto_reply.response = ReplyResponse::Ai;
                        need_ai_safety = true;
                    }
                    "canned" => {
                        let text = super::field(reply_src, "text");
                        if text.is_empty() {
                            return super::invalid("Write the canned reply message.");
                        }
                        if text.chars().count() > crate::prompt::MAX_CANNED_TEXT {
                            return super::invalid(
                                "Canned reply must be 2000 characters or fewer.",
                            );
                        }
                        let content_hash = sha256_hex(&text);
                        let mut verdict = safety::classify_text(&env, &text).await;
                        if matches!(verdict, safety::SafetyVerdict::Approved)
                            && !account.auto_reply.match_rule.is_empty()
                        {
                            verdict =
                                safety::classify_text(&env, &account.auto_reply.match_rule).await;
                        }
                        account.auto_reply.response = ReplyResponse::Canned { text };
                        account.canned_safety = safety::verdict_to_state(verdict, content_hash);
                    }
                    _ => {
                        return super::invalid("Reply mode must be 'ai' or 'canned'.");
                    }
                }
            }

            if (need_ai_safety
                || (bot_prompt_updated && matches!(account.auto_reply.response, ReplyResponse::Ai)))
                && !account.auto_reply.bot_prompt.trim().is_empty()
            {
                let content_hash = sha256_hex(&account.auto_reply.bot_prompt);
                let verdict = safety::classify_text(&env, &account.auto_reply.bot_prompt).await;
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
