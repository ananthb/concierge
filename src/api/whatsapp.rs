//! `/api/whatsapp/*` — the tenant's connected WhatsApp numbers.
//!
//! Connecting a number goes through Meta's Embedded Signup, which is a
//! redirect flow the Worker still serves directly at `/whatsapp/signup/*`.
//! This module covers everything after that: listing what's connected,
//! editing the auto-reply, and disconnecting.

use serde::Serialize;
use worker::*;

use crate::helpers::{generate_token, now_iso, truncate};
use crate::storage::*;
use crate::types::*;

#[derive(Serialize)]
pub struct AccountView {
    pub id: String,
    pub name: String,
    pub phone_number: String,
    pub phone_number_id: String,
    pub reply: ReplyView,
    pub created_at: String,
}

/// The auto-reply settings for one number.
///
/// `mode` is `"canned"` or `"prompt"`; `text` is the canned message or the
/// AI instruction depending on which. Flattening the `ReplyResponse` enum
/// into two fields keeps the frontend's decoder trivial — it's a radio
/// button and a textarea.
#[derive(Serialize)]
pub struct ReplyView {
    pub enabled: bool,
    pub mode: &'static str,
    pub text: String,
    pub wait_seconds: u32,
}

/// Parameters for Meta's Embedded Signup JS SDK.
#[derive(Serialize)]
pub struct SignupView {
    pub app_id: String,
    pub config_id: String,
    /// One-shot CSRF nonce, minted per read and valid for 10 minutes. The
    /// signup callback refuses a state it can't find in KV.
    pub state: String,
}

impl SignupView {
    /// `None` when the operator hasn't configured WhatsApp signup, so the
    /// frontend can hide the button instead of rendering one that dead-ends
    /// in the callback.
    pub async fn resolve(
        env: &Env,
        kv: &kv::KvStore,
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
        kv.put(&format!("wa_signup_state:{state}"), tenant_id)?
            .expiration_ttl(600)
            .execute()
            .await?;

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
            ReplyResponse::Canned { text } => ("canned", text.clone()),
            ReplyResponse::Prompt { text } => ("prompt", text.clone()),
        };
        AccountView {
            id: a.id.clone(),
            name: a.name.clone(),
            phone_number: a.phone_number.clone(),
            phone_number_id: a.phone_number_id.clone(),
            reply: ReplyView {
                enabled: a.auto_reply.enabled,
                mode,
                text,
                wait_seconds: a.auto_reply.wait_seconds,
            },
            created_at: a.created_at.clone(),
        }
    }
}

#[derive(Serialize)]
struct ListView {
    accounts: Vec<AccountView>,
    signup: Option<SignupView>,
}

pub async fn handle(mut req: Request, env: Env, rest: &[&str]) -> Result<Response> {
    let tenant_id = match super::require_tenant(&req, &env).await {
        Ok(t) => t,
        Err(r) => return r,
    };
    let kv = env.kv("KV")?;
    let method = req.method();

    match (method, rest) {
        (Method::Get, []) => {
            let accounts = list_whatsapp_accounts(&kv, &tenant_id).await?;
            super::json(&ListView {
                accounts: accounts.iter().map(AccountView::from).collect(),
                signup: SignupView::resolve(&env, &kv, &tenant_id).await?,
            })
        }

        (Method::Put, [id]) => {
            let Some(mut account) = owned(&kv, &tenant_id, id).await? else {
                return super::not_found("WhatsApp number");
            };
            let body = match super::read_json(&mut req).await {
                Ok(b) => b,
                Err(r) => return r,
            };

            if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
                account.name = truncate(name.trim(), 200);
            }

            // Auto-reply. `mode` picks which arm of ReplyResponse to build;
            // an unknown mode falls back to canned, which cannot spend a
            // credit and cannot reach the model.
            if let Some(reply) = body.get("reply") {
                account.auto_reply.enabled = super::field_bool(reply, "enabled");
                let mode = super::field(reply, "mode");
                let text = truncate(&super::field(reply, "text"), 2000);
                if mode == "prompt" && text.is_empty() {
                    return super::invalid("Write the instruction the AI should follow.");
                }
                if mode == "canned" && text.is_empty() && account.auto_reply.enabled {
                    return super::invalid("Write the message to send.");
                }
                account.auto_reply.set_default_response(&mode, text);
                if let Some(n) = super::field_i64(reply, "wait_seconds") {
                    account.auto_reply.wait_seconds = n.clamp(0, 30) as u32;
                }
            }

            account.updated_at = now_iso();
            save_whatsapp_account(&kv, &account).await?;
            super::json(&AccountView::from(&account))
        }

        (Method::Delete, [id]) => {
            if owned(&kv, &tenant_id, id).await?.is_none() {
                return super::not_found("WhatsApp number");
            }
            delete_whatsapp_account(&kv, &tenant_id, id).await?;
            super::no_content()
        }

        _ => super::not_found("Endpoint"),
    }
}

/// Load an account only if this tenant owns it.
///
/// Ownership is checked here rather than by the caller so no endpoint can
/// forget: WhatsApp account ids are a flat KV namespace, so without this an
/// id from another tenant would load fine.
async fn owned(kv: &kv::KvStore, tenant_id: &str, id: &str) -> Result<Option<WhatsAppAccount>> {
    Ok(get_whatsapp_account(kv, id)
        .await?
        .filter(|a| a.tenant_id == tenant_id))
}
