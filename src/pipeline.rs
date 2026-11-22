//! Unified message processing pipeline.
//! All inbound messages from any channel flow through here.

use worker::*;

use crate::ai;
use crate::billing;
use crate::channel;
use crate::helpers::generate_id;
use crate::storage::*;
use crate::types::*;

/// Process an inbound message, optionally batching via `ReplyBufferDO`.
pub async fn process_inbound(msg: &InboundMessage, env: &Env) -> Result<()> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;

    if let Err(e) = save_inbound_message(&db, msg, None).await {
        console_log!("Failed to log inbound message: {:?}", e);
    }

    let wait = get_channel_reply_config(&kv, &msg.channel, &msg.channel_account_id)
        .await?
        .map(|c| c.wait_seconds)
        .unwrap_or(0);

    if wait == 0 {
        process_inbound_immediate(msg, env).await?;
    } else if let Err(e) = forward_to_buffer(env, msg, wait).await {
        console_log!("buffer route failed, falling back to immediate: {:?}", e);
        process_inbound_immediate(msg, env).await?;
    }

    Ok(())
}

/// Process a single (possibly already-batched) message immediately.
pub async fn process_inbound_immediate(msg: &InboundMessage, env: &Env) -> Result<()> {
    let kv = env.kv("KV")?;
    let db = env.d1("DB")?;
    handle_auto_reply(msg, &kv, &db, env).await
}

async fn forward_to_buffer(env: &Env, msg: &InboundMessage, wait_seconds: u32) -> Result<()> {
    let ns = env.durable_object("REPLY_BUFFER")?;
    let id_name = format!(
        "{}:{}:{}",
        msg.tenant_id,
        msg.channel.as_str(),
        msg.conversation_key()
    );
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

/// Handle auto-reply for an inbound message:
///   1. Load `ReplyConfig` + canned safety via `get_channel_reply_info`.
///   2. Skip if disabled.
///   3. Gate on content safety (`profile_safety` for AI, `canned_safety` for Canned).
///   4. Run prompt-injection scan via `@cf/meta/prompt-guard-2-86m`.
///   5. Check conversation handoff state (`Silent` exits without spending a credit).
///   6. Deduct 1 credit (`billing::try_deduct`) for BOTH Canned and AI replies.
///   7. Generate AI reply (or use Canned text), run handoff + risk gate, send reply.
///      Restore credit on AI failure, empty reply, or channel send failure.
async fn handle_auto_reply(
    msg: &InboundMessage,
    kv: &kv::KvStore,
    db: &D1Database,
    env: &Env,
) -> Result<()> {
    let info = match get_channel_reply_info(kv, &msg.channel, &msg.channel_account_id).await? {
        Some(i) if i.config.enabled => i,
        _ => return Ok(()),
    };
    let config = info.config;
    let is_ai = matches!(config.response, ReplyResponse::Ai);

    let onboarding = if is_ai {
        let o = get_onboarding(kv, &msg.tenant_id).await?;
        if !o.profile_safety.is_approved_for(&o.business.name) {
            console_log!(
                "Profile not safety-approved for tenant {}, skipping AI reply",
                msg.tenant_id
            );
            return Ok(());
        }
        Some(o)
    } else {
        if let ReplyResponse::Canned { ref text } = config.response {
            if !info.canned_safety.is_approved_for(text) {
                console_log!(
                    "Canned reply not safety-approved for account {}, skipping reply",
                    msg.channel_account_id
                );
                return Ok(());
            }
        }
        None
    };

    let safe_body: String = msg.body.chars().take(1000).collect();

    if ai::is_prompt_injection(env, &safe_body).await {
        console_log!(
            "Prompt injection detected from {} in tenant {}, skipping reply",
            msg.sender,
            msg.tenant_id
        );
        return Ok(());
    }

    let window = onboarding
        .as_ref()
        .map(|o| o.conversation.resolve())
        .unwrap_or_else(|| ConversationConfig::default().resolve());

    enum HandoffMode {
        None,
        HoldingPattern,
        Silent,
    }

    let conv_key = msg.conversation_key();
    let existing_session = if is_ai {
        get_conversation_session(
            kv,
            &msg.tenant_id,
            &msg.channel,
            &msg.channel_account_id,
            &conv_key,
        )
        .await
        .unwrap_or(None)
    } else {
        None
    };

    let conversation_ended = match existing_session.as_ref() {
        Some(s) => match age_minutes(&s.last_inbound_at) {
            Some(mins) => mins >= window.idle_gap_mins,
            None => true,
        },
        None => false,
    };
    let active_handoff = if conversation_ended {
        None
    } else {
        existing_session.as_ref().and_then(|s| s.handoff.clone())
    };

    let (conversation_id, prior_messages) = if conversation_ended {
        (generate_id(), Vec::new())
    } else {
        match existing_session.as_ref() {
            Some(s) if !s.conversation_id.is_empty() => {
                (s.conversation_id.clone(), s.messages.clone())
            }
            _ => (generate_id(), Vec::new()),
        }
    };

    let handoff_mode = match active_handoff.as_ref() {
        Some(h) => match age_minutes(&h.signaled_at) {
            Some(mins) if mins < window.handoff_cooldown_mins => HandoffMode::HoldingPattern,
            Some(_) => HandoffMode::Silent,
            None => HandoffMode::None,
        },
        None => HandoffMode::None,
    };

    if is_ai {
        if let Err(e) = update_message_conversation_id(db, &msg.id, &conversation_id).await {
            console_log!("Failed to stamp inbound conversation_id: {:?}", e);
        }
    }

    if matches!(handoff_mode, HandoffMode::Silent) {
        console_log!(
            "Handoff cooldown expired for tenant={} sender={}, going silent",
            msg.tenant_id,
            msg.sender
        );
        let now = crate::helpers::now_iso();
        let mut messages = prior_messages;
        push_user_turn(&mut messages, &safe_body, &now, window.max_history_messages);
        let new_session = Session {
            last_inbound_at: now,
            handoff: active_handoff.clone(),
            conversation_id,
            messages,
        };
        let _ = save_conversation_session(
            kv,
            &msg.tenant_id,
            &msg.channel,
            &msg.channel_account_id,
            &conv_key,
            &new_session,
        )
        .await;
        return Ok(());
    }

    let Some(tenant) = get_tenant(db, &msg.tenant_id).await? else {
        return Ok(());
    };
    let metered = tenant.plan.is_metered();

    // Deduct 1 credit BEFORE generating AI reply or sending Canned reply.
    let outbound_conversation_id: Option<&str> = if is_ai {
        Some(conversation_id.as_str())
    } else {
        None
    };
    if !billing::try_deduct(db, &msg.tenant_id, metered).await? {
        console_log!(
            "Insufficient credits for tenant {}, skipping reply",
            msg.tenant_id
        );
        if let Err(e) = save_message(
            db,
            &generate_id(),
            &msg.channel,
            MessageDirection::Outbound,
            &msg.recipient,
            &msg.sender,
            &msg.tenant_id,
            &msg.channel_account_id,
            Some(MessageAction::NoCredits),
            outbound_conversation_id,
        )
        .await
        {
            console_log!("Failed to log no_credits message: {:?}", e);
        }
        return Ok(());
    }

    let now_iso = crate::helpers::now_iso();
    let mut session_messages = prior_messages;
    push_user_turn(
        &mut session_messages,
        &safe_body,
        &now_iso,
        window.max_history_messages,
    );

    let raw_reply = match &config.response {
        ReplyResponse::Canned { text } => text.clone(),
        ReplyResponse::Ai => {
            let brand_name = onboarding
                .as_ref()
                .map(|o| o.business.name.as_str())
                .unwrap_or("");
            let system_prompt = match handoff_mode {
                HandoffMode::Silent => unreachable!("Silent handoff returned earlier"),
                HandoffMode::HoldingPattern => {
                    crate::prompt::wrap(crate::prompt::HOLDING_PATTERN_MIDDLE)
                }
                HandoffMode::None => crate::prompt::core_system_prompt(brand_name),
            };

            let history: Vec<(String, String)> = session_messages
                .iter()
                .map(|m| (m.role.as_wire().to_string(), m.content.clone()))
                .collect();

            match ai::generate_chat_reply(env, &system_prompt, &history).await {
                Ok(r) => r,
                Err(e) => {
                    console_log!("AI auto-reply error: {:?}", e);
                    let _ = billing::restore_credit(db, &msg.tenant_id, metered).await;
                    return Ok(());
                }
            }
        }
    };

    let stripped = crate::prompt::detect_and_strip_handoff(&raw_reply);
    let mut reply = stripped.reply;
    let mut new_handoff = stripped.handoff && matches!(handoff_mode, HandoffMode::None);

    if reply.is_empty() {
        let _ = billing::restore_credit(db, &msg.tenant_id, metered).await;
        return Ok(());
    }

    let in_handoff_mode = matches!(handoff_mode, HandoffMode::HoldingPattern);
    let mut risk_reason = None;
    if is_ai && !in_handoff_mode && !new_handoff {
        if let Some(reason) = crate::risk::signal(&reply) {
            console_log!(
                "Risk gate withheld a draft for tenant={} sender={}: {}",
                msg.tenant_id,
                msg.sender,
                reason.as_str()
            );
            reply = crate::prompt::RISK_GATE_HOLDING_REPLY.to_string();
            new_handoff = true;
            risk_reason = Some(reason);
            if let Err(e) = save_message(
                db,
                &generate_id(),
                &msg.channel,
                MessageDirection::Outbound,
                &msg.recipient,
                &msg.sender,
                &msg.tenant_id,
                &msg.channel_account_id,
                Some(MessageAction::HandedOff),
                Some(&conversation_id),
            )
            .await
            {
                console_log!("Failed to log withheld draft: {:?}", e);
            }
        }
    }

    if let Err(e) = channel::send_reply(env, msg, &reply).await {
        console_log!("Auto-reply send error: {:?}", e);
        let _ = billing::restore_credit(db, &msg.tenant_id, metered).await;
        return Ok(());
    }

    if let Err(e) = save_message(
        db,
        &generate_id(),
        &msg.channel,
        MessageDirection::Outbound,
        &msg.recipient,
        &msg.sender,
        &msg.tenant_id,
        &msg.channel_account_id,
        Some(MessageAction::AutoReply),
        outbound_conversation_id,
    )
    .await
    {
        console_log!("Failed to log outbound message: {:?}", e);
    }

    if !is_ai {
        return Ok(());
    }

    let mut session_handoff = if new_handoff {
        Some(HandoffState {
            signaled_at: now_iso.clone(),
            notified: false,
        })
    } else {
        active_handoff
    };

    if new_handoff {
        if let Err(e) = crate::escalations::notify_human_requested(
            env,
            db,
            &msg.tenant_id,
            &msg.channel,
            &msg.sender,
            &safe_body,
            risk_reason,
        )
        .await
        {
            console_log!("Handoff notify failed: {:?}", e);
        }
        if let Some(ref mut h) = session_handoff {
            h.notified = true;
        }
    }

    push_assistant_turn(
        &mut session_messages,
        &reply,
        &now_iso,
        window.max_history_messages,
    );

    let new_session = Session {
        last_inbound_at: now_iso,
        handoff: session_handoff,
        conversation_id,
        messages: session_messages,
    };
    if let Err(e) = save_conversation_session(
        kv,
        &msg.tenant_id,
        &msg.channel,
        &msg.channel_account_id,
        &conv_key,
        &new_session,
    )
    .await
    {
        console_log!("Failed to persist conversation session: {:?}", e);
    }

    Ok(())
}

fn push_user_turn(messages: &mut Vec<ConversationMessage>, body: &str, at: &str, max: u32) {
    messages.push(ConversationMessage {
        role: ConversationRole::User,
        content: body.to_string(),
        at: at.to_string(),
    });
    trim_to_max(messages, max);
}

fn push_assistant_turn(messages: &mut Vec<ConversationMessage>, body: &str, at: &str, max: u32) {
    messages.push(ConversationMessage {
        role: ConversationRole::Assistant,
        content: body.to_string(),
        at: at.to_string(),
    });
    trim_to_max(messages, max);
}

fn trim_to_max(messages: &mut Vec<ConversationMessage>, max: u32) {
    let max = max as usize;
    if max == 0 {
        messages.clear();
        return;
    }
    if messages.len() > max {
        let excess = messages.len() - max;
        messages.drain(0..excess);
    }
}

fn age_minutes(timestamp: &str) -> Option<i64> {
    let then_ms = js_sys::Date::parse(timestamp);
    if then_ms.is_nan() {
        return None;
    }
    let now_ms = js_sys::Date::now();
    let delta_min = ((now_ms - then_ms) / 60_000.0) as i64;
    Some(delta_min)
}

#[cfg(test)]
mod history_tests {
    use super::*;

    fn msg(role: ConversationRole, content: &str) -> ConversationMessage {
        ConversationMessage {
            role,
            content: content.to_string(),
            at: "2026-05-04T12:00:00.000Z".to_string(),
        }
    }

    #[test]
    fn push_user_turn_appends_and_keeps_role() {
        let mut messages = vec![msg(ConversationRole::Assistant, "hi")];
        push_user_turn(&mut messages, "hello again", "2026-05-04T12:01:00.000Z", 10);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].role, ConversationRole::User);
        assert_eq!(messages[1].content, "hello again");
    }

    #[test]
    fn push_assistant_turn_appends_and_keeps_role() {
        let mut messages = vec![msg(ConversationRole::User, "ping")];
        push_assistant_turn(&mut messages, "pong", "2026-05-04T12:01:00.000Z", 10);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].role, ConversationRole::Assistant);
    }

    #[test]
    fn trim_drops_oldest_when_over_cap() {
        let mut messages = vec![
            msg(ConversationRole::User, "u1"),
            msg(ConversationRole::Assistant, "a1"),
            msg(ConversationRole::User, "u2"),
            msg(ConversationRole::Assistant, "a2"),
            msg(ConversationRole::User, "u3"),
        ];
        trim_to_max(&mut messages, 3);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "u2");
        assert_eq!(messages[2].content, "u3");
    }

    #[test]
    fn trim_with_zero_cap_clears() {
        let mut messages = vec![msg(ConversationRole::User, "u1")];
        trim_to_max(&mut messages, 0);
        assert!(messages.is_empty());
    }
}
