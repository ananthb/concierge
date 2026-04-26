//! Unified message processing pipeline.
//! All inbound messages from any channel flow through here.

use worker::*;

use crate::ai;
use crate::billing;
use crate::channel;
use crate::helpers::generate_id;
use crate::storage::*;
use crate::types::*;

/// Process an inbound message from WhatsApp, Instagram, or Discord.
///
/// Routes through the ReplyBufferDO so quick-fire messages from the same
/// sender batch into one AI call. wait_seconds=0 (or DO unreachable) falls
/// back to immediate processing.
pub async fn process_inbound(msg: &InboundMessage, env: &Env) -> Result<()> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    // 1. Log inbound to unified messages table
    if let Err(e) = save_inbound_message(&db, msg, None).await {
        console_log!("Failed to log inbound message: {:?}", e);
    }

    let wait = lookup_wait_seconds(&kv, msg).await.unwrap_or(0);
    if wait == 0 {
        process_inbound_immediate(msg, env).await?;
    } else if let Err(e) = forward_to_buffer(env, msg, wait).await {
        console_log!("buffer route failed, falling back to immediate: {:?}", e);
        process_inbound_immediate(msg, env).await?;
    }

    Ok(())
}

/// Process a single (possibly already-batched) message immediately, no
/// further buffering. Called both from `process_inbound` (when wait=0)
/// and from `ReplyBufferDO::alarm` after the wait window closes.
pub async fn process_inbound_immediate(msg: &InboundMessage, env: &Env) -> Result<()> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;
    handle_auto_reply(msg, &kv, &db, env).await
}

async fn lookup_wait_seconds(kv: &kv::KvStore, msg: &InboundMessage) -> Result<u32> {
    let cfg = match msg.channel {
        Channel::WhatsApp => get_whatsapp_account(kv, &msg.channel_account_id)
            .await?
            .map(|a| a.auto_reply),
        Channel::Instagram => get_instagram_account(kv, &msg.channel_account_id)
            .await?
            .map(|a| a.auto_reply),
        Channel::Discord => get_discord_config_by_tenant(kv, &msg.tenant_id)
            .await?
            .map(|c| c.auto_reply),
        Channel::Email => get_email_address(kv, &msg.tenant_id, &msg.channel_account_id)
            .await?
            .map(|a| a.auto_reply),
    };
    Ok(cfg.map(|c| c.wait_seconds).unwrap_or(0))
}

async fn forward_to_buffer(env: &Env, msg: &InboundMessage, wait_seconds: u32) -> Result<()> {
    let ns = env.durable_object("REPLY_BUFFER")?;
    // One DO per conversation: tenant + channel + sender.
    let id_name = format!("{}:{}:{}", msg.tenant_id, msg.channel.as_str(), msg.sender);
    let stub = ns.id_from_name(&id_name)?.get_stub()?;

    let payload = serde_json::json!({
        "msg": msg,
        "wait_seconds": wait_seconds,
    });
    let body = serde_json::to_string(&payload)?;

    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&body)));
    let req = Request::new_with_init("https://buffer.do/push", &init)?;
    let _ = stub.fetch_with_request(req).await?;
    Ok(())
}

/// Handle auto-reply for WhatsApp / Instagram / Email / Discord.
///
/// Pipeline:
///   1. Load the channel's `ReplyConfig`.
///   2. Skip if disabled.
///   3. Run prompt-injection scan on the body.
///   4. If any rule is `Prompt`-based, embed the body **once** for cosine
///      matching across all such rules.
///   5. Walk `rules` in order; first match wins. Otherwise the
///      mandatory `default_rule` fires.
///   6. Build the response: `Canned` → send verbatim (no AI, no credit);
///      `Prompt` → run the LLM with `persona prompt + rule prompt` (one credit).
///   7. AI replies are blocked unless the tenant's persona safety status
///      is `Approved` and unchanged.
async fn handle_auto_reply(
    msg: &InboundMessage,
    kv: &kv::KvStore,
    db: &D1Database,
    env: &Env,
) -> Result<()> {
    let config = match msg.channel {
        Channel::WhatsApp => get_whatsapp_account(kv, &msg.channel_account_id)
            .await?
            .map(|a| a.auto_reply),
        Channel::Instagram => get_instagram_account(kv, &msg.channel_account_id)
            .await?
            .filter(|a| a.enabled)
            .map(|a| a.auto_reply),
        Channel::Email => get_email_address(kv, &msg.tenant_id, &msg.channel_account_id)
            .await?
            .map(|a| a.auto_reply),
        Channel::Discord => get_discord_config_by_tenant(kv, &msg.tenant_id)
            .await?
            .map(|c| c.auto_reply),
    };

    let config = match config {
        Some(c) if c.enabled => c,
        _ => return Ok(()),
    };

    // Inbound text. Cap to limit injection surface; same value feeds the
    // injection scanner, the matcher, and the AI context.
    let safe_body: String = msg.body.chars().take(1000).collect();

    if ai::is_prompt_injection(env, &safe_body).await {
        console_log!(
            "Prompt injection detected from {} in tenant {}, skipping reply",
            msg.sender,
            msg.tenant_id
        );
        return Ok(());
    }

    // Embed once if any Prompt rule needs to be evaluated. Embedding errors
    // skip prompt-rule matching entirely (we fall through to keyword rules
    // and the default).
    let needs_embedding = config
        .rules
        .iter()
        .any(|r| matches!(r.matcher, ReplyMatcher::Prompt { .. }));
    let body_embedding = if needs_embedding {
        match ai::embed(env, &safe_body).await {
            Ok(v) => Some(v),
            Err(e) => {
                console_log!("Inbound embedding failed, prompt rules disabled: {:?}", e);
                None
            }
        }
    } else {
        None
    };

    // Pick the first matching rule, or fall back to the default.
    let matched: &ReplyRule = config
        .rules
        .iter()
        .find(|rule| matches_rule(&rule.matcher, &safe_body, body_embedding.as_deref()))
        .unwrap_or(&config.default_rule);

    // Load persona for AI-mode rules. Skip the load entirely when the
    // matched rule is canned — saves a KV hit on the hot keyword path.
    let needs_persona = matches!(matched.response, ReplyResponse::Prompt { .. });
    let persona = if needs_persona {
        Some(get_onboarding(kv, &msg.tenant_id).await?.persona)
    } else {
        None
    };

    let is_ai = matches!(matched.response, ReplyResponse::Prompt { .. });

    // Block AI replies unless the persona has been approved AND the prompt
    // hasn't drifted since approval.
    if is_ai {
        let safe = persona
            .as_ref()
            .map(|p| p.is_safe_to_use())
            .unwrap_or(false);
        if !safe {
            console_log!(
                "Persona not safety-approved for tenant {}, skipping AI reply",
                msg.tenant_id
            );
            return Ok(());
        }
    }

    if is_ai && !billing::try_deduct(db, &msg.tenant_id).await? {
        console_log!("Tenant {} out of AI-reply credits, skipping", msg.tenant_id);
        return Ok(());
    }

    let reply = match &matched.response {
        ReplyResponse::Canned { text } => text.clone(),
        ReplyResponse::Prompt { text: rule_prompt } => {
            let persona_prompt = persona
                .as_ref()
                .map(|p| p.active_prompt())
                .unwrap_or_default();
            let combined = if persona_prompt.is_empty() {
                rule_prompt.clone()
            } else {
                format!("{persona_prompt}\n\n{rule_prompt}")
            };

            let mut context = serde_json::Map::new();
            if let Some(ref name) = msg.sender_name {
                let safe_name: String = name.chars().take(100).collect();
                context.insert("sender_name".into(), serde_json::Value::String(safe_name));
            }
            context.insert(
                "message".into(),
                serde_json::Value::String(safe_body.clone()),
            );

            match ai::generate_response(env, &combined, &context).await {
                Ok(r) => r,
                Err(e) => {
                    console_log!("AI auto-reply error: {:?}", e);
                    if let Err(re) = billing::restore_credit(db, &msg.tenant_id).await {
                        console_log!("Failed to restore credit: {:?}", re);
                    }
                    return Ok(());
                }
            }
        }
    };

    if reply.is_empty() {
        if is_ai {
            if let Err(e) = billing::restore_credit(db, &msg.tenant_id).await {
                console_log!("Failed to restore credit: {:?}", e);
            }
        }
        return Ok(());
    }

    if let Err(e) = channel::send_reply(
        &msg.channel,
        env,
        &msg.raw_metadata,
        &msg.sender,
        &reply,
        None,
    )
    .await
    {
        console_log!("Auto-reply send error: {:?}", e);
        if is_ai {
            if let Err(re) = billing::restore_credit(db, &msg.tenant_id).await {
                console_log!("Failed to restore credit: {:?}", re);
            }
        }
        return Ok(());
    }

    if let Err(e) = save_message(
        db,
        &generate_id(),
        &msg.channel,
        "outbound",
        &msg.recipient,
        &msg.sender,
        &msg.tenant_id,
        &msg.channel_account_id,
        Some("auto_reply"),
    )
    .await
    {
        console_log!("Failed to log outbound message: {:?}", e);
    }

    Ok(())
}

/// Decide whether a single rule's matcher fires on the inbound text.
/// `body_embedding` is `None` if no Prompt rules exist or embedding failed —
/// in that case Prompt matchers can never fire.
fn matches_rule(matcher: &ReplyMatcher, body: &str, body_embedding: Option<&[f32]>) -> bool {
    match matcher {
        ReplyMatcher::Default => false, // default fires only via fallback path
        ReplyMatcher::StaticText { keywords } => {
            let lower = body.to_lowercase();
            keywords
                .iter()
                .any(|k| !k.is_empty() && lower.contains(&k.to_lowercase()))
        }
        ReplyMatcher::Prompt {
            embedding,
            threshold,
            ..
        } => {
            let Some(body_vec) = body_embedding else {
                return false;
            };
            if embedding.is_empty() {
                return false;
            }
            ai::cosine(body_vec, embedding) >= *threshold
        }
    }
}
