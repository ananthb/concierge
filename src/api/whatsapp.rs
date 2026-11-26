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

        (Method::Post, [id, "request-code"]) => {
            let Some(account) = owned(&db, &tenant_id, id).await? else {
                return super::not_found("WhatsApp number");
            };
            let body = req
                .json::<serde_json::Value>()
                .await
                .unwrap_or_else(|_| serde_json::json!({}));
            let raw_method = super::field(&body, "code_method").to_ascii_uppercase();
            let code_method = if raw_method == "VOICE" {
                "VOICE"
            } else {
                "SMS"
            };
            let payload = serde_json::json!({
                "code_method": code_method,
                "language": "en_US"
            });
            let url = format!(
                "https://graph.facebook.com/{}/{}/request_code",
                crate::META_API_VERSION,
                urlencoding::encode(&account.phone_number_id)
            );
            match graph_post_with_account_token(&env, &account, &url, Some(&payload)).await {
                Ok(()) => super::json(&serde_json::json!({ "ok": true })),
                Err(msg) => super::error("meta_error", &msg, 400),
            }
        }

        (Method::Post, [id, "verify-code"]) => {
            let Some(account) = owned(&db, &tenant_id, id).await? else {
                return super::not_found("WhatsApp number");
            };
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };
            let code: String = super::field(&body, "code")
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect();
            if code.is_empty() {
                return super::invalid("Enter the verification code sent to your number.");
            }
            let verify_url = format!(
                "https://graph.facebook.com/{}/{}/verify_code",
                crate::META_API_VERSION,
                urlencoding::encode(&account.phone_number_id)
            );
            let verify_payload = serde_json::json!({ "code": code });
            if let Err(msg) =
                graph_post_with_account_token(&env, &account, &verify_url, Some(&verify_payload))
                    .await
            {
                return super::error("meta_error", &msg, 400);
            }

            if let Err(msg) = register_and_subscribe(&env, &account).await {
                return super::error("meta_error", &msg, 400);
            }
            super::json(&serde_json::json!({ "ok": true }))
        }

        (Method::Post, [id, "register"]) => {
            let Some(account) = owned(&db, &tenant_id, id).await? else {
                return super::not_found("WhatsApp number");
            };
            match register_and_subscribe(&env, &account).await {
                Ok(()) => super::json(&serde_json::json!({ "ok": true })),
                Err(msg) => super::error("meta_error", &msg, 400),
            }
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

async fn register_and_subscribe(
    env: &Env,
    account: &WhatsAppAccount,
) -> std::result::Result<(), String> {
    if let Some(waba_id) = account.waba_id.as_deref().filter(|s| !s.trim().is_empty()) {
        let sub_url = format!(
            "https://graph.facebook.com/{}/{}/subscribed_apps",
            crate::META_API_VERSION,
            urlencoding::encode(waba_id.trim())
        );
        let _ = graph_post_with_account_token(env, account, &sub_url, None).await;
    }

    let reg_url = format!(
        "https://graph.facebook.com/{}/{}/register",
        crate::META_API_VERSION,
        urlencoding::encode(&account.phone_number_id)
    );
    let reg_payload = serde_json::json!({
        "messaging_product": "whatsapp",
        "pin": "000000"
    });
    match graph_post_with_account_token(env, account, &reg_url, Some(&reg_payload)).await {
        Ok(()) => Ok(()),
        Err(reg_err) => {
            let smb_url = format!(
                "https://graph.facebook.com/{}/{}/smb_app_data",
                crate::META_API_VERSION,
                urlencoding::encode(&account.phone_number_id)
            );
            let state_payload = serde_json::json!({
                "messaging_product": "whatsapp",
                "sync_type": "smb_app_state_sync"
            });
            let history_payload = serde_json::json!({
                "messaging_product": "whatsapp",
                "sync_type": "history"
            });
            if graph_post_with_account_token(env, account, &smb_url, Some(&state_payload))
                .await
                .is_ok()
            {
                let _ =
                    graph_post_with_account_token(env, account, &smb_url, Some(&history_payload))
                        .await;
                return Ok(());
            }
            Err(reg_err)
        }
    }
}

async fn graph_post_with_account_token(
    env: &Env,
    account: &WhatsAppAccount,
    url: &str,
    payload: Option<&serde_json::Value>,
) -> std::result::Result<(), String> {
    let platform_token = env
        .secret("WHATSAPP_ACCESS_TOKEN")
        .map(|s| s.to_string().trim().to_string())
        .unwrap_or_default();
    let acct_tok = account
        .auto_reply
        .access_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if let Some(tok) = acct_tok {
        match graph_post_once(url, tok, payload).await {
            Ok(()) => return Ok(()),
            Err(err) => {
                if !platform_token.is_empty() && platform_token != tok {
                    if let Ok(()) = graph_post_once(url, &platform_token, payload).await {
                        return Ok(());
                    }
                }
                return Err(err);
            }
        }
    }

    if platform_token.is_empty() {
        return Err("Missing WhatsApp access token. Reconnect your number first.".to_string());
    }
    graph_post_once(url, &platform_token, payload).await
}

async fn graph_post_once(
    url: &str,
    token: &str,
    payload: Option<&serde_json::Value>,
) -> std::result::Result<(), String> {
    let headers = Headers::new();
    headers
        .set("Authorization", &format!("Bearer {}", token.trim()))
        .map_err(|_| "Could not set Authorization header.".to_string())?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    if let Some(body) = payload {
        headers
            .set("Content-Type", "application/json")
            .map_err(|_| "Could not set Content-Type header.".to_string())?;
        init.with_body(Some(wasm_bindgen::JsValue::from_str(&body.to_string())));
    }
    init.with_headers(headers);
    let req =
        Request::new_with_init(url, &init).map_err(|_| "Could not build request.".to_string())?;
    let mut resp = Fetch::Request(req)
        .send()
        .await
        .map_err(|_| "Could not reach Meta Graph API.".to_string())?;
    if resp.status_code() < 300 {
        return Ok(());
    }
    let body_text = resp.text().await.unwrap_or_default();
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_text) {
        if let Some(err_obj) = json_val.get("error") {
            if let Some(user_msg) = err_obj
                .get("error_user_msg")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
            {
                return Err(user_msg.trim().to_string());
            }
            if let Some(msg) = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
            {
                return Err(msg.trim().to_string());
            }
        }
    }
    Err(format!("Meta API returned HTTP {}.", resp.status_code()))
}
