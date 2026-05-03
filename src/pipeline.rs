//! Unified message processing pipeline.
//! All inbound messages from any channel flow through here.

use worker::*;

use crate::ai;
use crate::approval;
use crate::approvals;
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

    let is_ai = matches!(matched.response, ReplyResponse::Prompt { .. });

    // Load full onboarding state once for AI-mode rules. We need the
    // persona (for the prompt) AND the conversation knobs (for the
    // idle-gap / handoff-cooldown / history-cap window). Skipping the
    // load on canned-only rules saves a KV hit on the hot keyword
    // path.
    let onboarding = if is_ai {
        Some(get_onboarding(kv, &msg.tenant_id).await?)
    } else {
        None
    };
    let persona = onboarding.as_ref().map(|o| o.persona.clone());
    let window = onboarding
        .as_ref()
        .map(|o| o.conversation.resolve())
        .unwrap_or_else(|| crate::types::ConversationConfig::default().resolve());

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

    // Resolve the conversation session for this customer thread before
    // we burn an AI credit. Two boundaries shape the behavior, both
    // sourced from the tenant's effective `ConversationWindow`:
    //
    //   1. Conversation idle gap: if the customer has been silent for
    //      at least this long, the previous conversation is over — we
    //      wipe any in-progress handoff and the message history, mint
    //      a fresh conversation_id, and treat this inbound as a fresh
    //      question.
    //
    //   2. Handoff cooldown: once the AI has signaled handoff on this
    //      conversation, the next cooldown-window of replies are
    //      holding-pattern, then we go silent until the conversation
    //      ends (idle gap or the human taking over via the channel).
    //
    // Three handoff branches:
    //   - None:           reply under the persona normally.
    //   - HoldingPattern: swap to HOLDING_PATTERN_MIDDLE.
    //   - Silent:         return early — the human owns the thread.
    enum HandoffMode {
        None,
        HoldingPattern,
        Silent,
    }
    let existing_session = if is_ai {
        crate::storage::get_conversation_session(
            kv,
            &msg.tenant_id,
            &msg.channel,
            &msg.channel_account_id,
            &msg.sender,
        )
        .await
        .unwrap_or(None)
    } else {
        None
    };
    let conversation_ended = match existing_session.as_ref() {
        Some(s) => match age_minutes(&s.last_inbound_at) {
            // Idle gap exceeded: conversation is over.
            Some(mins) => mins >= window.idle_gap_mins,
            // Unparseable timestamp: treat as fresh.
            None => true,
        },
        None => false,
    };
    let active_handoff = if conversation_ended {
        None
    } else {
        existing_session.as_ref().and_then(|s| s.handoff.clone())
    };
    // Carry the conversation_id and message history if (and only if)
    // we're continuing the same conversation. Idle-gap reset wipes
    // both. A non-empty existing id with no idle-gap reset is reused;
    // anything else mints a fresh id.
    let (conversation_id, prior_messages) = if conversation_ended {
        (crate::helpers::generate_id(), Vec::new())
    } else {
        match existing_session.as_ref() {
            Some(s) if !s.conversation_id.is_empty() => {
                (s.conversation_id.clone(), s.messages.clone())
            }
            _ => (crate::helpers::generate_id(), Vec::new()),
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

    // Stamp the inbound row (logged earlier in `process_inbound` with
    // NULL conversation_id) with the resolved id. We're past the
    // is_ai gate, so this only runs when the inbound is actually
    // entering a conversation. Best-effort: if the UPDATE fails the
    // outbound rows still get stamped, and the row is reconstructable
    // by adjacency.
    if is_ai {
        if let Err(e) =
            crate::storage::update_message_conversation_id(db, &msg.id, &conversation_id).await
        {
            console_log!("Failed to stamp inbound conversation_id: {:?}", e);
        }
    }

    if matches!(handoff_mode, HandoffMode::Silent) {
        console_log!(
            "Handoff cooldown expired for tenant={} sender={}, going silent",
            msg.tenant_id,
            msg.sender
        );
        // Still update last_inbound_at so the idle gap is measured
        // from this ping — that way the customer eventually escapes
        // the silent window once they actually go quiet. Append the
        // inbound to history so the audit/context is complete; the
        // history isn't used while we're silent, but max-cap guards
        // against unbounded growth.
        let now = crate::helpers::now_iso();
        let mut messages = prior_messages;
        push_user_turn(&mut messages, &safe_body, &now, window.max_history_messages);
        let new_session = crate::types::Session {
            last_inbound_at: now,
            handoff: active_handoff.clone(),
            conversation_id: conversation_id.clone(),
            messages,
        };
        let _ = crate::storage::save_conversation_session(
            kv,
            &msg.tenant_id,
            &msg.channel,
            &msg.channel_account_id,
            &msg.sender,
            &new_session,
        )
        .await;
        return Ok(());
    }

    if is_ai && !billing::try_deduct(db, &msg.tenant_id).await? {
        console_log!("Tenant {} out of AI-reply credits, skipping", msg.tenant_id);
        return Ok(());
    }

    // Build the chat history we'll hand to the model: prior turns
    // (capped) plus the new inbound as the latest user turn. The same
    // shape — Vec<(role, content)> — is also what `generate_chat_reply`
    // expects, so we can pass it through verbatim.
    let now_iso = crate::helpers::now_iso();
    let mut session_messages = prior_messages;
    push_user_turn(
        &mut session_messages,
        &safe_body,
        &now_iso,
        window.max_history_messages,
    );

    let reply = match &matched.response {
        ReplyResponse::Canned { text } => text.clone(),
        ReplyResponse::Prompt { text: rule_prompt } => {
            let wrapped = match handoff_mode {
                // Silent was handled with an early return above; if we
                // reached here in Silent something is very wrong.
                HandoffMode::Silent => unreachable!("Silent handoff returned earlier"),
                HandoffMode::HoldingPattern => {
                    crate::prompt::wrap(crate::prompt::HOLDING_PATTERN_MIDDLE)
                }
                HandoffMode::None => {
                    let persona_prompt = persona
                        .as_ref()
                        .map(|p| p.active_prompt())
                        .unwrap_or_default();
                    let combined = if persona_prompt.is_empty() {
                        rule_prompt.clone()
                    } else {
                        format!("{persona_prompt}\n\n{rule_prompt}")
                    };
                    // Wrap the tenant's persona+rule text in the
                    // safety/alignment envelope. The envelope is
                    // non-editable and ships globally; the bookends
                    // are visible everywhere the user views a prompt
                    // so there's no surprise about what's actually
                    // sent to the model.
                    crate::prompt::wrap(&combined)
                }
            };

            let history: Vec<(String, String)> = session_messages
                .iter()
                .map(|m| (m.role.as_wire().to_string(), m.content.clone()))
                .collect();

            match ai::generate_chat_reply(env, &wrapped, &history).await {
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

    // Strip the handoff sentinel before anything else looks at the
    // reply. If we're already in the holding pattern, the model was
    // told not to re-emit; this is a defence-in-depth strip.
    let stripped = crate::prompt::detect_and_strip_handoff(&reply);
    let reply = stripped.reply;
    let new_handoff = stripped.handoff && matches!(handoff_mode, HandoffMode::None);

    if reply.is_empty() {
        if is_ai {
            if let Err(e) = billing::restore_credit(db, &msg.tenant_id).await {
                console_log!("Failed to restore credit: {:?}", e);
            }
        }
        return Ok(());
    }

    // For AI drafts, run the approval gate. The risk gate is the always-on
    // safety net for `Auto`; `Always` always queues; `NoGate` skips the
    // gate, but only when the operator's env var is on.
    //
    // Skip the gate entirely for the handoff path:
    //   - holding-pattern replies are pre-approved by construction
    //     (the model is just saying "a human is on the way"), and
    //   - the turn that *signals* a handoff also bypasses the queue —
    //     it's a polite holding sentence, and we want it on the
    //     customer's screen immediately while we page the tenant.
    let in_handoff_mode = matches!(handoff_mode, HandoffMode::HoldingPattern);
    if is_ai && !in_handoff_mode && !new_handoff {
        let allow_no_gate = approval::allow_no_gate(env);
        let persona_ref = persona.as_ref().expect("AI rule must have loaded persona");
        let decision = approval::decide(matched, &reply, persona_ref, allow_no_gate);
        if let approval::ApprovalDecision::Queue { reason } = decision {
            if let Err(e) =
                approvals::enqueue(env, msg, matched, &reply, reason, &conversation_id).await
            {
                // Enqueue failed: don't send (we'd bypass the human review
                // the rule asked for) and don't restore credit (the AI ran).
                // Log for visibility and bail.
                console_log!("Approval enqueue failed: {:?}", e);
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
                Some(MessageAction::AiQueued),
                Some(&conversation_id),
            )
            .await
            {
                console_log!("Failed to log queued message: {:?}", e);
            }
            // Persist the inbound turn we just appended, but do NOT
            // append the queued draft as an assistant turn — only
            // sent outbounds enter the conversation history. (If the
            // draft is rejected or edited later, an unsent assistant
            // turn would poison the next AI call.)
            let new_session = crate::types::Session {
                last_inbound_at: now_iso,
                handoff: active_handoff.clone(),
                conversation_id: conversation_id.clone(),
                messages: session_messages,
            };
            if let Err(e) = crate::storage::save_conversation_session(
                kv,
                &msg.tenant_id,
                &msg.channel,
                &msg.channel_account_id,
                &msg.sender,
                &new_session,
            )
            .await
            {
                console_log!("Failed to persist conversation session: {:?}", e);
            }
            return Ok(());
        }
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

    let outbound_conversation_id: Option<&str> = if is_ai {
        Some(conversation_id.as_str())
    } else {
        None
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
        Some(MessageAction::AutoReply),
        outbound_conversation_id,
    )
    .await
    {
        console_log!("Failed to log outbound message: {:?}", e);
    }

    // Persist the conversation session. Four things happen here:
    //   - `last_inbound_at` is bumped to now so the idle-gap clock
    //     restarts from this ping.
    //   - If the model just signaled handoff on this turn, stamp a
    //     fresh `HandoffState` in.
    //   - Otherwise carry over whatever handoff sub-state we were
    //     already running with (still in holding-pattern, or none).
    //   - If we actually sent an AI reply (vs canned), append the
    //     assistant turn to history so the next turn has it as
    //     context. Canned replies skip session writes entirely —
    //     they don't use the conversation machinery.
    //
    // For a brand-new handoff we then page the tenant exactly once.
    // The notify call is best-effort — failing to fan out shouldn't
    // tear down the inbound flow.
    if !is_ai {
        return Ok(());
    }

    let mut session_handoff = if new_handoff {
        Some(crate::types::HandoffState {
            signaled_at: now_iso.clone(),
            notified: false,
        })
    } else {
        active_handoff.clone()
    };

    if new_handoff {
        if let Err(e) = crate::escalations::notify_human_requested(
            env,
            db,
            &msg.tenant_id,
            &msg.channel,
            &msg.sender,
            &safe_body,
        )
        .await
        {
            console_log!("Handoff notify failed: {:?}", e);
        }
        // Flip notified=true after the dispatch attempt — even if it
        // partially failed we don't want to re-page on the next turn.
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

    let new_session = crate::types::Session {
        last_inbound_at: now_iso,
        handoff: session_handoff,
        conversation_id,
        messages: session_messages,
    };
    if let Err(e) = crate::storage::save_conversation_session(
        kv,
        &msg.tenant_id,
        &msg.channel,
        &msg.channel_account_id,
        &msg.sender,
        &new_session,
    )
    .await
    {
        console_log!("Failed to persist conversation session: {:?}", e);
    }

    Ok(())
}

/// Append a user turn to the running `messages` list and trim from the
/// front to keep the cap honored. The cap is the tenant's effective
/// `max_history_messages` — never zero in practice (resolver clamps
/// to the default), so we don't bother special-casing it. `at` is the
/// turn's RFC3339 timestamp; the caller's "now" works for both
/// inbound (just-arrived) and assistant (just-sent) turns.
fn push_user_turn(
    messages: &mut Vec<crate::types::ConversationMessage>,
    body: &str,
    at: &str,
    max: u32,
) {
    messages.push(crate::types::ConversationMessage {
        role: crate::types::ConversationRole::User,
        content: body.to_string(),
        at: at.to_string(),
    });
    trim_to_max(messages, max);
}

fn push_assistant_turn(
    messages: &mut Vec<crate::types::ConversationMessage>,
    body: &str,
    at: &str,
    max: u32,
) {
    messages.push(crate::types::ConversationMessage {
        role: crate::types::ConversationRole::Assistant,
        content: body.to_string(),
        at: at.to_string(),
    });
    trim_to_max(messages, max);
}

fn trim_to_max(messages: &mut Vec<crate::types::ConversationMessage>, max: u32) {
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

/// Minutes between an ISO/RFC3339 timestamp and now. Uses the
/// platform `Date.parse` (no `chrono` dep), which accepts both
/// `Date.toISOString()` output (what `helpers::now_iso` writes) and
/// other RFC3339-shaped strings.
///
/// Returns `None` for unparseable input so callers can fall through
/// to "treat as fresh" rather than trapping a customer in silence on
/// a bad record.
fn age_minutes(timestamp: &str) -> Option<i64> {
    let then_ms = js_sys::Date::parse(timestamp);
    if then_ms.is_nan() {
        return None;
    }
    let now_ms = js_sys::Date::now();
    let delta_min = ((now_ms - then_ms) / 60_000.0) as i64;
    Some(delta_min)
}

/// Decide whether a single rule's matcher fires on the inbound text.
/// `body_embedding` is `None` if no Prompt rules exist or embedding failed —
/// in that case Prompt matchers can never fire.
fn matches_rule(matcher: &ReplyMatcher, body: &str, body_embedding: Option<&[f32]>) -> bool {
    match matcher {
        ReplyMatcher::Default => false, // default fires only via fallback path
        ReplyMatcher::Keyword { keywords } => {
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

#[cfg(test)]
mod history_tests {
    use super::*;
    use crate::types::{ConversationMessage, ConversationRole};

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
        assert_eq!(messages[1].at, "2026-05-04T12:01:00.000Z");
    }

    #[test]
    fn push_assistant_turn_appends_and_keeps_role() {
        let mut messages = vec![msg(ConversationRole::User, "ping")];
        push_assistant_turn(&mut messages, "pong", "2026-05-04T12:01:00.000Z", 10);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].role, ConversationRole::Assistant);
        assert_eq!(messages[1].content, "pong");
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
        // Oldest two ("u1", "a1") were drained; the most recent three remain.
        assert_eq!(messages[0].content, "u2");
        assert_eq!(messages[1].content, "a2");
        assert_eq!(messages[2].content, "u3");
    }

    #[test]
    fn trim_no_op_when_under_cap() {
        let mut messages = vec![
            msg(ConversationRole::User, "u1"),
            msg(ConversationRole::Assistant, "a1"),
        ];
        trim_to_max(&mut messages, 10);
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn trim_with_zero_cap_clears() {
        // A pathological 0 max (only reachable if a tenant explicitly
        // sets max_history_messages to 0) clears history entirely
        // rather than tripping a slice underflow.
        let mut messages = vec![msg(ConversationRole::User, "u1")];
        trim_to_max(&mut messages, 0);
        assert!(messages.is_empty());
    }

    #[test]
    fn push_user_turn_caps_at_max_when_already_full() {
        let mut messages = vec![
            msg(ConversationRole::User, "u1"),
            msg(ConversationRole::Assistant, "a1"),
            msg(ConversationRole::User, "u2"),
        ];
        push_user_turn(&mut messages, "u3", "2026-05-04T12:01:00.000Z", 3);
        assert_eq!(messages.len(), 3);
        // u1 was evicted; the new u3 is at the tail.
        assert_eq!(messages[0].content, "a1");
        assert_eq!(messages[2].content, "u3");
    }
}
