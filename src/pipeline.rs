//! Unified message processing pipeline.
//! All inbound messages from any channel flow through here.

use wasm_bindgen::JsValue;
use worker::*;

use crate::ai;
use crate::billing;
use crate::channel;
use crate::helpers::{generate_id, now_epoch_ms};
use crate::storage::*;
use crate::types::*;

/// Process an inbound message, optionally batching via D1 `pending_inbound`.
pub async fn process_inbound(msg: &InboundMessage, env: &Env) -> Result<()> {
    let db = env.d1("DB")?;

    if let Err(e) = save_inbound_message(&db, msg, None).await {
        console_log!("Failed to log inbound message: {:?}", e);
    }

    let wait = get_channel_reply_config(&db, &msg.channel, &msg.channel_account_id)
        .await?
        .map(|c| c.wait_seconds)
        .unwrap_or(0);

    if wait == 0 {
        process_inbound_immediate(msg, env).await?;
    } else if let Err(e) = buffer_and_flush(&db, env, msg, wait).await {
        console_log!("burst buffer failed, falling back to immediate: {:?}", e);
        process_inbound_immediate(msg, env).await?;
    }

    Ok(())
}

/// Process a single (possibly already-batched) message immediately.
pub async fn process_inbound_immediate(msg: &InboundMessage, env: &Env) -> Result<()> {
    let db = env.d1("DB")?;
    handle_auto_reply(msg, &db, env).await
}

async fn sleep_seconds(wait_seconds: u32) {
    let ms = (wait_seconds.max(1) as i32) * 1000;
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let global = js_sys::global();
        if let Ok(set_timeout_val) = js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout"))
        {
            let set_timeout: js_sys::Function = set_timeout_val.into();
            let _ = set_timeout.call2(&global, &resolve, &JsValue::from_f64(ms as f64));
        } else {
            let _ = resolve.call0(&JsValue::NULL);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

async fn buffer_and_flush(
    db: &D1Database,
    env: &Env,
    msg: &InboundMessage,
    wait_seconds: u32,
) -> Result<()> {
    let row_id = generate_id();
    let batch_key = format!(
        "{}:{}:{}",
        msg.tenant_id,
        msg.channel.as_str(),
        msg.conversation_key()
    );
    let message_json = serde_json::to_string(msg)?;
    let created_at_ms = now_epoch_ms();

    db.prepare(
        "INSERT INTO pending_inbound (id, batch_key, message_json, created_at_ms)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&[
        row_id.as_str().into(),
        batch_key.as_str().into(),
        message_json.as_str().into(),
        JsValue::from(created_at_ms as f64),
    ])?
    .run()
    .await?;

    sleep_seconds(wait_seconds).await;

    let latest = db
        .prepare(
            "SELECT id FROM pending_inbound
             WHERE batch_key = ?
             ORDER BY created_at_ms DESC, id DESC
             LIMIT 1",
        )
        .bind(&[batch_key.as_str().into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let latest_id = latest
        .as_ref()
        .and_then(|r| r.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if latest_id != row_id {
        return Ok(());
    }

    let rows = db
        .prepare(
            "SELECT id, message_json FROM pending_inbound
             WHERE batch_key = ?
             ORDER BY created_at_ms ASC, id ASC",
        )
        .bind(&[batch_key.as_str().into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    if rows.is_empty() {
        return Ok(());
    }

    let mut pending: Vec<InboundMessage> = Vec::with_capacity(rows.len());
    for row in &rows {
        if let Some(id) = row.get("id").and_then(|v| v.as_str()) {
            let _ = db
                .prepare("DELETE FROM pending_inbound WHERE id = ?")
                .bind(&[id.into()])?
                .run()
                .await;
        }
        if let Some(json_str) = row.get("message_json").and_then(|v| v.as_str()) {
            if let Ok(m) = serde_json::from_str::<InboundMessage>(json_str) {
                pending.push(m);
            }
        }
    }

    let Some(synth) = merge_inbound_batch(&pending) else {
        return Ok(());
    };

    process_inbound_immediate(&synth, env).await
}

pub(crate) fn merge_inbound_batch(pending: &[InboundMessage]) -> Option<InboundMessage> {
    let first = pending.first()?;
    let last = pending.last()?;
    let combined_body = pending
        .iter()
        .map(|m| m.body.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let reply_to = first.conversation_key();

    Some(InboundMessage {
        id: last.id.clone(),
        channel: first.channel,
        sender: first.sender.clone(),
        sender_name: first.sender_name.clone(),
        recipient: first.recipient.clone(),
        reply_to,
        body: combined_body,
        has_attachment: false,
        tenant_id: first.tenant_id.clone(),
        channel_account_id: first.channel_account_id.clone(),
        raw_metadata: last.raw_metadata.clone(),
    })
}

/// Handle auto-reply for an inbound message:
///   1. Load `ReplyConfig` + canned safety via `get_channel_reply_info`.
///   2. Skip if disabled.
///   3. Run prompt-injection scan via `@cf/meta/llama-3.1-8b-instruct-fast`.
///   4. Decide static rule vs AI: if `effective_rules()` is non-empty, check
///      `ai::pick_static_rule`; non-matches fall back to AI generation.
///   5. Gate on content safety (`canned_safety` for rules/bot_prompt, `profile_safety` for AI).
///   6. Check conversation handoff state (`Silent` exits without spending a credit).
///   7. Deduct 1 credit (`billing::try_deduct`) for every reply (rule-matched or AI).
///   8. Generate AI reply (or use matched rule text), run handoff + risk gate, send reply,
///      log `canned_reply` / `auto_reply`, and update the conversation session.
///      Restore credit on AI failure, empty reply, or channel send failure.
async fn handle_auto_reply(msg: &InboundMessage, db: &D1Database, env: &Env) -> Result<()> {
    let info = match get_channel_reply_info(db, &msg.channel, &msg.channel_account_id).await? {
        Some(i) if i.config.enabled => i,
        _ => return Ok(()),
    };
    let config = info.config;
    let safe_body: String = msg.body.chars().take(1000).collect();

    match ai::check_prompt_injection(env, &safe_body).await {
        ai::InjectionVerdict::Clean => {}
        ai::InjectionVerdict::Blocked { reason } => {
            console_log!(
                "Prompt injection detected (reason={}) from {} in tenant {}, skipping reply",
                reason,
                msg.sender,
                msg.tenant_id
            );
            crate::metrics::record(env, "whatsapp", "prompt_injection_blocked", reason);
            return Ok(());
        }
        ai::InjectionVerdict::Error => {
            console_log!(
                "Prompt guard error for message from {} in tenant {}, continuing",
                msg.sender,
                msg.tenant_id
            );
            crate::metrics::record(env, "whatsapp", "prompt_guard_error", "ai_error");
        }
    }

    let effective_rules = config.effective_rules();
    let matched_rule_idx = if effective_rules.is_empty() {
        None
    } else {
        ai::pick_static_rule(env, &safe_body, &effective_rules).await
    };
    let send_canned = matched_rule_idx.is_some();
    let use_ai = !send_canned;

    let safety_text = config.safety_text();
    let effective_bot_prompt = if use_ai {
        let safety_hash = crate::helpers::sha256_hex(&safety_text);
        let prompt_hash = crate::helpers::sha256_hex(&config.bot_prompt);
        if info.canned_safety.status == SafetyStatus::Rejected
            && (info.canned_safety.content_hash.as_deref() == Some(safety_hash.as_str())
                || info.canned_safety.content_hash.as_deref() == Some(prompt_hash.as_str()))
        {
            console_log!(
                "Bot prompt rejected by safety (action_taken=unsafe_bot_prompt) for account {}, skipping reply",
                msg.channel_account_id
            );
            crate::metrics::record(env, "whatsapp", "safety_rejected", "bot_prompt");
            return Ok(());
        }
        if info.canned_safety.is_approved_for(&safety_text)
            || info.canned_safety.is_approved_for(&config.bot_prompt)
            || info.canned_safety.status == SafetyStatus::Pending
            || config.bot_prompt.trim().is_empty()
            || config.bot_prompt.trim() == crate::prompt::DEFAULT_BOT_PROMPT
        {
            config.bot_prompt.as_str()
        } else {
            crate::prompt::DEFAULT_BOT_PROMPT
        }
    } else {
        ""
    };

    let onboarding = if use_ai {
        let o = get_onboarding(db, &msg.tenant_id).await?;
        if o.profile_safety.status == SafetyStatus::Rejected
            && !o.business.name.trim().is_empty()
            && o.profile_safety.content_hash.as_deref()
                == Some(crate::helpers::sha256_hex(&o.business.name).as_str())
        {
            console_log!(
                "Profile rejected by safety for tenant {}, skipping AI reply",
                msg.tenant_id
            );
            crate::metrics::record(env, "whatsapp", "safety_rejected", "profile");
            return Ok(());
        }
        Some(o)
    } else {
        if let Some(idx) = matched_rule_idx {
            let text = &effective_rules[idx].reply_text;
            let text_hash = crate::helpers::sha256_hex(text);
            let safety_hash = crate::helpers::sha256_hex(&safety_text);
            if info.canned_safety.status == SafetyStatus::Rejected
                && (info.canned_safety.content_hash.as_deref() == Some(safety_hash.as_str())
                    || info.canned_safety.content_hash.as_deref() == Some(text_hash.as_str()))
            {
                console_log!(
                    "Canned reply rejected by safety for account {}, skipping reply",
                    msg.channel_account_id
                );
                crate::metrics::record(env, "whatsapp", "safety_rejected", "canned_reply");
                return Ok(());
            }
        }
        None
    };

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
    let existing_session = get_conversation_session(
        db,
        &msg.tenant_id,
        &msg.channel,
        &msg.channel_account_id,
        &conv_key,
    )
    .await
    .unwrap_or(None);

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
        Some(h) if h.holding_replies_sent < crate::prompt::HANDOFF_HOLDING_REPLIES => {
            HandoffMode::HoldingPattern
        }
        Some(_) => HandoffMode::Silent,
        None => HandoffMode::None,
    };

    if let Err(e) = update_message_conversation_id(db, &msg.id, &conversation_id).await {
        console_log!("Failed to stamp inbound conversation_id: {:?}", e);
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
            db,
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

    // Deduct 1 credit BEFORE generating or sending any reply (1 credit = 1 reply).
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
            Some(conversation_id.as_str()),
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

    let (mut reply, mut new_handoff, mut risk_reason) = if let Some(idx) = matched_rule_idx {
        (effective_rules[idx].reply_text.clone(), false, None)
    } else {
        let brand_name = onboarding
            .as_ref()
            .map(|o| o.business.name.trim())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| info.account_name.trim());
        let system_prompt = match handoff_mode {
            HandoffMode::Silent => unreachable!("Silent handoff returned earlier"),
            HandoffMode::HoldingPattern => {
                crate::prompt::wrap(crate::prompt::HOLDING_PATTERN_MIDDLE)
            }
            HandoffMode::None => {
                crate::prompt::compose_bot_system_prompt(brand_name, effective_bot_prompt)
            }
        };

        let history: Vec<(String, String)> = session_messages
            .iter()
            .map(|m| (m.role.as_wire().to_string(), m.content.clone()))
            .collect();

        let raw_reply = match ai::generate_chat_reply(env, &system_prompt, &history).await {
            Ok(r) => r,
            Err(e) => {
                console_log!("AI auto-reply error: {:?}", e);
                let _ = billing::restore_credit(db, &msg.tenant_id, metered).await;
                return Ok(());
            }
        };

        let stripped = crate::prompt::detect_and_strip_handoff(&raw_reply);
        if stripped.reply.is_empty() {
            let _ = billing::restore_credit(db, &msg.tenant_id, metered).await;
            return Ok(());
        }
        (
            stripped.reply,
            stripped.handoff && matches!(handoff_mode, HandoffMode::None),
            None,
        )
    };

    if reply.is_empty() {
        let _ = billing::restore_credit(db, &msg.tenant_id, metered).await;
        return Ok(());
    }

    let in_handoff_mode = matches!(handoff_mode, HandoffMode::HoldingPattern);
    if use_ai && !in_handoff_mode && !new_handoff {
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

    if let Err(e) = channel::send_reply(env, msg, &reply, config.access_token.as_deref()).await {
        console_log!("Auto-reply send error: {:?}", e);
        let _ = billing::restore_credit(db, &msg.tenant_id, metered).await;
        return Ok(());
    }

    crate::metrics::record(
        env,
        "whatsapp",
        "reply_sent",
        if send_canned { "rule" } else { "ai" },
    );

    let action = if send_canned {
        MessageAction::CannedReply
    } else {
        MessageAction::AutoReply
    };
    if let Err(e) = save_message(
        db,
        &generate_id(),
        &msg.channel,
        MessageDirection::Outbound,
        &msg.recipient,
        &msg.sender,
        &msg.tenant_id,
        &msg.channel_account_id,
        Some(action),
        Some(conversation_id.as_str()),
    )
    .await
    {
        console_log!("Failed to log outbound message: {:?}", e);
    }

    let mut session_handoff = if use_ai && new_handoff {
        Some(HandoffState {
            signaled_at: now_iso.clone(),
            notified: false,
            holding_replies_sent: 0,
        })
    } else {
        let mut h = active_handoff;
        if in_handoff_mode {
            if let Some(ref mut state) = h {
                state.holding_replies_sent = state.holding_replies_sent.saturating_add(1);
            }
        }
        h
    };

    if use_ai && new_handoff {
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
        db,
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

    #[test]
    fn merge_inbound_batch_keeps_first_sender_name_and_last_id_and_metadata() {
        let m1 = InboundMessage {
            id: "id-1".into(),
            channel: Channel::WhatsApp,
            sender: "cust-1".into(),
            sender_name: Some("Alice".into()),
            recipient: "phone-1".into(),
            reply_to: "cust-1".into(),
            body: "Hello".into(),
            has_attachment: false,
            tenant_id: "t-1".into(),
            channel_account_id: "wa-1".into(),
            raw_metadata: serde_json::json!({"seq": 1}),
        };
        let m2 = InboundMessage {
            id: "id-2".into(),
            channel: Channel::WhatsApp,
            sender: "cust-1".into(),
            sender_name: None,
            recipient: "phone-1".into(),
            reply_to: "cust-1".into(),
            body: "Are you open?".into(),
            has_attachment: false,
            tenant_id: "t-1".into(),
            channel_account_id: "wa-1".into(),
            raw_metadata: serde_json::json!({"seq": 2}),
        };
        let merged = merge_inbound_batch(&[m1, m2]).unwrap();
        assert_eq!(merged.id, "id-2");
        assert_eq!(merged.sender_name.as_deref(), Some("Alice"));
        assert_eq!(merged.body, "Hello\nAre you open?");
        assert_eq!(merged.raw_metadata, serde_json::json!({"seq": 2}));
    }
}
